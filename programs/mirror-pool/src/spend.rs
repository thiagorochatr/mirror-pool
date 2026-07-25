//! The spend record.
//!
//! One account per nullifier, seeded by it. Its *existence* is the replay
//! guard — creating it fails if it already exists, so a second spend of the same
//! note cannot even reach the verifier — and its *contents* are the action the
//! proof authorised, held until settlement.
//!
//! Folding the nullifier marker and the pending action into one account is
//! deliberate: two accounts keyed by the same value can disagree, and an account
//! that exists to be checked but never read is easy to forget to check.

use crate::error::MirrorProgramError;

/// Not yet executed.
pub const STATUS_PENDING: u8 = 1;
/// Executed at settlement.
pub const STATUS_SETTLED: u8 = 2;

mod offset {
    pub const VERSION: usize = 0;
    pub const STATUS: usize = 1;
    pub const BUMP: usize = 2;
    pub const _RESERVED: usize = 3;
    pub const SELECTOR: usize = 8;
    pub const RELAY_FEE: usize = 16;
    pub const BENEFICIARY: usize = 24;
    pub const RELAY: usize = 56;
    pub const END: usize = 88;
}

pub const SPEND_LEN: usize = offset::END;
pub const SPEND_VERSION: u8 = 1;

const _: () = assert!(SPEND_LEN == 88);

pub struct Spend<'a> {
    data: &'a mut [u8],
}

impl<'a> Spend<'a> {
    pub fn load(data: &'a mut [u8]) -> Result<Self, MirrorProgramError> {
        if data.len() != SPEND_LEN || data[offset::VERSION] != SPEND_VERSION {
            return Err(MirrorProgramError::InvalidSpendAccount);
        }
        Ok(Spend { data })
    }

    pub fn load_uninitialised(data: &'a mut [u8]) -> Result<Self, MirrorProgramError> {
        if data.len() != SPEND_LEN {
            return Err(MirrorProgramError::InvalidSpendAccount);
        }
        // A non-zero version means this nullifier has already been recorded.
        // Account creation would normally have failed first; this is the second
        // line of the replay guard, not the first.
        if data[offset::VERSION] != 0 {
            return Err(MirrorProgramError::NullifierAlreadySpent);
        }
        Ok(Spend { data })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn initialise(
        &mut self,
        bump: u8,
        selector: u64,
        relay_fee: u64,
        beneficiary: &[u8; 32],
        relay: &[u8; 32],
    ) {
        self.data[offset::VERSION] = SPEND_VERSION;
        self.data[offset::STATUS] = STATUS_PENDING;
        self.data[offset::BUMP] = bump;
        self.data[offset::SELECTOR..offset::SELECTOR + 8].copy_from_slice(&selector.to_le_bytes());
        self.data[offset::RELAY_FEE..offset::RELAY_FEE + 8]
            .copy_from_slice(&relay_fee.to_le_bytes());
        self.data[offset::BENEFICIARY..offset::BENEFICIARY + 32].copy_from_slice(beneficiary);
        self.data[offset::RELAY..offset::RELAY + 32].copy_from_slice(relay);
    }

    pub fn status(&self) -> u8 {
        self.data[offset::STATUS]
    }
    pub fn bump(&self) -> u8 {
        self.data[offset::BUMP]
    }
    pub fn selector(&self) -> u64 {
        let mut b = [0u8; 8];
        b.copy_from_slice(&self.data[offset::SELECTOR..offset::SELECTOR + 8]);
        u64::from_le_bytes(b)
    }
    pub fn relay_fee(&self) -> u64 {
        let mut b = [0u8; 8];
        b.copy_from_slice(&self.data[offset::RELAY_FEE..offset::RELAY_FEE + 8]);
        u64::from_le_bytes(b)
    }
    pub fn beneficiary(&self) -> [u8; 32] {
        let mut b = [0u8; 32];
        b.copy_from_slice(&self.data[offset::BENEFICIARY..offset::BENEFICIARY + 32]);
        b
    }
    pub fn relay(&self) -> [u8; 32] {
        let mut b = [0u8; 32];
        b.copy_from_slice(&self.data[offset::RELAY..offset::RELAY + 32]);
        b
    }

    /// Marks the spend executed, refusing to do so twice.
    pub fn mark_settled(&mut self) -> Result<(), MirrorProgramError> {
        if self.status() != STATUS_PENDING {
            return Err(MirrorProgramError::AlreadySettled);
        }
        self.data[offset::STATUS] = STATUS_SETTLED;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Vec<u8> {
        vec![0u8; SPEND_LEN]
    }

    #[test]
    fn a_spend_reads_back_what_was_written() {
        let mut data = fresh();
        let beneficiary = [9u8; 32];
        let relay = [4u8; 32];
        {
            let mut s = Spend::load_uninitialised(&mut data).unwrap();
            s.initialise(251, 7, 12_345, &beneficiary, &relay);
        }
        let s = Spend::load(&mut data).unwrap();
        assert_eq!(s.status(), STATUS_PENDING);
        assert_eq!(s.bump(), 251);
        assert_eq!(s.selector(), 7);
        assert_eq!(s.relay_fee(), 12_345);
        assert_eq!(s.beneficiary(), beneficiary);
        assert_eq!(s.relay(), relay);
    }

    #[test]
    fn an_existing_record_refuses_reinitialisation() {
        let mut data = fresh();
        {
            let mut s = Spend::load_uninitialised(&mut data).unwrap();
            s.initialise(1, 1, 1, &[1u8; 32], &[2u8; 32]);
        }
        assert!(matches!(
            Spend::load_uninitialised(&mut data),
            Err(MirrorProgramError::NullifierAlreadySpent)
        ));
    }

    #[test]
    fn a_spend_settles_exactly_once() {
        let mut data = fresh();
        {
            let mut s = Spend::load_uninitialised(&mut data).unwrap();
            s.initialise(1, 1, 1, &[1u8; 32], &[2u8; 32]);
        }
        let mut s = Spend::load(&mut data).unwrap();
        s.mark_settled().unwrap();
        assert_eq!(s.status(), STATUS_SETTLED);
        assert!(matches!(
            s.mark_settled(),
            Err(MirrorProgramError::AlreadySettled)
        ));
    }

    #[test]
    fn a_wrong_length_account_is_refused() {
        let mut short = vec![0u8; SPEND_LEN - 1];
        assert!(Spend::load(&mut short).is_err());
        assert!(Spend::load_uninitialised(&mut short).is_err());
    }

    #[test]
    fn fields_do_not_overlap() {
        // Distinct values in every field; if two overlapped, one would clobber
        // the other and this would fail.
        let mut data = fresh();
        let beneficiary = [0xAB; 32];
        let relay = [0xCD; 32];
        {
            let mut s = Spend::load_uninitialised(&mut data).unwrap();
            s.initialise(
                0xEE,
                0x1122_3344_5566_7788,
                0x99AA_BBCC_DDEE_FF00,
                &beneficiary,
                &relay,
            );
        }
        let s = Spend::load(&mut data).unwrap();
        assert_eq!(s.bump(), 0xEE);
        assert_eq!(s.selector(), 0x1122_3344_5566_7788);
        assert_eq!(s.relay_fee(), 0x99AA_BBCC_DDEE_FF00);
        assert_eq!(s.beneficiary(), beneficiary);
        assert_eq!(s.relay(), relay);
    }
}
