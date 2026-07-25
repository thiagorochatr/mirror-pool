//! `mirror` — the operator and member CLI.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

/// The seed the committed verifying key was generated from.
///
/// Public on purpose. It makes the setup reproducible — anyone can re-derive the
/// deployed key and check it against the circuit in this repository — and it
/// makes the setup insecure in exactly the way a public seed implies: the toxic
/// waste is public, so proofs are forgeable. That trade is stated rather than
/// hidden, and the production path is a multi-party ceremony.
///
/// A competing submission publishes its entropy string *and* gitignores the
/// proving key, so its setup is insecure and unreproducible at the same time —
/// no third party can produce a valid proof for its deployed program at all.
pub const DEV_SETUP_SEED: &str = "mirror-pool-reproducible-dev-setup-v1";

#[derive(Parser)]
#[command(
    name = "mirror",
    about = "mirror-pool: a behavioral anonymity set on Solana",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generates the Groth16 keys and writes the verifying key as Rust source
    /// for the on-chain program.
    Setup {
        #[arg(long, default_value = DEV_SETUP_SEED)]
        seed: String,
        #[arg(long, default_value = "programs/mirror-pool/src/vk.rs")]
        out: PathBuf,
    },
    /// Checks that an RPC endpoint can actually serve the history a provenance
    /// measurement depends on, and refuses it otherwise.
    ///
    /// Run this before trusting any number a collection produces. A truncated
    /// endpoint does not error on old data — it returns nothing — so a collector
    /// pointed at one reports every old funding event as absent and produces a
    /// graph that looks like a finding.
    CheckEndpoint {
        #[arg(long, default_value = "https://api.mainnet-beta.solana.com")]
        endpoint: String,
        /// Requests per second. The public endpoint sustains a measured
        /// 0.28-0.55, far below its documented allowance.
        #[arg(long, default_value_t = 0.4)]
        rps: f64,
    },
    /// Recomputes the verifying key from a seed and reports its digest.
    ///
    /// This is the check a third party runs. It binds the *whole* key — alpha,
    /// beta, gamma, delta and every IC point — not just delta, so a transcript
    /// carrying a key belonging to a different circuit cannot pass. A competing
    /// submission's ceremony verifier checks delta alone and would certify one.
    VerifySetup {
        #[arg(long, default_value = DEV_SETUP_SEED)]
        seed: String,
        /// Expected digest, as printed by `setup`.
        #[arg(long)]
        expect: Option<String>,
    },
}

fn vk_digest(vk: &mirror_circuit::SolanaVerifyingKey) -> String {
    let mut hasher = Sha256::new();
    hasher.update(vk.digest_preimage());
    hex::encode(hasher.finalize())
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Setup { seed, out } => {
            eprintln!("generating keys from seed {seed:?} (this takes a moment)");
            let keys = mirror_circuit::generate_reproducible(seed.as_bytes())
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let svk = keys.solana_vk();
            let source = mirror_circuit::vk_to_rust_source(&svk);
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::write(&out, source).with_context(|| format!("writing {}", out.display()))?;
            println!("wrote {}", out.display());
            println!("public inputs: {}", svk.public_input_count());
            println!("vk sha256:     {}", vk_digest(&svk));
            Ok(())
        }
        Command::CheckEndpoint { endpoint, rps } => {
            let mut client = mirror_provenance::RpcClient::new(&endpoint, rps);
            match client.check_preconditions() {
                Ok(check) => {
                    println!("endpoint             {}", check.endpoint);
                    println!("first available block {}", check.first_available_block);
                    println!(
                        "archival probe slot   {} ok",
                        mirror_provenance::rpc::ARCHIVAL_PROBE_SLOT
                    );
                    println!("\nUSABLE — this endpoint serves history to genesis.");
                    Ok(())
                }
                Err(e) => {
                    eprintln!("REFUSED: {e}");
                    eprintln!(
                        "\nA measurement taken here would report old funding events as \
                         absent and the resulting unresolved bucket would be an artifact."
                    );
                    std::process::exit(2);
                }
            }
        }
        Command::VerifySetup { seed, expect } => {
            let keys = mirror_circuit::generate_reproducible(seed.as_bytes())
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let digest = vk_digest(&keys.solana_vk());
            println!("vk sha256: {digest}");
            match expect {
                Some(want) if want.eq_ignore_ascii_case(&digest) => {
                    println!("MATCH — this seed and this circuit produce that key");
                    Ok(())
                }
                Some(want) => {
                    eprintln!("MISMATCH — expected {want}");
                    std::process::exit(1);
                }
                None => Ok(()),
            }
        }
    }
}
