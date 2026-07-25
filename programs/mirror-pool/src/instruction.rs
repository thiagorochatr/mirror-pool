//! Instruction encoding.
//!
//! Every variant parses at an exact length. A trailing byte, a missing byte, or
//! an unknown tag is rejected rather than tolerated — there is no "read what you
//! need and ignore the rest", because that is how a caller ends up believing it
//! sent a field the program never read.
//!
//! Encoding is hand-rolled rather than borsh: the instruction data is part of
//! the on-chain ABI, the shapes are small and fixed, and a hand-rolled decoder
//! is one place to audit for length handling.

use crate::error::MirrorProgramError;

/// Instruction tags. Stable across versions; new instructions take new tags
/// rather than reusing a retired one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Tag {
    InitPool = 0,
    Deposit = 1,
}

impl Tag {
    fn from_u8(v: u8) -> Result<Self, MirrorProgramError> {
        match v {
            0 => Ok(Tag::InitPool),
            1 => Ok(Tag::Deposit),
            _ => Err(MirrorProgramError::MalformedInstruction),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    /// Creates the pool and its vault, and seeds the accumulator to an empty
    /// tree.
    InitPool {
        denomination: u64,
        entry_fee: u64,
        k_floor: u32,
    },
    /// Escrows exactly `denomination + entry_fee` and appends `commitment` to
    /// the accumulator.
    ///
    /// The amount is not a parameter. It is read from the pool, so a deposit
    /// cannot claim a size the pool did not set.
    Deposit { commitment: [u8; 32] },
}

/// `InitPool`: tag + u64 + u64 + u32.
pub const INIT_POOL_LEN: usize = 1 + 8 + 8 + 4;
/// `Deposit`: tag + one field element.
pub const DEPOSIT_LEN: usize = 1 + 32;

fn read_u64(data: &[u8], at: usize) -> u64 {
    let mut b = [0u8; 8];
    b.copy_from_slice(&data[at..at + 8]);
    u64::from_le_bytes(b)
}

fn read_u32(data: &[u8], at: usize) -> u32 {
    let mut b = [0u8; 4];
    b.copy_from_slice(&data[at..at + 4]);
    u32::from_le_bytes(b)
}

impl Instruction {
    pub fn unpack(data: &[u8]) -> Result<Self, MirrorProgramError> {
        let tag = Tag::from_u8(
            *data
                .first()
                .ok_or(MirrorProgramError::MalformedInstruction)?,
        )?;

        match tag {
            Tag::InitPool => {
                if data.len() != INIT_POOL_LEN {
                    return Err(MirrorProgramError::MalformedInstruction);
                }
                Ok(Instruction::InitPool {
                    denomination: read_u64(data, 1),
                    entry_fee: read_u64(data, 9),
                    k_floor: read_u32(data, 17),
                })
            }
            Tag::Deposit => {
                if data.len() != DEPOSIT_LEN {
                    return Err(MirrorProgramError::MalformedInstruction);
                }
                let mut commitment = [0u8; 32];
                commitment.copy_from_slice(&data[1..33]);
                Ok(Instruction::Deposit { commitment })
            }
        }
    }

    /// Serialises for a client. Kept beside the decoder so the two cannot drift.
    pub fn pack(&self) -> Vec<u8> {
        match self {
            Instruction::InitPool {
                denomination,
                entry_fee,
                k_floor,
            } => {
                let mut out = Vec::with_capacity(INIT_POOL_LEN);
                out.push(Tag::InitPool as u8);
                out.extend_from_slice(&denomination.to_le_bytes());
                out.extend_from_slice(&entry_fee.to_le_bytes());
                out.extend_from_slice(&k_floor.to_le_bytes());
                out
            }
            Instruction::Deposit { commitment } => {
                let mut out = Vec::with_capacity(DEPOSIT_LEN);
                out.push(Tag::Deposit as u8);
                out.extend_from_slice(commitment);
                out
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init() -> Instruction {
        Instruction::InitPool {
            denomination: 100_000_000,
            entry_fee: 5_000,
            k_floor: 8,
        }
    }

    fn deposit() -> Instruction {
        Instruction::Deposit {
            commitment: [3u8; 32],
        }
    }

    #[test]
    fn packing_then_unpacking_is_the_identity() {
        for ix in [init(), deposit()] {
            assert_eq!(Instruction::unpack(&ix.pack()).unwrap(), ix);
        }
    }

    #[test]
    fn the_encoded_lengths_are_pinned() {
        // These are the on-chain ABI. A change here breaks every deployed client.
        assert_eq!(init().pack().len(), INIT_POOL_LEN);
        assert_eq!(deposit().pack().len(), DEPOSIT_LEN);
        assert_eq!(INIT_POOL_LEN, 21);
        assert_eq!(DEPOSIT_LEN, 33);
    }

    #[test]
    fn empty_data_is_refused() {
        assert!(matches!(
            Instruction::unpack(&[]),
            Err(MirrorProgramError::MalformedInstruction)
        ));
    }

    #[test]
    fn an_unknown_tag_is_refused() {
        for tag in [2u8, 3, 99, 255] {
            assert!(
                matches!(
                    Instruction::unpack(&[tag]),
                    Err(MirrorProgramError::MalformedInstruction)
                ),
                "tag {tag} was accepted"
            );
        }
    }

    /// Fail-closed on shape, checked rather than asserted in a README. A short
    /// buffer must not be zero-extended and a long one must not be truncated.
    #[test]
    fn every_wrong_length_is_refused() {
        for ix in [init(), deposit()] {
            let good = ix.pack();
            for len in 0..good.len() {
                assert!(
                    Instruction::unpack(&good[..len]).is_err(),
                    "a {len}-byte prefix was accepted for {ix:?}"
                );
            }
            let mut long = good.clone();
            long.push(0);
            assert!(
                Instruction::unpack(&long).is_err(),
                "a trailing byte was tolerated for {ix:?}"
            );
        }
    }

    #[test]
    fn fields_decode_at_the_right_offsets() {
        // Distinct values so a swapped offset cannot pass by coincidence.
        let ix = Instruction::InitPool {
            denomination: 0x1122_3344_5566_7788,
            entry_fee: 0x99aa_bbcc_ddee_ff00,
            k_floor: 0xdead_beef,
        };
        assert_eq!(Instruction::unpack(&ix.pack()).unwrap(), ix);
    }

    #[test]
    fn a_deposit_commitment_survives_verbatim() {
        let mut commitment = [0u8; 32];
        for (i, b) in commitment.iter_mut().enumerate() {
            *b = i as u8;
        }
        let ix = Instruction::Deposit { commitment };
        match Instruction::unpack(&ix.pack()).unwrap() {
            Instruction::Deposit { commitment: got } => assert_eq!(got, commitment),
            other => panic!("decoded as {other:?}"),
        }
    }
}
