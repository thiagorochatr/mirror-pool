//! Anonymity metrics over a provenance partition.
//!
//! An anonymity set of `K` members is partitioned into classes by where each
//! member's capital came from. An adversary who learns a member's class is left
//! guessing only within it, so what survives is not `K` but the residual
//! anonymity `H(X|C)`.
//!
//! The whole module rests on one identity, which `the_chain_rule_holds` pins:
//!
//! ```text
//! log2 K = H(C) + H(X|C)
//! ```
//!
//! **The folklore formula is inverted.** `2^{H(C)}` — entropy over the
//! class-size *distribution* — is widely quoted as the effective anonymity set.
//! It is the leakage: it is maximised when every member is alone, which is total
//! deanonymisation. The anonymity is `2^{H(X|C)}`. Getting this backwards
//! produces a number that improves as privacy gets worse.
//!
//! ## Attribution
//!
//! `H` is the Serjantov–Danezis entropy metric (PET 2002, Definition 2), which
//! is defined in *bits* over *users*. The exponentiated form `2^H` as a member
//! count is Andersson & Lundin (IFIP AICT 262, 2008, Definition 1), who
//! explicitly dispute the common misattribution. We do not write "Serjantov and
//! Danezis define the effective anonymity set size as `2^H`", because they did
//! not.
//!
//! For each observation `C = c` the adversary's posterior is uniform on class
//! `c`, whose Serjantov–Danezis effective size is `log2 n_c` bits. `H(X|C)` is
//! therefore the expectation of that size over the adversary's observation.

use std::collections::BTreeMap;

/// Every quantity we report. Never a scalar — a single number hides the tail,
/// and the tail is where the people with no anonymity are.
#[derive(Debug, Clone, PartialEq)]
pub struct Anonymity {
    /// Members in the set.
    pub nominal_k: u64,
    /// Distinct provenance classes.
    pub classes: u64,
    /// `ρ = 2^{−H(C)}`, the fraction of nominal k that survives.
    ///
    /// The headline, because it is independent of `k` and therefore comparable
    /// across pools of different sizes. Effective-k measured at small `k`
    /// systematically understates the steady-state loss and cannot be
    /// extrapolated upward.
    pub loss_factor: f64,
    /// `2^{H(X|C)}`: the membership-weighted geometric mean of class sizes.
    pub eff_k_shannon: f64,
    /// `H(C)` in bits. Exactly the Shannon leakage `I(X;C)`.
    pub leakage_shannon_bits: f64,
    /// `K/m`: Bayes vulnerability, the unweighted arithmetic mean of class sizes.
    pub eff_k_min_entropy: f64,
    /// `log2 m` in bits. Exactly the min-entropy leakage.
    pub leakage_min_entropy_bits: f64,
    /// Smallest class. Meaningless without its null distribution — see
    /// [`Anonymity::worst_case_is_informative`].
    pub worst_case: u64,
    /// Expected number of guesses, `G(X|C)`. Uniform baseline is `(K+1)/2`.
    pub guessing_entropy: f64,
    /// Good–Turing coverage `1 − f₁/K`: the fraction of class mass observed.
    pub good_turing_coverage: f64,
    /// Chao1 richness estimate. A saturation diagnostic, not a class count.
    pub chao1: f64,
    /// `P(my class ≤ t)` at `t ∈ {1,2,4,8,16,32,64,128}`.
    pub class_size_ccdf: Vec<(u64, f64)>,
}

impl Anonymity {
    /// Computes every metric from the class sizes of a partition.
    ///
    /// Returns `None` for an empty partition or one containing an empty class —
    /// both mean the caller's partitioning is wrong, and silently coping would
    /// hide that.
    pub fn from_class_sizes(sizes: &[u64]) -> Option<Self> {
        if sizes.is_empty() || sizes.contains(&0) {
            return None;
        }
        let k: u64 = sizes.iter().sum();
        let m = sizes.len() as u64;
        let kf = k as f64;

        // H(C) = -Σ p_c log2 p_c  and  H(X|C) = Σ p_c log2 n_c
        let mut h_c = 0.0f64;
        let mut h_x_given_c = 0.0f64;
        for &n in sizes {
            let p = n as f64 / kf;
            h_c -= p * p.log2();
            h_x_given_c += p * (n as f64).log2();
        }

        let f1 = sizes.iter().filter(|&&n| n == 1).count() as f64;
        let f2 = sizes.iter().filter(|&&n| n == 2).count() as f64;

        // G(X|C) = (1/K) Σ n_c(n_c+1)/2
        let guessing_entropy = sizes
            .iter()
            .map(|&n| {
                let nf = n as f64;
                nf * (nf + 1.0) / 2.0
            })
            .sum::<f64>()
            / kf;

        let mut ccdf = Vec::new();
        for t in [1u64, 2, 4, 8, 16, 32, 64, 128] {
            let members_in_small_classes: u64 = sizes.iter().filter(|&&n| n <= t).sum();
            ccdf.push((t, members_in_small_classes as f64 / kf));
        }

        Some(Anonymity {
            nominal_k: k,
            classes: m,
            loss_factor: (-h_c).exp2(),
            eff_k_shannon: h_x_given_c.exp2(),
            leakage_shannon_bits: h_c,
            eff_k_min_entropy: kf / m as f64,
            leakage_min_entropy_bits: (m as f64).log2(),
            worst_case: *sizes.iter().min().expect("non-empty"),
            guessing_entropy,
            good_turing_coverage: 1.0 - f1 / kf,
            chao1: m as f64 + f1 * (f1 - 1.0) / (2.0 * (f2 + 1.0)),
            class_size_ccdf: ccdf,
        })
    }

    /// Groups members by class label and measures the resulting partition.
    pub fn from_labels<L: Ord + Clone>(labels: &[L]) -> Option<Self> {
        let mut counts: BTreeMap<L, u64> = BTreeMap::new();
        for l in labels {
            *counts.entry(l.clone()).or_insert(0) += 1;
        }
        let sizes: Vec<u64> = counts.into_values().collect();
        Self::from_class_sizes(&sizes)
    }

    /// Whether reporting `worst_case` says anything about *this* pool.
    ///
    /// Under any heavy-tailed provenance prior somebody is always alone: in
    /// simulation `P(min_c n_c = 1) ≈ 1.00` for every `k ≤ 512`. So "worst case
    /// is 1" describes the shape of provenance in general, not a property of the
    /// pool being measured, and quoting it as a finding — as a published
    /// measurement of a live Solana pool does — is not informative.
    ///
    /// It becomes informative only when the smallest class is larger than one,
    /// which is a genuine and unusual claim.
    pub fn worst_case_is_informative(&self) -> bool {
        self.worst_case > 1
    }

    /// The ordering that must hold for any partition.
    ///
    /// `min_c n_c ≤ K/m ≤ 2^{H(X|C)} ≤ K`
    ///
    /// Worst case is at most the arithmetic mean of class sizes, which is at
    /// most the membership-weighted geometric mean, which is at most nominal.
    /// A violation means an arithmetic error, so it is asserted rather than
    /// assumed.
    pub fn ordering_holds(&self) -> bool {
        const EPS: f64 = 1e-9;
        let worst = self.worst_case as f64;
        let nominal = self.nominal_k as f64;
        worst <= self.eff_k_min_entropy + EPS
            && self.eff_k_min_entropy <= self.eff_k_shannon + EPS
            && self.eff_k_shannon <= nominal + EPS
    }

    /// `H(C) + H(X|C) == log2 K`, the identity the module rests on.
    pub fn chain_rule_residual(&self) -> f64 {
        let h_x_given_c = self.eff_k_shannon.log2();
        (self.leakage_shannon_bits + h_x_given_c) - (self.nominal_k as f64).log2()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    /// Reference vector A from the methodology, computed independently at full
    /// double precision. Asserted to 1e-9 so an arithmetic slip cannot hide.
    #[test]
    fn reference_vector_a() {
        let a = Anonymity::from_class_sizes(&[50, 20, 10, 10, 5, 3, 1, 1]).unwrap();
        assert_eq!(a.nominal_k, 100);
        assert_eq!(a.classes, 8);
        assert!(
            close(a.leakage_shannon_bits, 2.1295115772, 1e-9),
            "H(C) = {}",
            a.leakage_shannon_bits
        );
        assert!(
            close(a.eff_k_shannon, 22.8535219812, 1e-9),
            "eff_k = {}",
            a.eff_k_shannon
        );
        assert!(close(a.eff_k_min_entropy, 12.5, 1e-9));
        assert!(close(a.leakage_min_entropy_bits, 3.0, 1e-9));
        assert_eq!(a.worst_case, 1);
        assert!(
            close(a.guessing_entropy, 16.18, 1e-9),
            "G = {}",
            a.guessing_entropy
        );
        assert!(close(a.good_turing_coverage, 0.98, 1e-9));
        assert!(close(a.chao1, 9.0, 1e-9));
    }

    /// Reference vector B: the shape of the published live-pool measurement —
    /// one class of 19 plus eleven singletons.
    #[test]
    fn reference_vector_b() {
        let b = Anonymity::from_class_sizes(&[19, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]).unwrap();
        assert_eq!(b.nominal_k, 30);
        assert_eq!(b.classes, 12);
        assert!(close(b.leakage_shannon_bits, 2.2165365038, 1e-9));
        assert!(
            close(b.eff_k_shannon, 6.4547181110, 1e-9),
            "eff_k = {}",
            b.eff_k_shannon
        );
        assert!(close(b.eff_k_min_entropy, 2.5, 1e-9));
        assert!(close(b.leakage_min_entropy_bits, 3.5849625007, 1e-9));
        assert!(close(b.guessing_entropy, 6.7, 1e-9));
        assert!(close(b.good_turing_coverage, 0.6333333333, 1e-9));
    }

    /// The identity the module rests on. If this fails, every number above is
    /// meaningless.
    #[test]
    fn the_chain_rule_holds() {
        for sizes in [
            vec![1u64],
            vec![5, 5],
            vec![50, 20, 10, 10, 5, 3, 1, 1],
            vec![19, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
            vec![1000, 1],
            (1..=40u64).collect::<Vec<_>>(),
        ] {
            let a = Anonymity::from_class_sizes(&sizes).unwrap();
            assert!(
                a.chain_rule_residual().abs() < 1e-9,
                "H(C) + H(X|C) != log2 K for {sizes:?}: residual {}",
                a.chain_rule_residual()
            );
        }
    }

    #[test]
    fn the_ordering_invariant_holds_over_many_partitions() {
        // A deterministic sweep rather than a random one, so a failure is
        // reproducible without recording a seed.
        let mut checked = 0;
        for m in 1..=12u64 {
            for spread in 0..12u64 {
                let sizes: Vec<u64> = (0..m).map(|i| 1 + i * spread).collect();
                let a = Anonymity::from_class_sizes(&sizes).unwrap();
                assert!(a.ordering_holds(), "ordering violated for {sizes:?}: {a:?}");
                assert!(a.chain_rule_residual().abs() < 1e-9);
                checked += 1;
            }
        }
        assert!(checked > 100);
    }

    /// One class means no leakage and full anonymity.
    #[test]
    fn a_single_class_leaks_nothing() {
        let a = Anonymity::from_class_sizes(&[64]).unwrap();
        assert!(close(a.leakage_shannon_bits, 0.0, 1e-12));
        assert!(close(a.loss_factor, 1.0, 1e-12));
        assert!(close(a.eff_k_shannon, 64.0, 1e-9));
        assert_eq!(a.worst_case, 64);
        assert!(a.worst_case_is_informative());
    }

    /// All singletons is total deanonymisation, and the folklore formula would
    /// report this as *maximum* anonymity.
    #[test]
    fn all_singletons_is_total_deanonymisation() {
        let a = Anonymity::from_class_sizes(&[1; 32]).unwrap();
        assert!(
            close(a.eff_k_shannon, 1.0, 1e-12),
            "eff_k = {}",
            a.eff_k_shannon
        );
        assert!(close(a.eff_k_min_entropy, 1.0, 1e-12));
        assert!(close(a.loss_factor, 1.0 / 32.0, 1e-12));
        // The inverted folklore quantity, for contrast: 2^{H(C)} = 32 here,
        // which would be reported as "effective k = 32" — the nominal size, at
        // the exact moment every member stands alone.
        assert!(close(a.leakage_shannon_bits.exp2(), 32.0, 1e-9));
    }

    #[test]
    fn the_loss_factor_is_independent_of_k_for_a_fixed_prior() {
        // Doubling every class doubles nominal k and doubles effective k, so the
        // ratio is unchanged. That is why it is the headline.
        let small = Anonymity::from_class_sizes(&[8, 4, 2, 1, 1]).unwrap();
        let large = Anonymity::from_class_sizes(&[16, 8, 4, 2, 2]).unwrap();
        assert!(
            close(small.loss_factor, large.loss_factor, 1e-12),
            "{} vs {}",
            small.loss_factor,
            large.loss_factor
        );
        assert!(large.nominal_k == 2 * small.nominal_k);
    }

    #[test]
    fn worst_case_alone_is_not_a_finding() {
        // Both have a singleton, but they are not equally private. Reporting
        // "worst case 1" for either says the same thing about two very
        // different pools.
        let bad = Anonymity::from_class_sizes(&[1; 20]).unwrap();
        let good = Anonymity::from_class_sizes(&[999, 1]).unwrap();
        assert_eq!(bad.worst_case, good.worst_case);
        assert!(!bad.worst_case_is_informative());
        assert!(!good.worst_case_is_informative());
        assert!(good.eff_k_shannon > 900.0);
        assert!(bad.eff_k_shannon < 1.001);
    }

    #[test]
    fn the_ccdf_reports_the_tail_the_averages_hide() {
        let a = Anonymity::from_class_sizes(&[100, 1, 1, 1, 1]).unwrap();
        // Four of 104 members are alone.
        let at_one = a.class_size_ccdf.iter().find(|(t, _)| *t == 1).unwrap().1;
        assert!(close(at_one, 4.0 / 104.0, 1e-12));
    }

    #[test]
    fn an_empty_or_degenerate_partition_is_refused() {
        assert!(Anonymity::from_class_sizes(&[]).is_none());
        assert!(Anonymity::from_class_sizes(&[3, 0, 2]).is_none());
    }

    #[test]
    fn labels_group_into_the_same_partition_as_sizes() {
        let by_label =
            Anonymity::from_labels(&["binance", "binance", "binance", "coinbase", "rootless"])
                .unwrap();
        let by_size = Anonymity::from_class_sizes(&[3, 1, 1]).unwrap();
        assert!(close(by_label.eff_k_shannon, by_size.eff_k_shannon, 1e-12));
        assert_eq!(by_label.classes, 3);
    }
}
