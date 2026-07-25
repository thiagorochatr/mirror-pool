//! The pool account.
//!
//! Laid out explicitly at fixed offsets rather than through a serialisation
//! framework. The layout is the on-chain ABI, so it is worth being able to read
//! it in one place, and every accessor is bounds-checked — the crate forbids
//! unsafe, so there are no transmutes over account data.
//!
//! Integers are little-endian, matching Solana convention. Field elements are
//! canonical big-endian 32-byte scalars, matching the Poseidon syscall and the
//! Groth16 verifier, so no value is ever byte-swapped in flight.

use crate::error::MirrorProgramError;
use mirror_core::{Field, TREE_DEPTH, ZERO_LADDER};

/// How many past roots stay acceptable.
///
/// A prover builds a proof against whatever root was current when they read the
/// chain; by the time a relay lands the transaction, later deposits may have
/// advanced it. The ring is how long a proof stays valid.
///
/// 128 is deliberate. A competing implementation uses 32 *and* appends two
/// leaves per spend, so a proof there ages out after about sixteen operations
/// and fails under any real load.
pub const ROOT_HISTORY: usize = 128;

/// Byte offsets. Kept together so the layout can be audited as a unit.
mod offset {
    use super::*;
    pub const VERSION: usize = 0;
    pub const BUMP: usize = 1;
    pub const VAULT_BUMP: usize = 2;
    pub const DEPTH: usize = 3;
    pub const DENOMINATION: usize = 4;
    pub const ENTRY_FEE: usize = 12;
    pub const K_FLOOR: usize = 20;
    pub const DEPOSIT_COUNT: usize = 24;
    pub const SPEND_COUNT: usize = 32;
    pub const NEXT_INDEX: usize = 40;
    pub const ROOT_POS: usize = 48;
    pub const _RESERVED: usize = 52;
    pub const FILLED: usize = 56;
    pub const ROOTS: usize = FILLED + 32 * TREE_DEPTH;
    pub const END: usize = ROOTS + 32 * ROOT_HISTORY;
}

/// Total size of the pool account.
pub const POOL_LEN: usize = offset::END;

/// The current layout version. Bumping it invalidates old accounts rather than
/// reinterpreting their bytes.
pub const POOL_VERSION: u8 = 1;

/// Smallest permitted anonymity floor. One is not a crowd.
pub const MIN_K_FLOOR: u32 = 2;

/// Largest permitted anonymity floor: the tree's capacity.
///
/// A floor above this can never be met, and since a pool is unique per
/// denomination and creation is permissionless, that would permanently deny the
/// protocol that denomination.
pub const MAX_K_FLOOR: u32 = 1 << TREE_DEPTH;

// Pin the layout. A field inserted in the middle would otherwise silently
// reinterpret every deployed account.
const _: () = assert!(offset::FILLED == 56);
const _: () = assert!(offset::ROOTS == 696);
const _: () = assert!(POOL_LEN == 4792);

/// A borrowed, bounds-checked view over the pool account's bytes.
pub struct Pool<'a> {
    data: &'a mut [u8],
}

macro_rules! read_u64 {
    ($self:ident, $off:expr) => {{
        let mut b = [0u8; 8];
        b.copy_from_slice(&$self.data[$off..$off + 8]);
        u64::from_le_bytes(b)
    }};
}

macro_rules! write_u64 {
    ($self:ident, $off:expr, $v:expr) => {
        $self.data[$off..$off + 8].copy_from_slice(&$v.to_le_bytes())
    };
}

impl<'a> Pool<'a> {
    /// Wraps account data, rejecting a wrong length or an unknown version.
    pub fn load(data: &'a mut [u8]) -> Result<Self, MirrorProgramError> {
        if data.len() != POOL_LEN {
            return Err(MirrorProgramError::InvalidPoolAccount);
        }
        if data[offset::VERSION] != POOL_VERSION {
            return Err(MirrorProgramError::InvalidPoolAccount);
        }
        if data[offset::DEPTH] as usize != TREE_DEPTH {
            return Err(MirrorProgramError::InvalidPoolAccount);
        }
        Ok(Pool { data })
    }

    /// Wraps a freshly allocated, all-zero account for initialisation.
    pub fn load_uninitialised(data: &'a mut [u8]) -> Result<Self, MirrorProgramError> {
        if data.len() != POOL_LEN {
            return Err(MirrorProgramError::InvalidPoolAccount);
        }
        if data[offset::VERSION] != 0 {
            return Err(MirrorProgramError::AlreadyInitialised);
        }
        Ok(Pool { data })
    }

    pub fn bump(&self) -> u8 {
        self.data[offset::BUMP]
    }
    pub fn vault_bump(&self) -> u8 {
        self.data[offset::VAULT_BUMP]
    }
    pub fn denomination(&self) -> u64 {
        read_u64!(self, offset::DENOMINATION)
    }
    pub fn entry_fee(&self) -> u64 {
        read_u64!(self, offset::ENTRY_FEE)
    }
    pub fn k_floor(&self) -> u32 {
        let mut b = [0u8; 4];
        b.copy_from_slice(&self.data[offset::K_FLOOR..offset::K_FLOOR + 4]);
        u32::from_le_bytes(b)
    }
    pub fn deposit_count(&self) -> u64 {
        read_u64!(self, offset::DEPOSIT_COUNT)
    }
    pub fn spend_count(&self) -> u64 {
        read_u64!(self, offset::SPEND_COUNT)
    }
    pub fn next_index(&self) -> u64 {
        read_u64!(self, offset::NEXT_INDEX)
    }
    fn root_pos(&self) -> u32 {
        let mut b = [0u8; 4];
        b.copy_from_slice(&self.data[offset::ROOT_POS..offset::ROOT_POS + 4]);
        u32::from_le_bytes(b)
    }

    /// Writes the immutable parameters and seeds the accumulator to an empty
    /// tree of the configured depth.
    #[allow(clippy::too_many_arguments)]
    pub fn initialise(
        &mut self,
        bump: u8,
        vault_bump: u8,
        denomination: u64,
        entry_fee: u64,
        k_floor: u32,
    ) -> Result<(), MirrorProgramError> {
        if denomination == 0 {
            return Err(MirrorProgramError::InvalidParameter);
        }
        // The floor must be reachable, and it must actually be a crowd.
        //
        // Pool creation is permissionless and a pool is unique per denomination
        // forever, so an unbounded floor is a griefing vector rather than a
        // configuration mistake: a floor above the tree's capacity makes every
        // spend fail permanently, and because no second pool for that
        // denomination can ever exist, every later depositor loses their deposit
        // with no recovery. Costing a fraction of a SOL, that would deny the
        // protocol one denomination at a time.
        //
        // MIN_K_FLOOR is 2 because a floor of one is the anonymity set of one
        // this check exists to prevent — it advertises that a privacy tool was
        // used while providing no cover.
        if !(MIN_K_FLOOR..=MAX_K_FLOOR).contains(&k_floor) {
            return Err(MirrorProgramError::InvalidParameter);
        }
        // An entry fee at or above the denomination costs more to join than the
        // note is worth.
        if entry_fee >= denomination {
            return Err(MirrorProgramError::InvalidParameter);
        }

        self.data[offset::VERSION] = POOL_VERSION;
        self.data[offset::BUMP] = bump;
        self.data[offset::VAULT_BUMP] = vault_bump;
        self.data[offset::DEPTH] = TREE_DEPTH as u8;
        write_u64!(self, offset::DENOMINATION, denomination);
        write_u64!(self, offset::ENTRY_FEE, entry_fee);
        self.data[offset::K_FLOOR..offset::K_FLOOR + 4].copy_from_slice(&k_floor.to_le_bytes());

        // An empty tree's frontier is the zero ladder, and its root is the top
        // of that ladder. Seeding from the shared constant is what keeps the
        // program and the host prover on one accumulator.
        for (level, zero) in ZERO_LADDER.iter().take(TREE_DEPTH).enumerate() {
            let at = offset::FILLED + level * 32;
            self.data[at..at + 32].copy_from_slice(zero);
        }
        self.push_root(Field::from_bytes(ZERO_LADDER[TREE_DEPTH])?);
        Ok(())
    }

    pub fn filled(&self, level: usize) -> Result<Field, MirrorProgramError> {
        if level >= TREE_DEPTH {
            return Err(MirrorProgramError::MalformedInstruction);
        }
        let at = offset::FILLED + level * 32;
        let mut b = [0u8; 32];
        b.copy_from_slice(&self.data[at..at + 32]);
        Ok(Field::from_bytes(b)?)
    }

    pub fn set_filled(&mut self, level: usize, value: Field) -> Result<(), MirrorProgramError> {
        if level >= TREE_DEPTH {
            return Err(MirrorProgramError::MalformedInstruction);
        }
        let at = offset::FILLED + level * 32;
        self.data[at..at + 32].copy_from_slice(value.as_bytes());
        Ok(())
    }

    /// Appends a root to the ring and makes it current.
    pub fn push_root(&mut self, root: Field) {
        let pos = (self.root_pos() as usize + 1) % ROOT_HISTORY;
        let at = offset::ROOTS + pos * 32;
        self.data[at..at + 32].copy_from_slice(root.as_bytes());
        self.data[offset::ROOT_POS..offset::ROOT_POS + 4]
            .copy_from_slice(&(pos as u32).to_le_bytes());
    }

    pub fn current_root(&self) -> Result<Field, MirrorProgramError> {
        let at = offset::ROOTS + self.root_pos() as usize * 32;
        let mut b = [0u8; 32];
        b.copy_from_slice(&self.data[at..at + 32]);
        Ok(Field::from_bytes(b)?)
    }

    /// Whether `root` appears anywhere in the retained history.
    ///
    /// The all-zero slots of a young ring are skipped, so a proof against a
    /// literal zero root can never be accepted by accident.
    pub fn knows_root(&self, root: Field) -> bool {
        if root.is_zero() {
            return false;
        }
        (0..ROOT_HISTORY).any(|i| {
            let at = offset::ROOTS + i * 32;
            self.data[at..at + 32] == root.to_bytes()
        })
    }

    pub fn record_deposit(&mut self, new_index: u64) -> Result<(), MirrorProgramError> {
        let count = self
            .deposit_count()
            .checked_add(1)
            .ok_or(MirrorProgramError::ArithmeticOverflow)?;
        write_u64!(self, offset::DEPOSIT_COUNT, count);
        write_u64!(self, offset::NEXT_INDEX, new_index);
        Ok(())
    }

    pub fn record_spend(&mut self) -> Result<(), MirrorProgramError> {
        let count = self
            .spend_count()
            .checked_add(1)
            .ok_or(MirrorProgramError::ArithmeticOverflow)?;
        // A spend without a matching deposit would mean the nullifier set and
        // the accumulator disagree, so refuse rather than record it.
        if count > self.deposit_count() {
            return Err(MirrorProgramError::InsolventVault);
        }
        write_u64!(self, offset::SPEND_COUNT, count);
        Ok(())
    }

    /// Notes that have been deposited and not yet spent.
    pub fn outstanding_notes(&self) -> Result<u64, MirrorProgramError> {
        self.deposit_count()
            .checked_sub(self.spend_count())
            .ok_or(MirrorProgramError::ArithmeticOverflow)
    }

    /// Lamports the vault must still hold to cover every unspent note.
    ///
    /// The accounting invariant of the whole protocol. Because the denomination
    /// is a pool constant rather than a hidden field, the amount owed is a
    /// function of two counters and cannot be influenced by anything a prover
    /// supplies. The two competing implementations are drainable precisely
    /// because they lack this: one escrows an amount never bound to its
    /// commitment, the other pays out once per epoch forever against one deposit.
    pub fn required_vault_lamports(&self) -> Result<u64, MirrorProgramError> {
        self.outstanding_notes()?
            .checked_mul(self.denomination())
            .ok_or(MirrorProgramError::ArithmeticOverflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank() -> Vec<u8> {
        vec![0u8; POOL_LEN]
    }

    fn initialised(denomination: u64) -> Vec<u8> {
        let mut data = blank();
        {
            let mut pool = Pool::load_uninitialised(&mut data).unwrap();
            // The fee has to stay below the denomination, so scale it rather
            // than hardcoding one that only suits large pools.
            let entry_fee = denomination / 100;
            pool.initialise(254, 253, denomination, entry_fee, 4)
                .unwrap();
        }
        data
    }

    #[test]
    fn a_fresh_pool_reads_back_its_parameters() {
        let mut data = initialised(100_000);
        let pool = Pool::load(&mut data).unwrap();
        assert_eq!(pool.bump(), 254);
        assert_eq!(pool.vault_bump(), 253);
        assert_eq!(pool.denomination(), 100_000);
        assert_eq!(pool.entry_fee(), 1_000); // 1% of 100_000
        assert_eq!(pool.k_floor(), 4);
        assert_eq!(pool.deposit_count(), 0);
        assert_eq!(pool.spend_count(), 0);
        assert_eq!(pool.next_index(), 0);
    }

    #[test]
    fn a_fresh_pool_starts_at_the_empty_tree_root() {
        let mut data = initialised(1);
        let pool = Pool::load(&mut data).unwrap();
        let empty = mirror_core::Frontier::new().unwrap();
        assert_eq!(
            pool.current_root().unwrap(),
            empty.root(),
            "the program and the host must start from one accumulator"
        );
    }

    #[test]
    fn the_frontier_is_seeded_from_the_zero_ladder() {
        let mut data = initialised(1);
        let pool = Pool::load(&mut data).unwrap();
        let ladder = mirror_core::baked_ladder().unwrap();
        for (level, expected) in ladder.iter().take(TREE_DEPTH).enumerate() {
            assert_eq!(pool.filled(level).unwrap(), *expected);
        }
    }

    #[test]
    fn a_wrong_length_account_is_refused() {
        let mut short = vec![0u8; POOL_LEN - 1];
        assert!(matches!(
            Pool::load(&mut short),
            Err(MirrorProgramError::InvalidPoolAccount)
        ));
        let mut long = vec![0u8; POOL_LEN + 1];
        assert!(matches!(
            Pool::load(&mut long),
            Err(MirrorProgramError::InvalidPoolAccount)
        ));
    }

    #[test]
    fn an_uninitialised_account_is_not_loadable_as_a_pool() {
        let mut data = blank();
        assert!(matches!(
            Pool::load(&mut data),
            Err(MirrorProgramError::InvalidPoolAccount)
        ));
    }

    #[test]
    fn initialising_twice_is_refused() {
        let mut data = initialised(1);
        assert!(matches!(
            Pool::load_uninitialised(&mut data),
            Err(MirrorProgramError::AlreadyInitialised)
        ));
    }

    #[test]
    fn a_zero_denomination_or_zero_floor_is_refused() {
        let mut data = blank();
        let mut pool = Pool::load_uninitialised(&mut data).unwrap();
        assert!(matches!(
            pool.initialise(1, 1, 0, 0, 4),
            Err(MirrorProgramError::InvalidParameter)
        ));
        assert!(matches!(
            pool.initialise(1, 1, 100, 0, 0),
            Err(MirrorProgramError::InvalidParameter)
        ));
    }

    /// A floor nobody can reach is a permanent denial of that denomination,
    /// because pools are unique per denomination and creation is permissionless.
    /// A griefer paying one pool's rent would otherwise lock out every honest
    /// depositor for that size, forever, and take their deposits with it.
    #[test]
    fn an_unreachable_anonymity_floor_is_refused() {
        let mut data = blank();
        let mut pool = Pool::load_uninitialised(&mut data).unwrap();
        assert!(matches!(
            pool.initialise(1, 1, 1_000_000, 0, u32::MAX),
            Err(MirrorProgramError::InvalidParameter)
        ));
        assert!(matches!(
            pool.initialise(1, 1, 1_000_000, 0, MAX_K_FLOOR + 1),
            Err(MirrorProgramError::InvalidParameter)
        ));
        assert!(pool.initialise(1, 1, 1_000_000, 0, MAX_K_FLOOR).is_ok());
    }

    #[test]
    fn a_floor_of_one_is_refused_because_one_is_not_a_crowd() {
        let mut data = blank();
        let mut pool = Pool::load_uninitialised(&mut data).unwrap();
        assert!(matches!(
            pool.initialise(1, 1, 1_000_000, 0, 1),
            Err(MirrorProgramError::InvalidParameter)
        ));
        assert!(pool.initialise(1, 1, 1_000_000, 0, MIN_K_FLOOR).is_ok());
    }

    #[test]
    fn an_entry_fee_worth_more_than_the_note_is_refused() {
        let mut data = blank();
        let mut pool = Pool::load_uninitialised(&mut data).unwrap();
        assert!(matches!(
            pool.initialise(1, 1, 1_000, 1_000, 4),
            Err(MirrorProgramError::InvalidParameter)
        ));
        assert!(pool.initialise(1, 1, 1_000, 999, 4).is_ok());
    }

    #[test]
    fn the_root_ring_remembers_and_then_forgets() {
        let mut data = initialised(1);
        let mut pool = Pool::load(&mut data).unwrap();

        let first = Field::from_u64(1_000);
        pool.push_root(first);
        assert!(pool.knows_root(first));
        assert_eq!(pool.current_root().unwrap(), first);

        // Fill the ring exactly once more; the first root should still be the
        // oldest survivor.
        for i in 1..ROOT_HISTORY {
            pool.push_root(Field::from_u64(1_000 + i as u64));
        }
        assert!(pool.knows_root(first), "evicted one push too early");

        pool.push_root(Field::from_u64(9_999));
        assert!(!pool.knows_root(first), "should have aged out by now");
    }

    #[test]
    fn an_unknown_root_is_not_accepted() {
        let mut data = initialised(1);
        let pool = Pool::load(&mut data).unwrap();
        assert!(!pool.knows_root(Field::from_u64(12345)));
    }

    #[test]
    fn a_zero_root_is_never_accepted() {
        // A young ring is mostly zeroes. Treating zero as a known root would let
        // a proof against an all-zero root pass while the ring is still filling.
        let mut data = initialised(1);
        let pool = Pool::load(&mut data).unwrap();
        assert!(!pool.knows_root(Field::ZERO));
    }

    #[test]
    fn the_vault_requirement_tracks_outstanding_notes() {
        let mut data = initialised(1_000_000);
        let mut pool = Pool::load(&mut data).unwrap();
        assert_eq!(pool.required_vault_lamports().unwrap(), 0);

        for i in 1..=5u64 {
            pool.record_deposit(i).unwrap();
        }
        assert_eq!(pool.outstanding_notes().unwrap(), 5);
        assert_eq!(pool.required_vault_lamports().unwrap(), 5_000_000);

        pool.record_spend().unwrap();
        pool.record_spend().unwrap();
        assert_eq!(pool.outstanding_notes().unwrap(), 3);
        assert_eq!(pool.required_vault_lamports().unwrap(), 3_000_000);
    }

    /// The shape of the drain that breaks both competing implementations: more
    /// payouts than deposits. Here it is not a matter of catching it late in the
    /// spend path — the counter itself refuses.
    #[test]
    fn spending_more_notes_than_were_deposited_is_refused() {
        let mut data = initialised(1_000);
        let mut pool = Pool::load(&mut data).unwrap();
        pool.record_deposit(1).unwrap();
        pool.record_spend().unwrap();
        assert!(matches!(
            pool.record_spend(),
            Err(MirrorProgramError::InsolventVault)
        ));
        assert_eq!(
            pool.spend_count(),
            1,
            "the refused spend must not be recorded"
        );
    }

    #[test]
    fn a_level_outside_the_tree_is_refused_rather_than_wrapping() {
        let mut data = initialised(1);
        let mut pool = Pool::load(&mut data).unwrap();
        assert!(pool.filled(TREE_DEPTH).is_err());
        assert!(pool.set_filled(TREE_DEPTH, Field::ZERO).is_err());
        assert!(pool.filled(TREE_DEPTH - 1).is_ok());
    }
}
