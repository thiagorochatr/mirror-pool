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
    SubmitSpend = 2,
}

impl Tag {
    fn from_u8(v: u8) -> Result<Self, MirrorProgramError> {
        match v {
            0 => Ok(Tag::InitPool),
            1 => Ok(Tag::Deposit),
            2 => Ok(Tag::SubmitSpend),
            _ => Err(MirrorProgramError::MalformedInstruction),
        }
    }
}

/// `SubmitSpend` carries a 256-byte proof, so it is far larger than the other
/// variants. Boxing it — clippy's usual remedy — would move it to the heap, and
/// a heap allocation inside an on-chain program costs compute units to avoid a
/// stack cost we can comfortably afford: 368 bytes against a 4 KB frame, decoded
/// once per transaction. The size difference is deliberate.
#[allow(clippy::large_enum_variant)]
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
    /// Proves membership, burns the nullifier, and records the authorised
    /// action. Nothing is paid out here — settlement executes the batch.
    ///
    /// The action binding is **not** transmitted. It is recomputed on-chain from
    /// `selector`, `beneficiary` and `relay_fee` and used as the third public
    /// input, so a relay that alters any of them produces a different binding
    /// and the pairing simply fails. There is no separate field to forget to
    /// check.
    SubmitSpend {
        proof_a: [u8; 64],
        proof_b: [u8; 128],
        proof_c: [u8; 64],
        root: [u8; 32],
        nullifier: [u8; 32],
        selector: u64,
        beneficiary: [u8; 32],
        relay_fee: u64,
    },
}

/// `InitPool`: tag + u64 + u64 + u32.
pub const INIT_POOL_LEN: usize = 1 + 8 + 8 + 4;
/// `Deposit`: tag + one field element.
pub const DEPOSIT_LEN: usize = 1 + 32;
/// `SubmitSpend`: tag + proof + root + nullifier + selector + beneficiary + fee.
pub const SUBMIT_SPEND_LEN: usize = 1 + 64 + 128 + 64 + 32 + 32 + 8 + 32 + 8;

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
            Tag::SubmitSpend => {
                if data.len() != SUBMIT_SPEND_LEN {
                    return Err(MirrorProgramError::MalformedInstruction);
                }
                let mut proof_a = [0u8; 64];
                let mut proof_b = [0u8; 128];
                let mut proof_c = [0u8; 64];
                let mut root = [0u8; 32];
                let mut nullifier = [0u8; 32];
                let mut beneficiary = [0u8; 32];
                proof_a.copy_from_slice(&data[1..65]);
                proof_b.copy_from_slice(&data[65..193]);
                proof_c.copy_from_slice(&data[193..257]);
                root.copy_from_slice(&data[257..289]);
                nullifier.copy_from_slice(&data[289..321]);
                let selector = read_u64(data, 321);
                beneficiary.copy_from_slice(&data[329..361]);
                let relay_fee = read_u64(data, 361);
                Ok(Instruction::SubmitSpend {
                    proof_a,
                    proof_b,
                    proof_c,
                    root,
                    nullifier,
                    selector,
                    beneficiary,
                    relay_fee,
                })
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
            Instruction::SubmitSpend {
                proof_a,
                proof_b,
                proof_c,
                root,
                nullifier,
                selector,
                beneficiary,
                relay_fee,
            } => {
                let mut out = Vec::with_capacity(SUBMIT_SPEND_LEN);
                out.push(Tag::SubmitSpend as u8);
                out.extend_from_slice(proof_a);
                out.extend_from_slice(proof_b);
                out.extend_from_slice(proof_c);
                out.extend_from_slice(root);
                out.extend_from_slice(nullifier);
                out.extend_from_slice(&selector.to_le_bytes());
                out.extend_from_slice(beneficiary);
                out.extend_from_slice(&relay_fee.to_le_bytes());
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

    fn submit_spend() -> Instruction {
        // Distinct byte patterns per field so a swapped offset cannot pass.
        Instruction::SubmitSpend {
            proof_a: [0xA1; 64],
            proof_b: [0xB2; 128],
            proof_c: [0xC3; 64],
            root: [0xD4; 32],
            nullifier: [0xE5; 32],
            selector: 0x0102_0304_0506_0708,
            beneficiary: [0xF6; 32],
            relay_fee: 0x1112_1314_1516_1718,
        }
    }

    #[test]
    fn packing_then_unpacking_is_the_identity() {
        for ix in [init(), deposit(), submit_spend()] {
            assert_eq!(Instruction::unpack(&ix.pack()).unwrap(), ix);
        }
    }

    #[test]
    fn the_encoded_lengths_are_pinned() {
        // These are the on-chain ABI. A change here breaks every deployed client.
        assert_eq!(init().pack().len(), INIT_POOL_LEN);
        assert_eq!(deposit().pack().len(), DEPOSIT_LEN);
        assert_eq!(submit_spend().pack().len(), SUBMIT_SPEND_LEN);
        assert_eq!(INIT_POOL_LEN, 21);
        assert_eq!(DEPOSIT_LEN, 33);
        assert_eq!(SUBMIT_SPEND_LEN, 369);
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
        for tag in [3u8, 4, 99, 255] {
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
        for ix in [init(), deposit(), submit_spend()] {
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
