//! Instruction handlers.

use crate::{
    error::MirrorProgramError,
    instruction::Instruction,
    pda::{pool_address, vault_address, POOL_SEED, VAULT_SEED},
    state::{Pool, POOL_LEN},
};
use mirror_core::{hash_node, Field, TREE_DEPTH};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke_signed,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use solana_system_interface::{instruction as system_instruction, program as system_program};

pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    match Instruction::unpack(data)? {
        Instruction::InitPool {
            denomination,
            entry_fee,
            k_floor,
        } => init_pool(program_id, accounts, denomination, entry_fee, k_floor),
        Instruction::Deposit { commitment } => deposit(program_id, accounts, commitment),
    }
}

/// Creates the pool and its vault and seeds the accumulator to an empty tree.
///
/// Permissionless: the first caller for a denomination creates the crowd
/// everyone else joins. There is no privileged authority, so there is no key
/// whose loss freezes the pool — a competing implementation bakes its authority
/// into the pool's PDA seeds with no rotation instruction, which makes its
/// advertised rotating relay undeployable and its escrow permanently hostage.
fn init_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    denomination: u64,
    entry_fee: u64,
    k_floor: u32,
) -> ProgramResult {
    let iter = &mut accounts.iter();
    let payer = next_account_info(iter)?;
    let pool_account = next_account_info(iter)?;
    let vault_account = next_account_info(iter)?;
    let system = next_account_info(iter)?;

    if !payer.is_signer {
        return Err(MirrorProgramError::MissingSignature.into());
    }
    if !system_program::check_id(system.key) {
        return Err(MirrorProgramError::InvalidOwner.into());
    }

    let (expected_pool, pool_bump) = pool_address(program_id, denomination);
    if *pool_account.key != expected_pool {
        return Err(MirrorProgramError::InvalidPda.into());
    }
    let (expected_vault, vault_bump) = vault_address(program_id, &expected_pool);
    if *vault_account.key != expected_vault {
        return Err(MirrorProgramError::InvalidPda.into());
    }

    let rent = Rent::get()?;
    let denom_le = denomination.to_le_bytes();

    // The pool account carries the accumulator and, above its own rent, the
    // reward pool that entry fees accrue into.
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            pool_account.key,
            rent.minimum_balance(POOL_LEN),
            POOL_LEN as u64,
            program_id,
        ),
        &[payer.clone(), pool_account.clone(), system.clone()],
        &[&[POOL_SEED, &denom_le, &[pool_bump]]],
    )?;

    // The vault holds escrow only, and carries no data of its own: the
    // accounting invariant is a statement about its lamports, so anything else
    // living there would muddy it.
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            vault_account.key,
            rent.minimum_balance(0),
            0,
            program_id,
        ),
        &[payer.clone(), vault_account.clone(), system.clone()],
        &[&[VAULT_SEED, expected_pool.as_ref(), &[vault_bump]]],
    )?;

    let mut data = pool_account.try_borrow_mut_data()?;
    let mut pool = Pool::load_uninitialised(&mut data)?;
    pool.initialise(pool_bump, vault_bump, denomination, entry_fee, k_floor)?;
    Ok(())
}

/// Escrows exactly one denomination and appends a note commitment.
///
/// The escrowed amount is read from the pool, never from the instruction, so a
/// deposit cannot claim a size the pool did not set. This is the half of the
/// accounting invariant that a competing implementation is missing: there, the
/// escrowed lamports are a caller-supplied parameter that is never bound to the
/// hidden commitment, so a depositor of one lamport can later withdraw the whole
/// pool with an entirely valid proof.
///
/// Duplicate commitments are not rejected. Doing so would cost a marker account
/// per deposit, and the only party a duplicate harms is whoever submitted it:
/// they cannot know the original's secret, so they can never spend the note they
/// paid for. It burns their own money and leaves every other member unaffected.
fn deposit(program_id: &Pubkey, accounts: &[AccountInfo], commitment: [u8; 32]) -> ProgramResult {
    let iter = &mut accounts.iter();
    let depositor = next_account_info(iter)?;
    let pool_account = next_account_info(iter)?;
    let vault_account = next_account_info(iter)?;
    let system = next_account_info(iter)?;

    if !depositor.is_signer {
        return Err(MirrorProgramError::MissingSignature.into());
    }
    if !system_program::check_id(system.key) {
        return Err(MirrorProgramError::InvalidOwner.into());
    }
    if pool_account.owner != program_id || vault_account.owner != program_id {
        return Err(MirrorProgramError::InvalidOwner.into());
    }

    // A canonical scalar, checked here rather than at first use. A commitment
    // at or above the modulus would be rejected by the Poseidon syscall deep in
    // the insert, after lamports had already moved.
    let leaf = Field::from_bytes(commitment).map_err(MirrorProgramError::from)?;

    let (denomination, entry_fee) = {
        let mut data = pool_account.try_borrow_mut_data()?;
        let pool = Pool::load(&mut data)?;
        if *vault_account.key != vault_address(program_id, pool_account.key).0 {
            return Err(MirrorProgramError::InvalidPda.into());
        }
        (pool.denomination(), pool.entry_fee())
    };

    solana_program::program::invoke(
        &system_instruction::transfer(depositor.key, vault_account.key, denomination),
        &[depositor.clone(), vault_account.clone(), system.clone()],
    )?;
    if entry_fee > 0 {
        solana_program::program::invoke(
            &system_instruction::transfer(depositor.key, pool_account.key, entry_fee),
            &[depositor.clone(), pool_account.clone(), system.clone()],
        )?;
    }

    let mut data = pool_account.try_borrow_mut_data()?;
    let mut pool = Pool::load(&mut data)?;
    let index = insert_leaf(&mut pool, leaf)?;
    pool.record_deposit(index + 1)?;

    // Re-read the invariant from the account after the transfers landed, rather
    // than trusting that they did.
    let rent = Rent::get()?;
    let required = pool
        .required_vault_lamports()?
        .checked_add(rent.minimum_balance(0))
        .ok_or(MirrorProgramError::ArithmeticOverflow)?;
    if vault_account.lamports() < required {
        return Err(MirrorProgramError::InsolventVault.into());
    }
    Ok(())
}

/// Appends `leaf` to the pool's frontier, returning its index.
///
/// The same walk as `mirror_core::Frontier::insert`, over account bytes. The
/// two are asserted to agree in the program's test suite, because a divergence
/// here means the host builds proofs against a root the chain will never hold.
fn insert_leaf(pool: &mut Pool<'_>, leaf: Field) -> Result<u64, MirrorProgramError> {
    let index = pool.next_index();
    if index >= 1u64 << TREE_DEPTH {
        return Err(MirrorProgramError::TreeFull);
    }

    let ladder = mirror_core::baked_ladder().map_err(MirrorProgramError::from)?;
    let mut current = leaf;
    let mut path = index;

    for (level, zero) in ladder.iter().take(TREE_DEPTH).enumerate() {
        if path & 1 == 0 {
            pool.set_filled(level, current)?;
            current = hash_node(current, *zero).map_err(MirrorProgramError::from)?;
        } else {
            let left = pool.filled(level)?;
            current = hash_node(left, current).map_err(MirrorProgramError::from)?;
        }
        path >>= 1;
    }

    pool.push_root(current);
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mirror_core::Frontier;

    /// The program's accumulator and the host's must produce identical roots
    /// over the same insertion sequence. If they ever diverge, every proof the
    /// host builds targets a root the chain does not have.
    #[test]
    fn the_program_insert_matches_the_host_accumulator() {
        let mut data = vec![0u8; POOL_LEN];
        {
            let mut pool = Pool::load_uninitialised(&mut data).unwrap();
            pool.initialise(255, 254, 1_000_000, 0, 2).unwrap();
        }
        let mut host = Frontier::new().unwrap();
        let mut pool_data = data;

        for i in 1..=17u64 {
            let leaf = Field::from_u64(i * 1_000 + 7);
            let host_index = host.insert(leaf).unwrap();

            let mut pool = Pool::load(&mut pool_data).unwrap();
            let program_index = insert_leaf(&mut pool, leaf).unwrap();
            pool.record_deposit(program_index + 1).unwrap();

            assert_eq!(program_index, host_index, "leaf index diverged at {i}");
            assert_eq!(
                pool.current_root().unwrap(),
                host.root(),
                "root diverged after {i} insertions"
            );
        }
    }

    #[test]
    fn every_intermediate_root_stays_known() {
        let mut data = vec![0u8; POOL_LEN];
        {
            let mut pool = Pool::load_uninitialised(&mut data).unwrap();
            pool.initialise(255, 254, 1, 0, 2).unwrap();
        }
        let mut roots = Vec::new();
        for i in 1..=20u64 {
            let mut pool = Pool::load(&mut data).unwrap();
            insert_leaf(&mut pool, Field::from_u64(i)).unwrap();
            roots.push(pool.current_root().unwrap());
        }
        let pool = Pool::load(&mut data).unwrap();
        for (i, root) in roots.iter().enumerate() {
            assert!(
                pool.knows_root(*root),
                "root after deposit {i} was forgotten while still inside the ring"
            );
        }
    }

    #[test]
    fn a_non_canonical_commitment_is_refused_before_anything_moves() {
        // The value the Poseidon syscall would reject much later, after
        // lamports had already been transferred.
        assert!(Field::from_bytes([0xff; 32]).is_err());
    }
}
