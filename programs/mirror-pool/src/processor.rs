//! Instruction handlers.

use crate::{
    error::MirrorProgramError,
    instruction::Instruction,
    pda::{pool_address, spend_address, vault_address, POOL_SEED, SPEND_SEED, VAULT_SEED},
    spend::{spend_len, Spend, MAX_PAYLOAD},
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
            target_program,
            beneficiary,
            relay_fee,
            action_accounts,
            payload,
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
                target_program,
                beneficiary,
                relay_fee,
                action_accounts,
                payload,
            },
        ),
        Instruction::SettleEpoch { count } => settle_epoch(program_id, accounts, count),
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
    target_program: [u8; 32],
    beneficiary: [u8; 32],
    relay_fee: u64,
    action_accounts: u8,
    payload: Vec<u8>,
}

/// Creates a program-owned account at a PDA, tolerating lamports already sent to
/// it.
///
/// `system_instruction::create_account` fails outright when the destination
/// holds any lamports, and that is a griefing vector rather than a safeguard:
/// every PDA this program creates is derived from public data. A nullifier is
/// visible in the `submit_spend` instruction, so anyone who sees the transaction
/// — most obviously the relay it was handed to — can send the rent-exempt
/// minimum to that spend PDA first and make the note permanently unspendable for
/// about 0.00089 SOL. The same trick on a pool or vault address prevents that
/// denomination's pool from ever being created.
///
/// The three-step form is immune: top the account up to rent exemption, then
/// allocate and assign under the PDA's own seeds. A squatter can send lamports
/// but cannot allocate or assign without those seeds, so their deposit only
/// reduces what the legitimate payer owes.
fn create_pda_account<'a>(
    payer: &AccountInfo<'a>,
    account: &AccountInfo<'a>,
    system: &AccountInfo<'a>,
    space: usize,
    owner: &Pubkey,
    seeds: &[&[u8]],
) -> ProgramResult {
    let rent = Rent::get()?;
    let required = rent.minimum_balance(space);
    let held = account.lamports();

    if held < required {
        solana_program::program::invoke(
            &system_instruction::transfer(payer.key, account.key, required - held),
            &[payer.clone(), account.clone(), system.clone()],
        )?;
    }
    invoke_signed(
        &system_instruction::allocate(account.key, space as u64),
        &[account.clone(), system.clone()],
        &[seeds],
    )?;
    invoke_signed(
        &system_instruction::assign(account.key, owner),
        &[account.clone(), system.clone()],
        &[seeds],
    )
}

/// Creates the pool and its vault and seeds the accumulator to an empty tree.
///
/// Permissionless: the first caller for a denomination creates the crowd
/// everyone else joins. There is no privileged authority, so there is no key
/// whose loss freezes the pool. Baking an authority into the pool's PDA seeds is
/// the tempting alternative and it is a one-way door: seeds cannot change, so
/// without a rotation instruction that authority is permanent, a rotating relay
/// becomes undeployable, and the escrow is hostage to a single key forever.
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

    let denom_le = denomination.to_le_bytes();

    // The pool account carries the accumulator and, above its own rent, the
    // reward pool that entry fees accrue into.
    create_pda_account(
        payer,
        pool_account,
        system,
        POOL_LEN,
        program_id,
        &[POOL_SEED, &denom_le, &[pool_bump]],
    )?;

    // The vault holds escrow only, and carries no data of its own: the
    // accounting invariant is a statement about its lamports, so anything else
    // living there would muddy it.
    create_pda_account(
        payer,
        vault_account,
        system,
        0,
        program_id,
        &[VAULT_SEED, expected_pool.as_ref(), &[vault_bump]],
    )?;

    let mut data = pool_account.try_borrow_mut_data()?;
    let mut pool = Pool::load_uninitialised(&mut data)?;
    pool.initialise(pool_bump, vault_bump, denomination, entry_fee, k_floor)?;
    Ok(())
}

/// Escrows exactly one denomination and appends a note commitment.
///
/// The escrowed amount is read from the pool, never from the instruction, so a
/// deposit cannot claim a size the pool did not set. This is the ingress half of
/// the accounting invariant, and it is the half most easily left out: take the
/// escrowed lamports as a caller-supplied parameter without binding them to the
/// hidden commitment, and a depositor of one lamport can later withdraw the
/// whole pool with an entirely valid proof.
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

    // The replay guard, and it runs before the pairing rather than after.
    //
    // A spent nullifier is knowable from one account read; verifying a proof
    // costs about 95k compute units. Checking the cheap thing first means a
    // replay is rejected at roughly 11k CU instead of 95k, which matters because
    // replays are exactly what an attacker submits in bulk.
    //
    // It is also an explicit check rather than a reliance on account creation
    // failing: a failed CPI terminates the instruction with the *system
    // program's* error, so a replay would report a bare "already in use" that
    // proves nothing about this program. Devnet evidence records error codes,
    // and a code that could have come from anywhere is not evidence.
    let (expected_spend, spend_bump) = spend_address(program_id, pool_account.key, &req.nullifier);
    if *spend_account.key != expected_spend {
        return Err(MirrorProgramError::InvalidPda.into());
    }
    // Keyed on allocated data, not on lamports. Anyone can send lamports to a
    // PDA derived from a public nullifier; only this program can allocate it.
    // Treating a balance as "already spent" would let a bystander brick a note
    // for the price of rent exemption.
    if !spend_account.data_is_empty() {
        return Err(MirrorProgramError::NullifierAlreadySpent.into());
    }

    // Recomputed, never transmitted. If the relay altered the selector, the
    // beneficiary or its own fee, this binding differs from the one the prover
    // committed to and the pairing fails. There is no separate field that could
    // be checked incorrectly or forgotten.
    if req.payload.len() > MAX_PAYLOAD {
        return Err(MirrorProgramError::PayloadTooLarge.into());
    }
    // Refuse a shape settlement could never satisfy, here rather than at
    // settlement. Reaching settlement means the nullifier has already burned,
    // and there is no instruction that can amend or refund a spend — so a
    // record that cannot settle is a note destroyed. The binding covers these
    // fields too; this check turns a griefing attempt into a failed transaction
    // instead of a failed transaction plus a dead note.
    match req.selector {
        SELECTOR_TRANSFER if req.action_accounts != 0 => {
            return Err(MirrorProgramError::MalformedInstruction.into());
        }
        SELECTOR_TRANSFER | SELECTOR_INVOKE => {}
        _ => return Err(MirrorProgramError::UnknownSelector.into()),
    }
    let binding = mirror_core::action_binding(
        req.selector,
        &req.target_program,
        &req.beneficiary,
        req.relay_fee,
        req.action_accounts,
        &req.payload,
    );

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

    create_pda_account(
        relay,
        spend_account,
        system,
        spend_len(req.payload.len()),
        program_id,
        &[
            SPEND_SEED,
            pool_account.key.as_ref(),
            &req.nullifier,
            &[spend_bump],
        ],
    )?;

    let now = solana_program::clock::Clock::get()?.unix_timestamp;
    let mut spend_data = spend_account.try_borrow_mut_data()?;
    let mut record = Spend::load_uninitialised(&mut spend_data)?;
    record.initialise(
        spend_bump,
        req.selector,
        req.relay_fee,
        now,
        &req.beneficiary,
        &relay.key.to_bytes(),
        &pool_account.key.to_bytes(),
        &req.target_program,
        req.action_accounts,
        &req.payload,
    )?;

    // The spend counter is deliberately *not* advanced here. The note is
    // committed to be paid but has not been paid, so the vault must still cover
    // it; leaving the counter alone keeps `required_vault_lamports` an upper
    // bound on what is owed until settlement actually disburses.
    Ok(())
}

/// The plain-transfer action: pay the beneficiary, no CPI.
///
/// Kept as selector zero because a transfer is the degenerate action and the
/// protocol should not need a target program to express it.
pub const SELECTOR_TRANSFER: u64 = 0;

/// Invoke `target_program` with the stored payload, signed by the pool.
///
/// This is what makes the pool a behavioural anonymity set rather than a value
/// mixer. The brief asks for "Tornado Cash for behavioural patterns and
/// withdrawals — not for funds": an observer should see that a stake, a swap or
/// a vote happened and be unable to say which member asked for it. A pool that
/// only moves lamports answers the wrong question.
pub const SELECTOR_INVOKE: u64 = 1;

/// How long a spend may wait before it can settle alone.
///
/// Below the crowd size, a batch must wait this out. It is the escape valve that
/// makes the crowd requirement safe: without it, a quiet pool could hold a
/// member's funds indefinitely because the crowd never arrives, and a privacy
/// tool that can strand your money is not one anybody should use.
pub const SETTLE_TIMEOUT_SECONDS: i64 = 3_600;

/// Executes a batch of pending spends in one transaction.
///
/// Every payout in the batch shares a timestamp and an ordering, which is what
/// makes the crowd synchronised: an observer watching beneficiaries receive
/// funds cannot use arrival time to tell them apart.
///
/// Permissionless. Anyone may settle, so no operator's absence can strand a
/// member — and a member can always settle their own batch once the timeout has
/// passed.
///
/// The crowd rule: a batch must carry at least `k_floor` spends, **or** every
/// spend in it must have waited out `SETTLE_TIMEOUT_SECONDS`. Requiring the crowd
/// unconditionally would be a liveness hazard on a quiet pool; dropping the
/// requirement would make "synchronised" a word rather than a property. This is
/// the honest middle: synchronised when there is traffic, still liquid when
/// there is not.
fn settle_epoch(program_id: &Pubkey, accounts: &[AccountInfo], count: u8) -> ProgramResult {
    if count == 0 {
        return Err(MirrorProgramError::MalformedInstruction.into());
    }

    let iter = &mut accounts.iter();
    let settler = next_account_info(iter)?;
    let pool_account = next_account_info(iter)?;
    let vault_account = next_account_info(iter)?;

    if !settler.is_signer {
        return Err(MirrorProgramError::MissingSignature.into());
    }
    if pool_account.owner != program_id || vault_account.owner != program_id {
        return Err(MirrorProgramError::InvalidOwner.into());
    }
    if *vault_account.key != vault_address(program_id, pool_account.key).0 {
        return Err(MirrorProgramError::InvalidPda.into());
    }

    let (denomination, k_floor) = {
        let mut data = pool_account.try_borrow_mut_data()?;
        let pool = Pool::load(&mut data)?;
        (pool.denomination(), pool.k_floor())
    };

    let now = solana_program::clock::Clock::get()?.unix_timestamp;
    let crowd_satisfied = count as u32 >= k_floor;

    // Each spend brings its record, its beneficiary and its relay.
    let mut settled = 0u64;
    for _ in 0..count {
        let spend_account = next_account_info(iter)?;
        let beneficiary = next_account_info(iter)?;
        let relay = next_account_info(iter)?;

        if spend_account.owner != program_id {
            return Err(MirrorProgramError::InvalidOwner.into());
        }

        let (relay_fee, payout, selector, action_accounts) = {
            let mut data = spend_account.try_borrow_mut_data()?;
            let mut record = Spend::load(&mut data)?;

            // A record from another pool must not reach this vault.
            if record.pool() != pool_account.key.to_bytes() {
                return Err(MirrorProgramError::InvalidPda.into());
            }
            // The accounts must be the ones the proof bound.
            if record.beneficiary() != beneficiary.key.to_bytes()
                || record.relay() != relay.key.to_bytes()
            {
                return Err(MirrorProgramError::InvalidPda.into());
            }
            if !crowd_satisfied
                && now.saturating_sub(record.submitted_at()) < SETTLE_TIMEOUT_SECONDS
            {
                return Err(MirrorProgramError::CrowdTooSmall.into());
            }

            // Refuses a second settlement of the same record, which is the
            // double-spend an attacker would reach for: pass one pending spend
            // twice in a single batch and be paid twice.
            record.mark_settled()?;

            let relay_fee = record.relay_fee();
            let payout = denomination
                .checked_sub(relay_fee)
                .ok_or(MirrorProgramError::ArithmeticOverflow)?;
            (
                relay_fee,
                payout,
                record.selector(),
                record.action_accounts(),
            )
        };

        // The relay is paid the same way regardless of what the action is.
        if relay_fee > 0 {
            move_lamports(vault_account, relay, relay_fee)?;
        }

        match selector {
            SELECTOR_TRANSFER => {
                if action_accounts != 0 {
                    return Err(MirrorProgramError::MalformedInstruction.into());
                }
                // The vault is owned by this program, so lamports move by direct
                // mutation rather than a system CPI: no signer seeds and no
                // nested invoke on the hot path.
                move_lamports(vault_account, beneficiary, payout)?;
            }
            SELECTOR_INVOKE => {
                // The target program's own account comes first, because a CPI
                // requires the callee to be present in the caller's account
                // list. Its key is checked against the record, so a settler
                // supplying a different program is refused before any value
                // moves rather than discovered by the runtime.
                let target_info = next_account_info(iter)?;

                // Then the action's own accounts.
                let mut action_infos = Vec::with_capacity(action_accounts as usize);
                for _ in 0..action_accounts {
                    action_infos.push(next_account_info(iter)?.clone());
                }
                invoke_action(
                    program_id,
                    pool_account,
                    vault_account,
                    spend_account,
                    beneficiary,
                    target_info,
                    payout,
                    &action_infos,
                )?;
            }
            _ => return Err(MirrorProgramError::UnknownSelector.into()),
        }
        settled += 1;
    }

    let mut data = pool_account.try_borrow_mut_data()?;
    let mut pool = Pool::load(&mut data)?;
    for _ in 0..settled {
        pool.record_spend()?;
    }

    // The accounting invariant, re-read from the account after the lamports
    // actually moved rather than assumed from the arithmetic above.
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

/// Invokes the member's chosen program on their behalf, signed by the vault.
///
/// The vault PDA is the signer, so from the chain's point of view the action was
/// taken by the pool. Every member's action carries the same signer, which is
/// what makes an action unattributable: the on-chain trace of a stake made
/// through this pool is identical whoever asked for it.
///
/// The payload and the target were both fixed at submit time and bound into the
/// proof, so a settler chooses neither, and the declared account count is bound
/// too. What a settler still supplies is *which* accounts fill those slots, and
/// those are the target program's problem to validate — exactly as they would be
/// for any caller. For a target whose destination is an account rather than
/// instruction data, that is a real limit, and the threat model says so.
///
/// The vault is appended by this program as the final account and marked signer.
/// A settler that tries to place it in the action's own list is refused, because
/// the same account appearing twice has its lamport change applied twice.
#[allow(clippy::too_many_arguments)]
fn invoke_action<'a>(
    program_id: &Pubkey,
    pool_account: &AccountInfo<'a>,
    vault_account: &AccountInfo<'a>,
    spend_account: &AccountInfo<'a>,
    beneficiary: &AccountInfo<'a>,
    target_info: &AccountInfo<'a>,
    payout: u64,
    action_infos: &[AccountInfo<'a>],
) -> ProgramResult {
    let (target, payload) = {
        let mut data = spend_account.try_borrow_mut_data()?;
        let record = Spend::load(&mut data)?;
        (record.target_program(), record.payload().to_vec())
    };
    let vault_bump = {
        let pool_data = pool_account.try_borrow_data()?;
        *pool_data
            .get(2)
            .ok_or(MirrorProgramError::InvalidPoolAccount)?
    };

    // The pool never invokes itself. Doing so would let a member craft a payload
    // that re-enters settlement, and re-entrancy around a lamport-moving loop is
    // not something to leave to careful reading.
    let target_key = Pubkey::new_from_array(target);
    if target_key == *program_id {
        return Err(MirrorProgramError::SelfInvocationRefused.into());
    }
    // The account supplied must be the program the member proved.
    if *target_info.key != target_key {
        return Err(MirrorProgramError::InvalidPda.into());
    }

    // Fund the action before invoking, so the target sees the value it is meant
    // to act on. The vault signs, so the funds visibly come from the pool.
    move_lamports(vault_account, beneficiary, payout)?;

    // The vault authorises the call through its seeds but is deliberately not
    // one of the callee's accounts.
    //
    // This program has already moved the payout out of the vault by direct
    // mutation. Handing that same account to a callee makes the runtime
    // reconcile those lamports across the CPI boundary, and it rejects the whole
    // instruction as unbalanced — with the vault marked writable or not. So the
    // vault reaches `invoke_signed` in the account infos, which is what lets its
    // seeds sign, and never appears in the instruction's account list.
    //
    // The consequence is a real limit, and THREAT_MODEL.md states it: an action
    // whose target needs the pool itself as one of its accounts is not
    // expressible here. Value reaches the action through the beneficiary.
    if action_infos.iter().any(|a| a.key == vault_account.key) {
        return Err(MirrorProgramError::MalformedInstruction.into());
    }

    let metas: Vec<solana_program::instruction::AccountMeta> = action_infos
        .iter()
        .map(|a| solana_program::instruction::AccountMeta {
            pubkey: *a.key,
            is_signer: a.is_signer,
            is_writable: a.is_writable,
        })
        .collect();

    let ix = solana_program::instruction::Instruction {
        program_id: target_key,
        accounts: metas,
        data: payload,
    };

    // The vault and the target may already be among the action's accounts — the
    // vault whenever the action needs the pool to sign for it, which is the
    // normal case. Appending them unconditionally puts the same account in the
    // list twice, and the runtime then reconciles its lamports against itself
    // and fails the whole instruction as unbalanced.
    let mut infos = action_infos.to_vec();
    infos.push(vault_account.clone());
    if !infos.iter().any(|a| a.key == target_info.key) {
        infos.push(target_info.clone());
    }

    invoke_signed(
        &ix,
        &infos,
        &[&[VAULT_SEED, pool_account.key.as_ref(), &[vault_bump]]],
    )
}

/// Moves lamports between two accounts this program owns or may credit.
fn move_lamports(
    from: &AccountInfo,
    to: &AccountInfo,
    amount: u64,
) -> Result<(), MirrorProgramError> {
    let mut from_lamports = from
        .try_borrow_mut_lamports()
        .map_err(|_| MirrorProgramError::InsolventVault)?;
    let mut to_lamports = to
        .try_borrow_mut_lamports()
        .map_err(|_| MirrorProgramError::InsolventVault)?;
    **from_lamports = from_lamports
        .checked_sub(amount)
        .ok_or(MirrorProgramError::InsolventVault)?;
    **to_lamports = to_lamports
        .checked_add(amount)
        .ok_or(MirrorProgramError::ArithmeticOverflow)?;
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
