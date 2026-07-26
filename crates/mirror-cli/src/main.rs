//! `mirror` — the operator and member CLI.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

mod chain;
mod soak;

/// The seed the committed verifying key was generated from.
///
/// Public on purpose. It makes the setup reproducible — anyone can re-derive the
/// deployed key and check it against the circuit in this repository — and it
/// makes the setup insecure in exactly the way a public seed implies: the toxic
/// waste is public, so proofs are forgeable. That trade is stated rather than
/// hidden, and the production path is a multi-party ceremony.
///
/// Reproducible-and-insecure is a coherent position for an unaudited tool. The
/// incoherent one is publishing the entropy *and* withholding the proving key,
/// which is insecure and unreproducible at once: the toxic waste is public, and
/// no third party can regenerate the key to produce a valid proof at all.
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
        /// Signature pages to walk before declaring an address high-activity.
        ///
        /// Raising it buys a better age estimate for busy funders, which is what
        /// lets the volume-hub rule classify them instead of leaving them in the
        /// budget bucket.
        #[arg(long, default_value_t = 20)]
        page_cap: u32,
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
    /// Compares the loss factor of two populations measured the same way.
    ///
    /// `ρ` is the headline because it is independent of `k` and therefore
    /// comparable across pools of different sizes. This is the command that
    /// exercises that property rather than asserting it.
    ///
    /// The comparison is a bootstrap over the difference, not a subtraction of
    /// two point estimates. Two samples drawn from the same underlying shape
    /// will differ by *something*, and reporting that something as a finding is
    /// the error this command exists to prevent: if the interval contains zero,
    /// it says so and declines to rank them.
    Compare {
        /// The population being examined.
        #[arg(long)]
        sample: PathBuf,
        /// The population it is measured against.
        #[arg(long)]
        against: PathBuf,
        /// A name for each, used only in the output.
        #[arg(long, default_value = "sample")]
        label: String,
        #[arg(long, default_value = "baseline")]
        against_label: String,
    },
    /// Runs the whole lifecycle against a live cluster and prints every
    /// signature, so the result is checkable rather than asserted.
    Soak {
        #[arg(long)]
        program: String,
        #[arg(long, default_value = "https://api.devnet.solana.com")]
        url: String,
        /// Payer and settler keypair.
        #[arg(long, default_value = "~/.config/solana/id.json")]
        keypair: String,
        /// Where to write the evidence.
        #[arg(long, default_value = "docs/PROOF.md")]
        out: PathBuf,
    },
    /// Recomputes the verifying key from a seed and reports its digest.
    ///
    /// This is the check a third party runs. It binds the *whole* key — alpha,
    /// beta, gamma, delta and every IC point — not just delta, so a transcript
    /// carrying a key belonging to a different circuit cannot pass. Verifying
    /// delta alone is the common shortcut and it would certify exactly that.
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

/// Loads a committed sample and returns one class label per resolved member.
///
/// Applies the same failure gate `analyze` does. A comparison drawn from a run
/// whose unresolved bucket is substantially our own infrastructure would be
/// comparing endpoints rather than populations, and it would do so invisibly —
/// the difference between two pools and the difference between two collection
/// runs look identical in the output.
fn resolved_labels(path: &std::path::Path) -> Result<(Vec<String>, u64)> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let sample = mirror_provenance::Sample::from_json(&text)?;
    let (results, census) = mirror_provenance::classify_sample(
        &sample,
        &mirror_provenance::AnchorSet::default(),
        &mirror_provenance::Thresholds::default(),
        sample.manifest.collected_at,
    );
    if !census.may_publish() {
        anyhow::bail!(
            "{}: the RPC failure rate is {:.2}%, above the 1% limit. This sample yields no \
             headline on its own and cannot be one side of a comparison.",
            path.display(),
            census.failure_rate() * 100.0
        );
    }
    let labels: Vec<String> = results
        .iter()
        .filter_map(|(_, o)| o.label().map(|s| s.to_string()))
        .collect();
    // Unresolved excluding our own RPC failures, which belong to neither side.
    let unresolved = census.measurable() - census.resolved;
    Ok((labels, unresolved))
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
            page_cap,
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
                sig_page_cap: page_cap,
                ..Default::default()
            };
            let thresholds = mirror_provenance::Thresholds::default();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs() as i64;

            let collector = mirror_provenance::Collector::new(&mut client, config);
            let started = std::time::Instant::now();
            let sample =
                collector.collect_with_progress(&seed_list, &check, &thresholds, now, |p| {
                    // Rate is the useful number here: it tells the operator
                    // whether the run is progressing or the endpoint has
                    // started throttling.
                    let elapsed = started.elapsed().as_secs_f64().max(0.001);
                    eprintln!(
                        "  [{:>3}/{}] {}… {} hops, {:?}  ({} calls, {:.1}/s, {:.0}s elapsed){}",
                        p.done,
                        p.total,
                        &p.seed[..p.seed.len().min(8)],
                        p.hops,
                        p.stop,
                        p.rpc_calls,
                        p.rpc_calls as f64 / elapsed,
                        elapsed,
                        p.error
                            .as_deref()
                            .map(|e| format!("\n        -> {}", &e[..e.len().min(160)]))
                            .unwrap_or_default(),
                    );
                });

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

            // Members that reached no class, excluding our own failures: those
            // belong to neither reading of the bracket.
            let unresolved = census.measurable() - census.resolved;

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

                    // Sampling error, which the bracket below does not cover.
                    // These depositors are a draw from a larger population, and
                    // without an interval over that draw a reader cannot tell a
                    // real difference between two pools from a lucky sample.
                    if let Some(i) = mirror_provenance::loss_factor_interval(
                        &labels,
                        mirror_provenance::bootstrap::DEFAULT_REPLICATES,
                        mirror_provenance::bootstrap::DEFAULT_SEED,
                    ) {
                        println!();
                        println!(
                            "rho under resampling   {:.4} .. {:.4}   (2.5-97.5%, {} replicates)",
                            i.lo, i.hi, i.replicates
                        );
                        println!(
                            "  resampling bias      {:+.4}   (mean {:.4} against a point estimate \
                             of {:.4})",
                            i.resampling_bias(),
                            i.mean,
                            i.point
                        );
                        if !i.contains_point() {
                            println!(
                                "  The range does not contain the point estimate, and that is a \
                                 property of this\n  population rather than an error. Resampling \
                                 leaves about 37% of members unpicked,\n  so single-member classes \
                                 vanish from most replicates; fewer classes means lower\n  H(C) \
                                 and therefore higher rho. The size of that gap is a tail \
                                 diagnostic."
                            );
                        }
                        println!(
                            "  This is the spread of the estimator, not its distance from the \
                             truth. Plug-in\n  entropy is biased low at small n, so rho is biased \
                             HIGH: the real loss factor is\n  plausibly below all of this, and \
                             equally so for any population measured this way."
                        );
                    }

                    // The bracket. A point estimate alone would not say whether
                    // the number is driven by what was measured or by what was
                    // not.
                    let mut sizes: std::collections::BTreeMap<&str, u64> =
                        std::collections::BTreeMap::new();
                    for l in &labels {
                        *sizes.entry(l.as_str()).or_insert(0) += 1;
                    }
                    let resolved_sizes: Vec<u64> = sizes.into_values().collect();
                    if let Some(b) = mirror_provenance::Bracket::new(&resolved_sizes, unresolved) {
                        println!();
                        println!(
                            "unresolved bracket ({} resolved, {} unresolved):",
                            b.resolved, b.unresolved
                        );
                        println!(
                            "  rho             {:.4} .. {:.4}",
                            b.lower.loss_factor, b.upper.loss_factor
                        );
                        println!(
                            "  effective-k     {:.4} .. {:.4}",
                            b.lower.eff_k_shannon, b.upper.eff_k_shannon
                        );
                        if !b.is_informative() {
                            println!();
                            println!(
                                "  NOT INFORMATIVE: fewer than half the members reached a class, so \
                                 the two readings\n  diverge and either one quoted alone would \
                                 describe the sampling budget rather than\n  the pool. The point \
                                 estimate above is reported for completeness, not as a result."
                            );
                        }
                    }

                    if a.good_turing_coverage < 0.8 {
                        println!();
                        println!(
                            "  UNDER-SAMPLED: Good-Turing coverage {:.2} and Chao1 estimates {:.0} \
                             classes against\n  {} observed, so most of the class distribution \
                             was never seen. Effective-k measured at\n  small k understates the \
                             steady-state loss and does not extrapolate upward.",
                            a.good_turing_coverage, a.chao1, a.classes
                        );
                    }
                }
            }
            Ok(())
        }
        Command::Compare {
            sample,
            against,
            label,
            against_label,
        } => {
            let (a, a_unresolved) = resolved_labels(&sample)?;
            let (b, b_unresolved) = resolved_labels(&against)?;

            let reps = mirror_provenance::bootstrap::DEFAULT_REPLICATES;
            let seed = mirror_provenance::bootstrap::DEFAULT_SEED;

            let ia = mirror_provenance::loss_factor_interval(&a, reps, seed)
                .ok_or_else(|| anyhow::anyhow!("{label}: no member resolved to a class"))?;
            let ib = mirror_provenance::loss_factor_interval(&b, reps, seed)
                .ok_or_else(|| anyhow::anyhow!("{against_label}: no member resolved to a class"))?;

            let width = label.len().max(against_label.len()).max(10);
            println!(
                "{:<width$}  members   rho      resampled 2.5-97.5%   bias",
                "population"
            );
            for (name, labels, i) in [(&label, &a, &ia), (&against_label, &b, &ib)] {
                println!(
                    "{name:<width$}  {:>7}   {:.4}   {:.4} .. {:.4}     {:+.4}",
                    labels.len(),
                    i.point,
                    i.lo,
                    i.hi,
                    i.resampling_bias()
                );
            }

            let d = mirror_provenance::difference_interval(&a, &b, reps, seed)
                .ok_or_else(|| anyhow::anyhow!("nothing to compare"))?;
            println!();
            println!(
                "difference ({label} − {against_label}): {:+.4}   95% {:+.4} .. {:+.4}",
                d.point, d.lo, d.hi
            );
            println!();

            // The same informativeness gate `analyze` applies to a single
            // headline, applied to each side. It matters *more* here, not less.
            //
            // Comparing two populations of which one is mostly unresolved is
            // comparing their traceable subsets, and traceability is not
            // independent of provenance class: a wallet funded by an exchange
            // resolves in one hop, and one funded through a chain of fresh
            // intermediaries exhausts the budget. So the unresolved members are
            // plausibly drawn from different classes than the resolved ones, and
            // a difference between the two subsets can be manufactured entirely
            // by that selection.
            let under = |labels: &[String], unresolved: u64| -> bool {
                let total = labels.len() as u64 + unresolved;
                total > 0 && (labels.len() as u64) * 2 < total
            };
            let a_under = under(&a, a_unresolved);
            let b_under = under(&b, b_unresolved);
            if a_under || b_under {
                let who = match (a_under, b_under) {
                    (true, true) => format!("{label} and {against_label} both resolve"),
                    (true, false) => format!("{label} resolves"),
                    _ => format!("{against_label} resolves"),
                };
                println!(
                    "REFUSING to rank these populations: {who} fewer than half its members.\n\n\
                     What is left after the unresolved are dropped is each population's\n\
                     *traceable* subset, and traceability is not independent of provenance\n\
                     class — a wallet funded straight from an exchange resolves in one hop,\n\
                     one funded through fresh intermediaries exhausts the budget. A difference\n\
                     between two such subsets can be produced entirely by that selection, and\n\
                     nothing in the numbers above would show it.\n\n\
                     The figures are printed for completeness, not as a comparison. Closing\n\
                     this needs resolution above half on both sides, which is a bigger\n\
                     traversal budget rather than a different metric."
                );
                return Ok(());
            }

            if d.excludes_zero() {
                let (more, less) = if d.point > 0.0 {
                    (&label, &against_label)
                } else {
                    (&against_label, &label)
                };
                println!(
                    "SEPARATED. The interval on the difference excludes zero, so at these sample\n\
                     sizes {more} is the more provenance-concentrated population of the two —\n\
                     a member's funding class narrows the guess further there than in {less}."
                );
            } else {
                println!(
                    "NOT SEPARATED. The interval on the difference contains zero, so these two\n\
                     populations are indistinguishable at these sample sizes. The point estimates\n\
                     differ, and that difference is not evidence: quoting it as one would be\n\
                     reporting the draw."
                );
            }
            println!();
            println!(
                "Both estimates share the same downward bias in entropy, so both rho values are\n\
                 biased high by roughly the same amount. That is why the comparison survives a\n\
                 bias that neither individual number does."
            );
            Ok(())
        }
        Command::Soak {
            program,
            url,
            keypair,
            out,
        } => soak::run(&program, &url, &keypair, &out),
        Command::VerifySetup { seed, expect } => {
            let keys = mirror_circuit::generate_reproducible(seed.as_bytes())
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let regenerated = keys.solana_vk();
            let digest = vk_digest(&regenerated);
            println!("vk sha256 (regenerated): {digest}");

            // Compare against the key actually compiled into the program, not
            // only against a digest the caller supplies. Printing a hash and
            // exiting zero verifies nothing; the point of a reproducible setup
            // is that a third party can confirm the *deployed* key is the one
            // this circuit and this seed produce.
            let baked = mirror_pool_program::vk::VERIFYING_KEY;
            let mut mismatches: Vec<&str> = Vec::new();
            if regenerated.alpha_g1 != baked.vk_alpha_g1 {
                mismatches.push("alpha_g1");
            }
            if regenerated.beta_g2 != baked.vk_beta_g2 {
                mismatches.push("beta_g2");
            }
            if regenerated.gamma_g2 != baked.vk_gamme_g2 {
                mismatches.push("gamma_g2");
            }
            if regenerated.delta_g2 != baked.vk_delta_g2 {
                mismatches.push("delta_g2");
            }
            if regenerated.ic.len() != baked.vk_ic.len()
                || regenerated.ic.iter().zip(baked.vk_ic).any(|(a, b)| a != b)
            {
                mismatches.push("ic");
            }

            if mismatches.is_empty() {
                println!(
                    "MATCH — every element of the program's verifying key is reproduced \
                     by this seed and this circuit ({} IC points checked)",
                    regenerated.ic.len()
                );
            } else {
                eprintln!(
                    "MISMATCH against the program's baked key: {}",
                    mismatches.join(", ")
                );
                eprintln!(
                    "The deployed program does not verify proofs from this circuit. \
                     A ceremony verifier that checked only delta would have passed this."
                );
                std::process::exit(1);
            }

            match expect {
                Some(want) if want.eq_ignore_ascii_case(&digest) => {
                    println!("and it matches the digest you supplied");
                    Ok(())
                }
                Some(want) => {
                    eprintln!("but the digest you supplied was {want}");
                    std::process::exit(1);
                }
                None => Ok(()),
            }
        }
    }
}
