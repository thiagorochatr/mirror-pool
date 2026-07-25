//! The whole protocol against the real compiled program.
//!
//! Every other test in this repository calls functions directly. This one loads
//! the `.so` that `make build-sbf` produces into a real SVM, sends real
//! transactions, and verifies a real Groth16 proof through the actual syscall.
//! It is the only test that can catch a divergence between what the host
//! believes and what the chain does.
//!
//! Requires `make build-sbf` first. If the artifact is missing the test says so
//! rather than silently passing, because a privacy test that quietly does
//! nothing is worse than no test.

use litesvm::LiteSVM;
use mirror_circuit::{generate_reproducible, prove, Keys, Witness};
use mirror_core::{Field, MerkleTree, Note};
use mirror_pool_program::{
    instruction::Instruction as MirrorIx,
    pda::{pool_address, spend_address, vault_address},
    spend::{Spend, SPEND_LEN, STATUS_PENDING},
    Pool, POOL_LEN,
};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use solana_signer::Signer;
use solana_transaction::Transaction;

const SEED: &[u8] = b"mirror-pool-reproducible-dev-setup-v1";
const DENOMINATION: u64 = 100_000_000; // 0.1 SOL
const ENTRY_FEE: u64 = 0;
const K_FLOOR: u32 = 4;
const RELAY_FEE: u64 = 1_000_000;
const SELECTOR: u64 = 1;

fn program_bytes() -> Vec<u8> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/sbf/deploy/mirror_pool_program.so"
    );
    std::fs::read(path).unwrap_or_else(|e| {
        panic!("could not read {path}: {e}\n\nRun `make build-sbf` first.");
    })
}

struct Env {
    svm: LiteSVM,
    program_id: Pubkey,
    payer: Keypair,
    pool: Pubkey,
    vault: Pubkey,
}

fn setup() -> Env {
    let mut svm = LiteSVM::new();
    let program_id = Pubkey::new_unique();
    svm.add_program(program_id, &program_bytes())
        .expect("loading the built program");

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 100_000_000_000).unwrap();

    let (pool, _) = pool_address(&program_id, DENOMINATION);
    let (vault, _) = vault_address(&program_id, &pool);

    Env {
        svm,
        program_id,
        payer,
        pool,
        vault,
    }
}

impl Env {
    fn send(&mut self, ix: Instruction, signer: &Keypair) -> Result<(), String> {
        let msg = Message::new(&[ix], Some(&signer.pubkey()));
        let tx = Transaction::new(&[signer], msg, self.svm.latest_blockhash());
        self.svm.send_transaction(tx).map(|_| ()).map_err(|e| {
            format!(
                "{:?} | logs: {:?}",
                e.err,
                e.meta.logs.iter().rev().take(4).collect::<Vec<_>>()
            )
        })
    }

    fn send_expect_cu(&mut self, ix: Instruction, signer: &Keypair) -> u64 {
        let msg = Message::new(&[ix], Some(&signer.pubkey()));
        let tx = Transaction::new(&[signer], msg, self.svm.latest_blockhash());
        let meta = self.svm.send_transaction(tx).unwrap_or_else(|e| {
            panic!("transaction failed: {:?}\nlogs: {:#?}", e.err, e.meta.logs)
        });
        meta.compute_units_consumed
    }

    fn init_pool(&mut self) {
        let ix = Instruction::new_with_bytes(
            self.program_id,
            &MirrorIx::InitPool {
                denomination: DENOMINATION,
                entry_fee: ENTRY_FEE,
                k_floor: K_FLOOR,
            }
            .pack(),
            vec![
                AccountMeta::new(self.payer.pubkey(), true),
                AccountMeta::new(self.pool, false),
                AccountMeta::new(self.vault, false),
                AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            ],
        );
        let payer = self.payer.insecure_clone();
        self.send(ix, &payer).expect("init_pool");
    }

    fn deposit(&mut self, commitment: Field, depositor: &Keypair) -> Result<(), String> {
        let ix = Instruction::new_with_bytes(
            self.program_id,
            &MirrorIx::Deposit {
                commitment: commitment.to_bytes(),
            }
            .pack(),
            vec![
                AccountMeta::new(depositor.pubkey(), true),
                AccountMeta::new(self.pool, false),
                AccountMeta::new(self.vault, false),
                AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            ],
        );
        self.send(ix, depositor)
    }

    fn pool_state(&self) -> Vec<u8> {
        self.svm.get_account(&self.pool).expect("pool exists").data
    }

    fn vault_lamports(&self) -> u64 {
        self.svm
            .get_account(&self.vault)
            .map(|a| a.lamports)
            .unwrap_or(0)
    }
}

/// Builds a pool with `count` deposits and returns the host-side tree plus the
/// notes, so a proof can be produced for any of them.
fn seeded_pool(count: u64) -> (Env, MerkleTree, Vec<Note>, Keys) {
    let mut env = setup();
    env.init_pool();

    let denom_tag = Field::from_u64(DENOMINATION);
    let mut tree = MerkleTree::new().unwrap();
    let mut notes = Vec::new();

    for i in 1..=count {
        let note = Note::new(
            Field::from_u64(i * 1_000_003),
            Field::from_u64(i * 7_919),
            denom_tag,
        );
        let depositor = Keypair::new();
        env.svm
            .airdrop(&depositor.pubkey(), DENOMINATION + 10_000_000)
            .unwrap();
        env.deposit(note.commitment().unwrap(), &depositor)
            .unwrap_or_else(|e| panic!("deposit {i} failed: {e}"));
        tree.insert(note.commitment().unwrap()).unwrap();
        notes.push(note);
    }

    let keys = generate_reproducible(SEED).expect("setup");
    (env, tree, notes, keys)
}

fn spend_ix(
    env: &Env,
    proof: &mirror_circuit::SolanaProof,
    nullifier: [u8; 32],
    beneficiary: &Pubkey,
    relay: &Pubkey,
) -> Instruction {
    let (spend_pda, _) = spend_address(&env.program_id, &env.pool, &nullifier);
    Instruction::new_with_bytes(
        env.program_id,
        &MirrorIx::SubmitSpend {
            proof_a: proof.proof_a,
            proof_b: proof.proof_b,
            proof_c: proof.proof_c,
            root: proof.public_inputs[0],
            nullifier,
            selector: SELECTOR,
            beneficiary: beneficiary.to_bytes(),
            relay_fee: RELAY_FEE,
        }
        .pack(),
        vec![
            AccountMeta::new(*relay, true),
            AccountMeta::new(env.pool, false),
            AccountMeta::new(spend_pda, false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        ],
    )
}

/// Produces a proof for note `index`, bound to `beneficiary`.
fn proof_for(
    keys: &Keys,
    tree: &MerkleTree,
    notes: &[Note],
    index: usize,
    beneficiary: &Pubkey,
) -> mirror_circuit::SolanaProof {
    use ark_std::rand::SeedableRng;
    let merkle_proof = tree.proof(index as u64).unwrap();
    let binding =
        mirror_core::action_binding(SELECTOR, &beneficiary.to_bytes(), RELAY_FEE).unwrap();
    let witness = Witness {
        note: notes[index],
        merkle_proof: &merkle_proof,
        root: tree.root().unwrap(),
        action_binding: binding,
    };
    let mut rng = ark_std::rand::rngs::StdRng::from_seed([index as u8; 32]);
    prove(keys, &witness, &mut rng).expect("proving")
}

#[test]
fn a_pool_initialises_with_an_empty_accumulator() {
    let mut env = setup();
    env.init_pool();

    let mut data = env.pool_state();
    assert_eq!(data.len(), POOL_LEN);
    let pool = Pool::load(&mut data).unwrap();
    assert_eq!(pool.denomination(), DENOMINATION);
    assert_eq!(pool.k_floor(), K_FLOOR);
    assert_eq!(pool.deposit_count(), 0);
    assert_eq!(
        pool.current_root().unwrap(),
        mirror_core::Frontier::new().unwrap().root(),
        "the deployed program and the host must start from one accumulator"
    );
}

#[test]
fn deposits_escrow_exactly_the_denomination_and_advance_the_root() {
    let (env, tree, _notes, _keys) = seeded_pool(4);

    let mut data = env.pool_state();
    let pool = Pool::load(&mut data).unwrap();
    assert_eq!(pool.deposit_count(), 4);
    assert_eq!(
        pool.current_root().unwrap(),
        tree.root().unwrap(),
        "the on-chain root diverged from the host tree"
    );

    // Escrow, plus the vault's own rent exemption.
    assert!(
        env.vault_lamports() >= 4 * DENOMINATION,
        "vault holds {} for 4 notes of {DENOMINATION}",
        env.vault_lamports()
    );
    assert_eq!(pool.required_vault_lamports().unwrap(), 4 * DENOMINATION);
}

/// The decisive test: a proof built by the host prover, verified by the real
/// Groth16 syscall inside the deployed program.
#[test]
fn a_real_proof_verifies_on_chain_and_records_the_spend() {
    let (mut env, tree, notes, keys) = seeded_pool(5);
    let beneficiary = Pubkey::new_unique();
    let relay = Keypair::new();
    env.svm.airdrop(&relay.pubkey(), 10_000_000_000).unwrap();

    let proof = proof_for(&keys, &tree, &notes, 2, &beneficiary);
    let nullifier = proof.public_inputs[1];

    let ix = spend_ix(&env, &proof, nullifier, &beneficiary, &relay.pubkey());
    let cu = env.send_expect_cu(ix, &relay);
    println!("submit_spend consumed {cu} compute units");
    assert!(
        cu < 200_000,
        "submit_spend used {cu} CU, above the default per-instruction budget"
    );

    let (spend_pda, _) = spend_address(&env.program_id, &env.pool, &nullifier);
    let mut data = env
        .svm
        .get_account(&spend_pda)
        .expect("spend recorded")
        .data;
    assert_eq!(data.len(), SPEND_LEN);
    let record = Spend::load(&mut data).unwrap();
    assert_eq!(record.status(), STATUS_PENDING);
    assert_eq!(record.selector(), SELECTOR);
    assert_eq!(record.relay_fee(), RELAY_FEE);
    assert_eq!(record.beneficiary(), beneficiary.to_bytes());
    assert_eq!(
        record.relay(),
        relay.pubkey().to_bytes(),
        "the relay that submitted must be the one paid at settlement"
    );

    // No member key appears anywhere in this path.
    assert_ne!(relay.pubkey(), beneficiary);
}

#[test]
fn the_same_note_cannot_be_spent_twice() {
    let (mut env, tree, notes, keys) = seeded_pool(5);
    let beneficiary = Pubkey::new_unique();
    let first_relay = Keypair::new();
    let second_relay = Keypair::new();
    env.svm
        .airdrop(&first_relay.pubkey(), 10_000_000_000)
        .unwrap();
    env.svm
        .airdrop(&second_relay.pubkey(), 10_000_000_000)
        .unwrap();

    let proof = proof_for(&keys, &tree, &notes, 1, &beneficiary);
    let nullifier = proof.public_inputs[1];

    let ix = spend_ix(&env, &proof, nullifier, &beneficiary, &first_relay.pubkey());
    env.send(ix, &first_relay).expect("first spend");

    // The replay goes through a *different* relay, so the transaction is not
    // byte-identical and the runtime's duplicate-signature check cannot be what
    // rejects it. The relay identity is not part of the action binding, so the
    // proof itself is still entirely valid — the only thing standing in the way
    // is the nullifier record. An earlier version of this test reused the first
    // relay and passed for the wrong reason: it was rejected as AlreadyProcessed
    // before the program ran at all.
    let replay = spend_ix(
        &env,
        &proof,
        nullifier,
        &beneficiary,
        &second_relay.pubkey(),
    );
    let err = env
        .send(replay, &second_relay)
        .expect_err("a replayed proof was accepted");
    println!("replay rejected: {err}");
    assert!(
        !err.contains("AlreadyProcessed"),
        "the runtime deduplicated the transaction; the nullifier guard was never reached: {err}"
    );
    assert!(
        err.contains("Custom(15)"),
        "expected NullifierAlreadySpent (15), got: {err}"
    );
}

#[test]
fn a_relay_cannot_redirect_the_payout() {
    let (mut env, tree, notes, keys) = seeded_pool(5);
    let honest_beneficiary = Pubkey::new_unique();
    let relay = Keypair::new();
    env.svm.airdrop(&relay.pubkey(), 10_000_000_000).unwrap();

    let proof = proof_for(&keys, &tree, &notes, 0, &honest_beneficiary);
    let nullifier = proof.public_inputs[1];

    // The relay swaps in its own address after the member proved.
    let thief = Pubkey::new_unique();
    let ix = spend_ix(&env, &proof, nullifier, &thief, &relay.pubkey());
    let err = env
        .send(ix, &relay)
        .expect_err("a redirected payout was accepted");
    println!("redirect rejected: {err}");
}

#[test]
fn a_relay_cannot_inflate_its_own_fee() {
    let (mut env, tree, notes, keys) = seeded_pool(5);
    let beneficiary = Pubkey::new_unique();
    let relay = Keypair::new();
    env.svm.airdrop(&relay.pubkey(), 10_000_000_000).unwrap();

    let proof = proof_for(&keys, &tree, &notes, 3, &beneficiary);
    let nullifier = proof.public_inputs[1];
    let (spend_pda, _) = spend_address(&env.program_id, &env.pool, &nullifier);

    let ix = Instruction::new_with_bytes(
        env.program_id,
        &MirrorIx::SubmitSpend {
            proof_a: proof.proof_a,
            proof_b: proof.proof_b,
            proof_c: proof.proof_c,
            root: proof.public_inputs[0],
            nullifier,
            selector: SELECTOR,
            beneficiary: beneficiary.to_bytes(),
            relay_fee: RELAY_FEE * 50, // proved for RELAY_FEE
        }
        .pack(),
        vec![
            AccountMeta::new(relay.pubkey(), true),
            AccountMeta::new(env.pool, false),
            AccountMeta::new(spend_pda, false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        ],
    );
    let err = env
        .send(ix, &relay)
        .expect_err("an inflated relay fee was accepted");
    println!("fee inflation rejected: {err}");
}

#[test]
fn a_proof_against_an_unknown_root_is_rejected() {
    let (mut env, tree, notes, keys) = seeded_pool(5);
    let beneficiary = Pubkey::new_unique();
    let relay = Keypair::new();
    env.svm.airdrop(&relay.pubkey(), 10_000_000_000).unwrap();

    let mut proof = proof_for(&keys, &tree, &notes, 4, &beneficiary);
    let nullifier = proof.public_inputs[1];
    // A root the pool has never held. Flip a low bit so the value stays a
    // canonical scalar and the rejection comes from the history check.
    proof.public_inputs[0][31] ^= 1;

    let ix = spend_ix(&env, &proof, nullifier, &beneficiary, &relay.pubkey());
    let err = env
        .send(ix, &relay)
        .expect_err("an unknown root was accepted");
    println!("unknown root rejected: {err}");
}

#[test]
fn a_pool_below_its_anonymity_floor_refuses_to_act() {
    // Three deposits against a floor of four: acting here would give the member
    // an anonymity set smaller than the pool promises.
    let (mut env, tree, notes, keys) = seeded_pool(3);
    let beneficiary = Pubkey::new_unique();
    let relay = Keypair::new();
    env.svm.airdrop(&relay.pubkey(), 10_000_000_000).unwrap();

    let proof = proof_for(&keys, &tree, &notes, 0, &beneficiary);
    let nullifier = proof.public_inputs[1];
    let ix = spend_ix(&env, &proof, nullifier, &beneficiary, &relay.pubkey());
    let err = env
        .send(ix, &relay)
        .expect_err("a spend below the anonymity floor was accepted");
    println!("below-floor rejected: {err}");
}

#[test]
fn a_forged_proof_is_rejected() {
    let (mut env, tree, notes, keys) = seeded_pool(5);
    let beneficiary = Pubkey::new_unique();
    let relay = Keypair::new();
    env.svm.airdrop(&relay.pubkey(), 10_000_000_000).unwrap();

    let mut proof = proof_for(&keys, &tree, &notes, 2, &beneficiary);
    let nullifier = proof.public_inputs[1];
    // Corrupt the proof itself.
    proof.proof_a[63] ^= 1;

    let ix = spend_ix(&env, &proof, nullifier, &beneficiary, &relay.pubkey());
    let err = env
        .send(ix, &relay)
        .expect_err("a corrupted proof was accepted");
    println!("forged proof rejected: {err}");
}
