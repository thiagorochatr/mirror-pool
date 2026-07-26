//! The member-facing half of the tool: join a pool, act through it, settle it.
//!
//! Every command here reads what it needs from the chain. Nothing depends on a
//! file this tool wrote earlier except the note itself, and nothing depends on a
//! server we run. A member who keeps their note file can act from a fresh
//! machine; a member who loses it has lost the deposit, and no operator can
//! change that in either direction.

use crate::chain::Chain;
use crate::history::{self, History};
use crate::note::StoredNote;
use anyhow::{anyhow, Context, Result};
use ark_std::rand::SeedableRng;
use mirror_pool_program::{
    instruction::Instruction as MirrorIx,
    pda::{pool_address, spend_address, vault_address},
    processor::{SELECTOR_INVOKE, SELECTOR_INVOKE_SIGNED, SELECTOR_TRANSFER},
    spend::{Spend, STATUS_PENDING},
    Pool,
};
use solana_keypair::Keypair;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::path::Path;

/// What the pool account says about itself.
pub struct PoolState {
    pub pool: Pubkey,
    pub vault: Pubkey,
    pub denomination: u64,
    pub k_floor: u32,
    pub deposits: u64,
    pub spends: u64,
    pub root: [u8; 32],
}

pub fn read_pool(chain: &Chain, program_id: &Pubkey, denomination: u64) -> Result<PoolState> {
    let (pool, _) = pool_address(program_id, denomination);
    let (vault, _) = vault_address(program_id, &pool);
    let mut data = chain.account_data(&pool)?.ok_or_else(|| {
        anyhow!(
            "no pool exists at {pool} for denomination {denomination}. \
             Pool creation is permissionless — `mirror init-pool` makes one."
        )
    })?;
    let state =
        Pool::load(&mut data).map_err(|e| anyhow!("{pool} is not a pool account: {e:?}"))?;
    Ok(PoolState {
        pool,
        vault,
        denomination: state.denomination(),
        k_floor: state.k_floor(),
        deposits: state.deposit_count(),
        spends: state.spend_count(),
        root: state
            .current_root()
            .map_err(|e| anyhow!("{e:?}"))?
            .to_bytes(),
    })
}

fn send(chain: &Chain, ix: Instruction, signers: &[&Keypair]) -> Result<String> {
    let blockhash = chain.latest_blockhash()?;
    let message = solana_message::Message::new(&[ix], Some(&signers[0].pubkey()));
    let tx = Transaction::new(signers, message, blockhash);
    chain.send(&tx)
}

// ---------------------------------------------------------------------------
// init-pool

pub fn init_pool(
    chain: &Chain,
    program_id: &Pubkey,
    denomination: u64,
    k_floor: u32,
    payer: &Keypair,
) -> Result<()> {
    let (pool, _) = pool_address(program_id, denomination);
    let (vault, _) = vault_address(program_id, &pool);
    if chain.account_data(&pool)?.is_some() {
        println!("the pool for denomination {denomination} already exists at {pool}");
        println!("nothing to do — one pool per denomination is the whole point");
        return Ok(());
    }
    let ix = Instruction::new_with_bytes(
        *program_id,
        &MirrorIx::InitPool {
            denomination,
            entry_fee: 0,
            k_floor,
        }
        .pack(),
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(pool, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        ],
    );
    let sig = send(chain, ix, &[payer])?;
    println!("pool    {pool}");
    println!("vault   {vault}");
    println!("signature {sig}");
    Ok(())
}

// ---------------------------------------------------------------------------
// deposit

pub fn deposit(
    chain: &Chain,
    program_id: &Pubkey,
    note_path: &Path,
    depositor: &Keypair,
) -> Result<()> {
    let stored = StoredNote::read(note_path)?;
    let note = stored.note()?;
    let state = read_pool(chain, program_id, stored.denomination)?;
    let commitment = stored.commitment_bytes()?;

    // Refuse a second deposit of the same note before it costs anything. The
    // program allows duplicate commitments — the only party a duplicate harms is
    // whoever paid for it — but a member doing it by accident is out a
    // denomination for a leaf they cannot distinguish from their first.
    let history = history::scan(chain, program_id, &state.pool, false)?;
    if history.index_of(&commitment).is_some() {
        return Err(anyhow!(
            "this note's commitment is already a leaf of this pool. Depositing it \
             again would escrow a second denomination against a note you cannot \
             tell apart from the first."
        ));
    }

    let balance = chain.balance(&depositor.pubkey())?;
    if balance < state.denomination {
        return Err(anyhow!(
            "{} holds {} lamports and this pool escrows {} per deposit",
            depositor.pubkey(),
            balance,
            state.denomination
        ));
    }

    let ix = Instruction::new_with_bytes(
        *program_id,
        &MirrorIx::Deposit { commitment }.pack(),
        vec![
            AccountMeta::new(depositor.pubkey(), true),
            AccountMeta::new(state.pool, false),
            AccountMeta::new(state.vault, false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        ],
    );
    let sig = send(chain, ix, &[depositor])?;
    println!(
        "deposited {} lamports into {}",
        state.denomination, state.pool
    );
    println!("leaf      {}", state.deposits);
    println!("signature {sig}");
    println!();
    println!(
        "Keep {} safe. It is the only way to spend this note,",
        note_path.display()
    );
    println!("and nobody — including this pool's authors — can reissue it.");
    let _ = note;
    Ok(())
}

// ---------------------------------------------------------------------------
// tree

/// Rebuilds the accumulator from chain history and checks it against the pool.
///
/// The check is the point. A rebuilt tree whose root matches the one the program
/// holds is proof that the history is complete and correctly ordered, which is
/// exactly the precondition a membership proof needs. A mismatch means any proof
/// built here would fail on-chain for reasons that would be very hard to read
/// backwards from the failure.
pub fn tree(chain: &Chain, program_id: &Pubkey, denomination: u64) -> Result<History> {
    let state = read_pool(chain, program_id, denomination)?;
    println!("pool          {}", state.pool);
    println!("vault         {}", state.vault);
    println!("denomination  {}", state.denomination);
    println!("k floor       {}", state.k_floor);
    println!(
        "notes         {} deposited, {} settled, {} outstanding",
        state.deposits,
        state.spends,
        state.deposits.saturating_sub(state.spends)
    );
    println!();
    println!("rebuilding the accumulator from chain history:");

    let history = history::scan(chain, program_id, &state.pool, true)?;
    let rebuilt = history.tree()?;
    let root = rebuilt.root().map_err(|e| anyhow!("{e:?}"))?;

    println!();
    println!("  leaves recovered  {}", history.commitments.len());
    println!("  pool reports      {}", state.deposits);
    println!("  rebuilt root      {}", hex::encode(root.to_bytes()));
    println!("  on-chain root     {}", hex::encode(state.root));

    if history.commitments.len() as u64 != state.deposits {
        return Err(anyhow!(
            "recovered {} leaves but the pool has inserted {}. The history is \
             incomplete — most likely the endpoint has pruned it. `mirror \
             check-endpoint` tests for exactly that.",
            history.commitments.len(),
            state.deposits
        ));
    }
    if root.to_bytes() != state.root {
        return Err(anyhow!(
            "the rebuilt root does not match the pool's. The leaf set or its \
             order is wrong, and any proof built against this tree would be \
             rejected on-chain."
        ));
    }
    println!();
    println!("  the rebuilt tree matches the chain — proofs built from it will verify");
    Ok(history)
}

// ---------------------------------------------------------------------------
// spend

/// What a member is asking the pool to do.
pub enum Action {
    /// Pay the beneficiary, no CPI.
    Transfer,
    /// Call `target` with `payload`. `pool_signs` picks the selector that hands
    /// the vault to the callee as a signer.
    Invoke {
        target: Pubkey,
        payload: Vec<u8>,
        accounts: u8,
        pool_signs: bool,
    },
}

impl Action {
    fn selector(&self) -> u64 {
        match self {
            Action::Transfer => SELECTOR_TRANSFER,
            Action::Invoke {
                pool_signs: false, ..
            } => SELECTOR_INVOKE,
            Action::Invoke {
                pool_signs: true, ..
            } => SELECTOR_INVOKE_SIGNED,
        }
    }
    fn target(&self) -> [u8; 32] {
        match self {
            Action::Transfer => [0u8; 32],
            Action::Invoke { target, .. } => target.to_bytes(),
        }
    }
    fn payload(&self) -> &[u8] {
        match self {
            Action::Transfer => &[],
            Action::Invoke { payload, .. } => payload,
        }
    }
    fn accounts(&self) -> u8 {
        match self {
            Action::Transfer => 0,
            Action::Invoke { accounts, .. } => *accounts,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn spend(
    chain: &Chain,
    program_id: &Pubkey,
    note_path: &Path,
    beneficiary: &Pubkey,
    relay: &Keypair,
    relay_fee: u64,
    action: Action,
) -> Result<()> {
    let stored = StoredNote::read(note_path)?;
    let note = stored.note()?;
    let state = read_pool(chain, program_id, stored.denomination)?;

    if relay.pubkey() == *beneficiary {
        eprintln!(
            "warning: the relay and the beneficiary are the same key, which links \
             the payout to whoever paid the fee."
        );
    }
    if relay_fee >= state.denomination {
        return Err(anyhow!(
            "a relay fee of {relay_fee} is not less than the denomination {}",
            state.denomination
        ));
    }
    if state.deposits < state.k_floor as u64 {
        return Err(anyhow!(
            "this pool holds {} notes and its floor is {}. It refuses to act \
             below the floor, because a crowd of one is not a crowd.",
            state.deposits,
            state.k_floor
        ));
    }

    println!("rebuilding the accumulator so this note can be proved a member:");
    let history = history::scan(chain, program_id, &state.pool, true)?;
    let commitment = stored.commitment_bytes()?;
    let index = history.index_of(&commitment).ok_or_else(|| {
        anyhow!(
            "this note is not a leaf of the pool. Either it was never deposited, \
             or it belongs to a different denomination."
        )
    })?;
    let tree = history.tree()?;
    let root = tree.root().map_err(|e| anyhow!("{e:?}"))?;
    if root.to_bytes() != state.root {
        return Err(anyhow!(
            "the rebuilt root does not match the pool's, so a proof against it \
             would be rejected. Run `mirror tree` to see the mismatch."
        ));
    }
    println!(
        "  this note is leaf {index} of {}",
        history.commitments.len()
    );

    let binding = mirror_core::action_binding(
        action.selector(),
        &action.target(),
        &beneficiary.to_bytes(),
        relay_fee,
        action.accounts(),
        action.payload(),
    );
    let merkle_proof = tree.proof(index).map_err(|e| anyhow!("{e:?}"))?;
    let witness = mirror_circuit::Witness {
        note,
        merkle_proof: &merkle_proof,
        root,
        action_binding: binding,
    };

    println!("deriving the proving key from the published seed (this takes a moment)");
    let keys = crate::soak::keys()?;
    println!("proving membership");
    let mut rng = ark_std::rand::rngs::StdRng::from_entropy();
    let proof = mirror_circuit::prove(&keys, &witness, &mut rng).map_err(|e| anyhow!("{e:?}"))?;
    let nullifier = proof.public_inputs[1];
    let (spend_pda, _) = spend_address(program_id, &state.pool, &nullifier);

    let ix = Instruction::new_with_bytes(
        *program_id,
        &MirrorIx::SubmitSpend {
            proof_a: proof.proof_a,
            proof_b: proof.proof_b,
            proof_c: proof.proof_c,
            root: proof.public_inputs[0],
            nullifier,
            selector: action.selector(),
            target_program: action.target(),
            beneficiary: beneficiary.to_bytes(),
            relay_fee,
            action_accounts: action.accounts(),
            payload: action.payload().to_vec(),
        }
        .pack(),
        vec![
            AccountMeta::new(relay.pubkey(), true),
            AccountMeta::new(state.pool, false),
            AccountMeta::new(spend_pda, false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        ],
    );
    let sig = send(chain, ix, &[relay])?;

    println!();
    println!("submitted. The note is spent and the action is recorded.");
    println!("  nullifier {}", hex::encode(nullifier));
    println!("  record    {spend_pda}");
    println!("  signature {sig}");
    println!();
    println!("Nothing has been paid out yet — settlement executes the batch, which is");
    println!("what gives every member's action one timestamp. Run `mirror settle`, or");
    println!("wait for anyone else to.");
    Ok(())
}

// ---------------------------------------------------------------------------
// settle

pub fn settle(
    chain: &Chain,
    program_id: &Pubkey,
    denomination: u64,
    settler: &Keypair,
    now: i64,
) -> Result<()> {
    let state = read_pool(chain, program_id, denomination)?;
    println!("looking for spends waiting to settle:");
    let history = history::scan(chain, program_id, &state.pool, true)?;

    let mut ready: Vec<(Pubkey, Pubkey, Pubkey, i64)> = Vec::new();
    let mut skipped_actions: Vec<([u8; 32], u64, u8)> = Vec::new();
    for submitted in &history.spends {
        let Some(mut data) = chain.account_data(&submitted.spend)? else {
            continue;
        };
        let Ok(record) = Spend::load(&mut data) else {
            continue;
        };
        if record.status() != STATUS_PENDING {
            continue;
        }
        if record.selector() != SELECTOR_TRANSFER {
            // A CPI action needs the account list its callee expects, which this
            // command cannot infer from the chain — the record binds how many
            // accounts, never which. Settling those is a caller's job.
            skipped_actions.push((
                submitted.nullifier,
                record.selector(),
                record.action_accounts(),
            ));
            continue;
        }
        ready.push((
            submitted.spend,
            Pubkey::new_from_array(record.beneficiary()),
            Pubkey::new_from_array(record.relay()),
            record.submitted_at(),
        ));
    }

    if !skipped_actions.is_empty() {
        println!(
            "  {} pending action(s) left alone: a CPI needs the account list its callee",
            skipped_actions.len()
        );
        println!("  expects, and the record binds how many accounts, never which.");
        for (nullifier, selector, accounts) in &skipped_actions {
            println!(
                "    nullifier {} — selector {selector}, {accounts} account(s)",
                hex::encode(nullifier)
            );
        }
    }
    if ready.is_empty() {
        println!("  nothing to settle");
        return Ok(());
    }

    // The crowd rule, checked here so a caller learns why rather than reading a
    // custom error code off a failed transaction.
    let crowd = ready.len() as u32 >= state.k_floor;
    let timeout = mirror_pool_program::processor::SETTLE_TIMEOUT_SECONDS;
    if !crowd {
        let youngest = ready.iter().map(|(_, _, _, at)| *at).max().unwrap_or(0);
        let waited = now.saturating_sub(youngest);
        if waited < timeout {
            println!();
            println!(
                "  {} spend(s) pending, and this pool's floor is {}.",
                ready.len(),
                state.k_floor
            );
            println!(
                "  A batch below the floor may settle once every spend in it has waited {timeout}s;"
            );
            println!(
                "  the youngest has waited {waited}s, so {}s remain.",
                timeout - waited
            );
            println!();
            println!("  This is a liveness guarantee rather than a restriction: nobody can");
            println!("  hold a member's funds waiting for a crowd that never arrives.");
            return Ok(());
        }
    }

    let mut metas = vec![
        AccountMeta::new(settler.pubkey(), true),
        AccountMeta::new(state.pool, false),
        AccountMeta::new(state.vault, false),
    ];
    for (spend, beneficiary, relay, _) in &ready {
        metas.push(AccountMeta::new(*spend, false));
        metas.push(AccountMeta::new(*beneficiary, false));
        metas.push(AccountMeta::new(*relay, false));
    }
    let ix = Instruction::new_with_bytes(
        *program_id,
        &MirrorIx::SettleEpoch {
            count: ready.len() as u8,
        }
        .pack(),
        metas,
    );
    let sig = send(chain, ix, &[settler])?;
    println!();
    println!("settled {} spends in one transaction", ready.len());
    println!("  signature {sig}");
    println!();
    println!("Every payout in that batch shares one timestamp and one ordering, which");
    println!("is what stops arrival time from telling the members apart.");
    Ok(())
}

/// Creates a note and writes it, printing what to do next.
pub fn note_new(denomination: u64, path: &Path) -> Result<()> {
    let note = crate::note::generate(denomination)?;
    let stored = StoredNote::from_note(&note, denomination)?;
    stored.write(path)?;
    println!("wrote {}", path.display());
    println!("  denomination {denomination}");
    println!("  commitment   {}", stored.commitment);
    println!();
    println!("This file is the note. Treat it exactly as you would a keypair:");
    println!("anyone holding it can spend the deposit, and losing it loses the");
    println!("deposit with no way back.");
    println!();
    println!("Next: mirror deposit --note {}", path.display());
    Ok(())
}

/// The current wall clock, for the crowd-rule explanation.
pub fn now() -> Result<i64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .context("reading the clock")?
        .as_secs() as i64)
}
