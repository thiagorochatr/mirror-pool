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
    /// Builds a member-weighted frame from a pool's depositors.
    ///
    /// Each depositor appears once however often they transact, and the sample
    /// is spread across the pool's whole signature history rather than its most
    /// recent minute. Nothing is dropped for looking hard to trace.
    Seeds {
        /// The pool program to enumerate depositors of.
        #[arg(long)]
        program: String,
        /// How many distinct depositors to collect.
        #[arg(long, default_value_t = 40)]
        n: usize,
        /// Signature pages to walk back through. More pages means a sample
        /// spread over more of the pool's lifetime.
        #[arg(long, default_value_t = 8)]
        pages: u32,
        /// Transactions to fetch in total, spread across the pages.
        ///
        /// Sized from the measured deposit rate rather than guessed: about one
        /// program transaction in eight is a deposit, so reaching N depositors
        /// costs roughly 8N fetches.
        #[arg(long, default_value_t = 400)]
        scan: usize,
        /// Minimum lamports a payer must part with for it to count as a deposit.
        #[arg(long, default_value_t = 10_000_000)]
        min_deposit: u64,
        #[arg(long, default_value = "seeds.txt")]
        out: PathBuf,
        #[arg(long, default_value = "https://api.mainnet-beta.solana.com")]
        endpoint: String,
        #[arg(long, default_value_t = 0.4)]
        rps: f64,
    },
    /// Pass one: walks each seed's funding chain and writes a sample file.
    ///
    /// The only step that touches the network. Everything it observes goes into
    /// the sample, so the number can be recomputed later by anyone holding it.
    Collect {
        /// File with one seed address per line.
        #[arg(long)]
        seeds: PathBuf,
        #[arg(long, default_value = "sample.json")]
        out: PathBuf,
        #[arg(long, default_value = "https://api.mainnet-beta.solana.com")]
        endpoint: String,
        #[arg(long, default_value_t = 0.4)]
        rps: f64,
        #[arg(long, default_value_t = 6)]
        depth: u32,
    },
    /// Pass two: classifies a committed sample and reports the anonymity ladder.
    ///
    /// Pure and offline. Given the same sample it always produces the same
    /// numbers, which is what makes a published result checkable by someone who
    /// does not trust our endpoint or our run.
    Analyze {
        #[arg(long, default_value = "sample.json")]
        sample: PathBuf,
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
        Command::Seeds {
            program,
            n,
            pages,
            scan,
            min_deposit,
            out,
            endpoint,
            rps,
        } => {
            let mut client = mirror_provenance::RpcClient::new(&endpoint, rps);
            let check = client
                .check_preconditions()
                .map_err(|e| anyhow::anyhow!("endpoint refused: {e}"))?;
            eprintln!(
                "endpoint ok (first available block {})",
                check.first_available_block
            );

            let mut seeds: Vec<String> = Vec::new();
            let mut seen = std::collections::BTreeSet::new();
            let mut before: Option<String> = None;
            let mut scanned = 0usize;
            let mut slots: Vec<u64> = Vec::new();

            'paging: for page in 0..pages {
                let batch = client
                    .signatures_for_address(&program, before.as_deref(), 1_000)
                    .map_err(|e| anyhow::anyhow!("listing program signatures: {e}"))?;
                if batch.is_empty() {
                    break;
                }
                before = Some(batch.last().expect("non-empty").signature.clone());
                eprintln!(
                    "page {page}: {} signatures, {} depositors so far",
                    batch.len(),
                    seeds.len()
                );

                // Spread the scan budget evenly over the pages, and stride
                // within each page so the sample spans its time range instead of
                // clustering at its head.
                let scan_this_page = (scan / pages as usize).max(1);
                let stride = (batch.len() / scan_this_page).max(1);
                for info in batch.iter().step_by(stride) {
                    if info.err {
                        continue;
                    }
                    scanned += 1;
                    let tx = match client.transaction(&info.signature) {
                        Ok(t) => t,
                        // A failure here drops one candidate; it never becomes a
                        // claim about the pool.
                        Err(_) => continue,
                    };
                    if let Some(d) = mirror_provenance::depositor_of(&tx, min_deposit) {
                        if seen.insert(d.clone()) {
                            slots.push(tx.slot);
                            seeds.push(d);
                            if seeds.len() >= n {
                                break 'paging;
                            }
                        }
                    }
                }
            }

            anyhow::ensure!(!seeds.is_empty(), "no depositors found for {program}");
            std::fs::write(&out, format!("{}\n", seeds.join("\n")))
                .with_context(|| format!("writing {}", out.display()))?;

            let span = match (slots.iter().min(), slots.iter().max()) {
                (Some(lo), Some(hi)) => hi - lo,
                _ => 0,
            };
            println!(
                "wrote {} ({} distinct depositors)",
                out.display(),
                seeds.len()
            );
            println!(
                "scanned {scanned} transactions, {} rpc calls",
                client.calls_made()
            );
            println!(
                "sample spans {span} slots (~{:.1} hours of chain time)",
                span as f64 * 0.4 / 3600.0
            );
            Ok(())
        }
        Command::Collect {
            seeds,
            out,
            endpoint,
            rps,
            depth,
        } => {
            let text = std::fs::read_to_string(&seeds)
                .with_context(|| format!("reading {}", seeds.display()))?;
            let seed_list: Vec<String> = text
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .collect();
            anyhow::ensure!(!seed_list.is_empty(), "no seeds in {}", seeds.display());

            let mut client = mirror_provenance::RpcClient::new(&endpoint, rps);
            let check = client
                .check_preconditions()
                .map_err(|e| anyhow::anyhow!("endpoint refused: {e}"))?;
            eprintln!(
                "endpoint ok (first available block {}), collecting {} seeds",
                check.first_available_block,
                seed_list.len()
            );

            let config = mirror_provenance::CollectionConfig {
                depth_max: depth,
                ..Default::default()
            };
            let thresholds = mirror_provenance::Thresholds::default();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs() as i64;

            let collector = mirror_provenance::Collector::new(&mut client, config);
            let sample = collector.collect(&seed_list, &check, &thresholds, now);

            std::fs::write(&out, sample.to_json()?)
                .with_context(|| format!("writing {}", out.display()))?;
            println!(
                "wrote {} ({} rpc calls)",
                out.display(),
                sample.manifest.rpc_calls
            );
            if !sample.manifest.excluded_non_wallet.is_empty() {
                println!(
                    "excluded {} seeds that are not wallets (program, PDA, token account, \
                     or closed): a definitional frame criterion, not a difficulty one",
                    sample.manifest.excluded_non_wallet.len()
                );
            }
            println!(
                "ambiguous attribution rate: {:.1}%",
                sample.manifest.ambiguous_attribution_rate * 100.0
            );
            Ok(())
        }
        Command::Analyze { sample } => {
            let text = std::fs::read_to_string(&sample)
                .with_context(|| format!("reading {}", sample.display()))?;
            let sample = mirror_provenance::Sample::from_json(&text)?;
            let thresholds = mirror_provenance::Thresholds::default();
            let anchors = mirror_provenance::AnchorSet::default();

            let (results, census) = mirror_provenance::classify_sample(
                &sample,
                &anchors,
                &thresholds,
                sample.manifest.collected_at,
            );

            println!("endpoint  {}", sample.manifest.endpoint);
            println!("scope     {:?}", sample.manifest.config.scope);
            println!("census    {}", census.summary());
            println!(
                "frame     {} seeds excluded as non-wallets before tracing",
                sample.manifest.excluded_non_wallet.len()
            );
            println!();

            if !census.may_publish() {
                eprintln!(
                    "REFUSING to report a headline: the RPC failure rate is {:.2}%, above the \
                     1% limit. Above that threshold the unresolved bucket is substantially our \
                     own making and any effective-k computed from it would be measuring our \
                     infrastructure rather than the pool.",
                    census.failure_rate() * 100.0
                );
                std::process::exit(3);
            }

            let labels: Vec<String> = results
                .iter()
                .filter_map(|(_, o)| o.label().map(|s| s.to_string()))
                .collect();
            match mirror_provenance::Anonymity::from_labels(&labels) {
                None => {
                    println!("no member resolved to a class; nothing to report");
                }
                Some(a) => {
                    println!("resolved members     {}", a.nominal_k);
                    println!("provenance classes   {}", a.classes);
                    println!();
                    println!(
                        "loss factor rho      {:.4}   <- headline, independent of k",
                        a.loss_factor
                    );
                    println!("effective-k Shannon  {:.4}", a.eff_k_shannon);
                    println!("effective-k min-ent  {:.4}", a.eff_k_min_entropy);
                    println!("leakage Shannon      {:.4} bits", a.leakage_shannon_bits);
                    println!(
                        "leakage min-entropy  {:.4} bits",
                        a.leakage_min_entropy_bits
                    );
                    println!("guessing entropy     {:.2}", a.guessing_entropy);
                    println!("Good-Turing coverage {:.4}", a.good_turing_coverage);
                    println!("Chao1 richness       {:.2}", a.chao1);
                    println!(
                        "worst-case class     {}{}",
                        a.worst_case,
                        if a.worst_case_is_informative() {
                            ""
                        } else {
                            "   (not informative: under any heavy-tailed prior somebody is always alone)"
                        }
                    );
                    println!();
                    println!("class-size CCDF (share of members in a class of at most t):");
                    for (t, share) in &a.class_size_ccdf {
                        println!("  t={t:<4} {:.4}", share);
                    }
                }
            }
            Ok(())
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
