//! The spend record.
//!
//! One account per nullifier, seeded by it. Its *existence* is the replay
//! guard — `submit_spend` refuses outright if it is already there — and its
//! contents are the action the proof authorised, held until settlement.
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
    pub const SUBMITTED_AT: usize = 24;
    pub const BENEFICIARY: usize = 32;
    pub const RELAY: usize = 64;
    pub const POOL: usize = 96;
    pub const END: usize = 128;
}

pub const SPEND_LEN: usize = offset::END;
pub const SPEND_VERSION: u8 = 1;

const _: () = assert!(SPEND_LEN == 128);

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
        submitted_at: i64,
        beneficiary: &[u8; 32],
        relay: &[u8; 32],
        pool: &[u8; 32],
    ) {
        self.data[offset::VERSION] = SPEND_VERSION;
        self.data[offset::STATUS] = STATUS_PENDING;
        self.data[offset::BUMP] = bump;
        self.data[offset::SELECTOR..offset::SELECTOR + 8].copy_from_slice(&selector.to_le_bytes());
        self.data[offset::RELAY_FEE..offset::RELAY_FEE + 8]
            .copy_from_slice(&relay_fee.to_le_bytes());
        self.data[offset::SUBMITTED_AT..offset::SUBMITTED_AT + 8]
            .copy_from_slice(&submitted_at.to_le_bytes());
        self.data[offset::BENEFICIARY..offset::BENEFICIARY + 32].copy_from_slice(beneficiary);
        self.data[offset::RELAY..offset::RELAY + 32].copy_from_slice(relay);
        self.data[offset::POOL..offset::POOL + 32].copy_from_slice(pool);
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
    /// Unix time the spend was accepted. Settlement uses it to decide whether a
    /// batch may go out below the crowd size.
    pub fn submitted_at(&self) -> i64 {
        let mut b = [0u8; 8];
        b.copy_from_slice(&self.data[offset::SUBMITTED_AT..offset::SUBMITTED_AT + 8]);
        i64::from_le_bytes(b)
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
    /// The pool this spend belongs to.
    ///
    /// Settlement checks it, so a record from one denomination cannot be
    /// presented to another pool's vault.
    pub fn pool(&self) -> [u8; 32] {
        let mut b = [0u8; 32];
        b.copy_from_slice(&self.data[offset::POOL..offset::POOL + 32]);
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

    fn init(data: &mut [u8]) {
        let mut s = Spend::load_uninitialised(data).unwrap();
        s.initialise(
            251,
            7,
            12_345,
            1_700_000_000,
            &[9u8; 32],
            &[4u8; 32],
            &[5u8; 32],
        );
    }

    #[test]
    fn a_spend_reads_back_what_was_written() {
        let mut data = fresh();
        init(&mut data);
        let s = Spend::load(&mut data).unwrap();
        assert_eq!(s.status(), STATUS_PENDING);
        assert_eq!(s.bump(), 251);
        assert_eq!(s.selector(), 7);
        assert_eq!(s.relay_fee(), 12_345);
        assert_eq!(s.submitted_at(), 1_700_000_000);
        assert_eq!(s.beneficiary(), [9u8; 32]);
        assert_eq!(s.relay(), [4u8; 32]);
        assert_eq!(s.pool(), [5u8; 32]);
    }

    #[test]
    fn an_existing_record_refuses_reinitialisation() {
        let mut data = fresh();
        init(&mut data);
        assert!(matches!(
            Spend::load_uninitialised(&mut data),
            Err(MirrorProgramError::NullifierAlreadySpent)
        ));
    }

    #[test]
    fn a_spend_settles_exactly_once() {
        let mut data = fresh();
        init(&mut data);
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
        {
            let mut s = Spend::load_uninitialised(&mut data).unwrap();
            s.initialise(
                0xEE,
                0x1122_3344_5566_7788,
                0x99AA_BBCC_DDEE_FF00,
                -0x0102_0304_0506_0708,
                &[0xAB; 32],
                &[0xCD; 32],
                &[0xEF; 32],
            );
        }
        let s = Spend::load(&mut data).unwrap();
        assert_eq!(s.bump(), 0xEE);
        assert_eq!(s.selector(), 0x1122_3344_5566_7788);
        assert_eq!(s.relay_fee(), 0x99AA_BBCC_DDEE_FF00);
        assert_eq!(s.submitted_at(), -0x0102_0304_0506_0708);
        assert_eq!(s.beneficiary(), [0xAB; 32]);
        assert_eq!(s.relay(), [0xCD; 32]);
        assert_eq!(s.pool(), [0xEF; 32]);
    }
}
