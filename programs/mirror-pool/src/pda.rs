//! Program-derived addresses.
//!
//! One pool per denomination, globally. That is a privacy decision rather than a
//! convenience one: fragmenting deposits of the same size across several pools
//! splits the anonymity set, and a split set is strictly worse for every member
//! in it. Making the denomination the only seed means two users choosing "0.1
//! SOL" cannot accidentally end up in different crowds.

use solana_program::pubkey::Pubkey;

pub const POOL_SEED: &[u8] = b"pool";
pub const VAULT_SEED: &[u8] = b"vault";

/// The pool account for a denomination.
pub fn pool_address(program_id: &Pubkey, denomination: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[POOL_SEED, &denomination.to_le_bytes()], program_id)
}

/// The vault that escrows a pool's notes.
///
/// Kept separate from the pool account so that the accounting invariant reads
/// against a balance holding nothing but escrow and its own rent. Entry fees
/// accrue on the pool account instead, so reward lamports can never be mistaken
/// for lamports backing an unspent note.
pub fn vault_address(program_id: &Pubkey, pool: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[VAULT_SEED, pool.as_ref()], program_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_are_deterministic() {
        let program = Pubkey::new_unique();
        let (a, bump_a) = pool_address(&program, 100_000);
        let (b, bump_b) = pool_address(&program, 100_000);
        assert_eq!(a, b);
        assert_eq!(bump_a, bump_b);
    }

    #[test]
    fn each_denomination_gets_its_own_pool() {
        let program = Pubkey::new_unique();
        assert_ne!(
            pool_address(&program, 100_000).0,
            pool_address(&program, 200_000).0
        );
    }

    #[test]
    fn the_vault_is_derived_from_the_pool_not_the_denomination() {
        let program = Pubkey::new_unique();
        let (pool, _) = pool_address(&program, 100_000);
        let (vault, _) = vault_address(&program, &pool);
        assert_ne!(vault, pool);
        assert_eq!(vault, vault_address(&program, &pool).0);
    }

    #[test]
    fn a_different_program_yields_different_addresses() {
        let (a, _) = pool_address(&Pubkey::new_unique(), 1);
        let (b, _) = pool_address(&Pubkey::new_unique(), 1);
        assert_ne!(a, b);
    }
}
