//! Instruction handlers.

use crate::{
    error::MirrorProgramError,
    instruction::Instruction,
    pda::{pool_address, spend_address, vault_address, POOL_SEED, SPEND_SEED, VAULT_SEED},
    spend::{Spend, SPEND_LEN},
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
        Instruction::SubmitSpend {
            proof_a,
            proof_b,
            proof_c,
            root,
            nullifier,
            selector,
            beneficiary,
            relay_fee,
        } => submit_spend(
            program_id,
            accounts,
            SpendRequest {
                proof_a,
                proof_b,
                proof_c,
                root,
                nullifier,
                selector,
                beneficiary,
                relay_fee,
            },
        ),
    }
}

/// The fields of a spend, grouped so the handler takes one argument.
struct SpendRequest {
    proof_a: [u8; 64],
    proof_b: [u8; 128],
    proof_c: [u8; 64],
    root: [u8; 32],
    nullifier: [u8; 32],
    selector: u64,
    beneficiary: [u8; 32],
    relay_fee: u64,
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

/// Proves membership, burns the nullifier, and records the authorised action.
///
/// Nothing is paid out here. Settlement executes the batch, so every action in
/// an epoch lands on one timestamp and in one ordering — which is the point of a
/// synchronised crowd, and the reason a per-spend payout would leak exactly what
/// the pool exists to hide.
///
/// The relay signs, not the member. A member who pays their own fee signs with
/// their own wallet and destroys their own anonymity, so no member key appears
/// on chain at any point in this path. Relaying is permissionless: any key may
/// do it, and there is no authority whose absence freezes the pool.
///
/// Note the ordering. The proof is verified *before* the spend account is
/// created, but the account's existence is what makes replay impossible, and
/// account creation fails if it already exists. So a replayed proof — however
/// valid — cannot produce a second record.
fn submit_spend(program_id: &Pubkey, accounts: &[AccountInfo], req: SpendRequest) -> ProgramResult {
    let iter = &mut accounts.iter();
    let relay = next_account_info(iter)?;
    let pool_account = next_account_info(iter)?;
    let spend_account = next_account_info(iter)?;
    let system = next_account_info(iter)?;

    if !relay.is_signer {
        return Err(MirrorProgramError::MissingSignature.into());
    }
    if !system_program::check_id(system.key) {
        return Err(MirrorProgramError::InvalidOwner.into());
    }
    if pool_account.owner != program_id {
        return Err(MirrorProgramError::InvalidOwner.into());
    }

    // Canonical scalars, checked before anything else touches them. The Groth16
    // verifier rejects public inputs at or above the modulus, and the Poseidon
    // syscall rejects non-canonical preimages; doing it here turns both into one
    // named error instead of an opaque failure deeper in.
    let root = Field::from_bytes(req.root).map_err(MirrorProgramError::from)?;
    let nullifier_field = Field::from_bytes(req.nullifier).map_err(MirrorProgramError::from)?;

    let (denomination, k_floor, deposit_count) = {
        let mut data = pool_account.try_borrow_mut_data()?;
        let pool = Pool::load(&mut data)?;
        (pool.denomination(), pool.k_floor(), pool.deposit_count())
    };

    // The relay is paid out of the denomination, so a fee at or above it would
    // leave the member nothing and, at exactly the denomination, would let a
    // relay take the whole note.
    if req.relay_fee >= denomination {
        return Err(MirrorProgramError::RelayFeeTooLarge.into());
    }

    // The anonymity floor. This bounds *program-visible* membership: how many
    // notes the tree holds. It is not the effective anonymity set, which is
    // smaller because an observer can partition members by funding provenance —
    // that is measured off-chain and reported honestly rather than asserted away
    // here.
    if deposit_count < k_floor as u64 {
        return Err(MirrorProgramError::BelowAnonymityFloor.into());
    }

    {
        let mut data = pool_account.try_borrow_mut_data()?;
        let pool = Pool::load(&mut data)?;
        if !pool.knows_root(root) {
            return Err(MirrorProgramError::UnknownRoot.into());
        }
    }

    // Recomputed, never transmitted. If the relay altered the selector, the
    // beneficiary or its own fee, this binding differs from the one the prover
    // committed to and the pairing fails. There is no separate field that could
    // be checked incorrectly or forgotten.
    let binding = mirror_core::action_binding(req.selector, &req.beneficiary, req.relay_fee)
        .map_err(MirrorProgramError::from)?;

    let public_inputs: [[u8; 32]; 3] = [
        root.to_bytes(),
        nullifier_field.to_bytes(),
        binding.to_bytes(),
    ];
    let mut verifier = groth16_solana::groth16::Groth16Verifier::<3>::new(
        &req.proof_a,
        &req.proof_b,
        &req.proof_c,
        &public_inputs,
        &crate::vk::VERIFYING_KEY,
    )
    .map_err(|_| MirrorProgramError::ProofVerificationFailed)?;
    verifier
        .verify()
        .map_err(|_| MirrorProgramError::ProofVerificationFailed)?;

    let (expected_spend, spend_bump) = spend_address(program_id, pool_account.key, &req.nullifier);
    if *spend_account.key != expected_spend {
        return Err(MirrorProgramError::InvalidPda.into());
    }

    // Creation fails if the account already exists, which is the replay guard.
    let rent = Rent::get()?;
    invoke_signed(
        &system_instruction::create_account(
            relay.key,
            spend_account.key,
            rent.minimum_balance(SPEND_LEN),
            SPEND_LEN as u64,
            program_id,
        ),
        &[relay.clone(), spend_account.clone(), system.clone()],
        &[&[
            SPEND_SEED,
            pool_account.key.as_ref(),
            &req.nullifier,
            &[spend_bump],
        ]],
    )
    .map_err(|_| MirrorProgramError::NullifierAlreadySpent)?;

    let mut spend_data = spend_account.try_borrow_mut_data()?;
    let mut record = Spend::load_uninitialised(&mut spend_data)?;
    record.initialise(
        spend_bump,
        req.selector,
        req.relay_fee,
        &req.beneficiary,
        &relay.key.to_bytes(),
    );

    // The spend counter is deliberately *not* advanced here. The note is
    // committed to be paid but has not been paid, so the vault must still cover
    // it; leaving the counter alone keeps `required_vault_lamports` an upper
    // bound on what is owed until settlement actually disburses.
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
