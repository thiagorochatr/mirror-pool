# mirror-pool

A behavioral anonymity set for Solana. Members deposit a fixed denomination;
later, a member proves in zero knowledge that they own some note in the set and
directs the pool to act. The pool executes. An observer sees that an action
happened and cannot say which member asked for it.

Rust end to end. MIT. No Anchor, no Circom, no JavaScript anywhere in the
proving path.

```
make verify          # fmt, clippy -D warnings, tests, build-sbf
```

## The problem this takes as its subject

Every serious submission to this bounty — across all three repositories —
identifies the same open channel and none of them closes it:

> An anonymity set on a public ledger can be partitioned by **where each member's
> capital came from**. Learning a member's funding class leaves only that class to
> guess within, so what survives is the size of the class, not `k`.

No deposit pool controls where its users' money came from, so this cannot be
fixed by a better circuit. What it can be is *measured*, and measured honestly.

So this submission claims exactly two things:

1. **The action side is closed.** Actions execute from the pool's vault PDA, so an
   action's on-chain funding trace leads to the pool and is identical for every
   member.
2. **The membership side is measured**, from real mainnet data, with the method
   and its limits published beside the number. Measured on a live pool's
   depositors: **`ρ = 0.1032`**, inside an unresolved bracket of
   `0.0316 … 0.1318`, from a clean census with zero infrastructure failures —
   50 of 84 members reached a provenance class. Knowing a member's funding class
   costs that pool roughly an order of magnitude of its nominal anonymity.

   The sample's class distribution is heavy-tailed and two-thirds unobserved,
   and that is reported as a finding rather than hidden as a caveat: raising the
   traversal budget between runs resolved ten more members and made coverage
   *worse*, because the new members landed in new singleton classes rather than
   in the observed ones. There is no budget at which this distribution becomes
   well-observed. `docs/MEASUREMENT_LOG.md` has every run, including the two
   that produced nothing, and Run 4's budget was committed before it ran.

Anything we cannot support with a measurement whose method is published, we do
not say. There is a section below of things we deliberately do not claim.

## What is here

| | |
|---|---|
| `programs/mirror-pool` | The on-chain program. `submit_spend`, proof and all, measured at **97,860 CU** on a real SVM. |
| `crates/mirror-core` | Field, Poseidon, Merkle accumulator, notes. Linked on-chain. |
| `crates/mirror-circuit` | R1CS gadget, membership circuit, prover, key export. |
| `crates/mirror-provenance` | The funding-provenance measurement. |
| `crates/mirror-cli` | `setup`, `verify-setup`, `check-endpoint`, `seeds`, `collect`, `analyze`, `soak`. |

**182 tests.** The end-to-end suite loads the `.so` that `make build-sbf`
produces into a real SVM, sends real transactions, and verifies a real Groth16
proof through the actual syscall — so a divergence between what the host believes
and what the chain does cannot pass unnoticed.

## Properties, and how each is checked

**A note's value cannot disagree with its commitment.** The denomination is a
pool constant rather than a field in the note, so `vault ≥ denomination ×
outstanding` is a function of two counters that nothing a prover supplies can
influence. It is re-read from the vault after lamports move rather than inferred
from the arithmetic that moved them.

**A nullifier is spent once, ever** — never epoch-scoped.

Those two together make a class of drain *unrepresentable* rather than merely
untested. Two competing submissions are drainable at exactly this point: one
escrows an amount never bound to its hidden commitment, so a depositor of one
lamport can withdraw the whole pool with a valid proof; the other issues an
epoch-scoped nullifier against a value payout, so one deposit pays out once per
epoch forever.

**A relay cannot redirect or re-price an action.** The action binding is never
transmitted — it is recomputed on-chain from the selector, the target program,
the beneficiary, the relay fee, the declared account count and the payload, then
used as the third public input, so altering any of them changes the binding and
the pairing fails. The test tampers with that exact input and asserts the real
verifier rejects it.

It does **not** bind *which* accounts fill an action's slots, only how many.
Settlement is permissionless, so a settler chooses them; for a target whose
destination is an account rather than instruction data, that is a real limit and
`docs/THREAT_MODEL.md` states it.

**No member key ever appears on chain** on the relay path.

**Nothing can hold a member's escrow.** There is no `self_spend` instruction
because none is needed: a member acts as their own relay at zero fee, and
settlement is permissionless, so they settle their own batch once the timeout
passes. The cost is the expected one — their wallet signs, giving up anonymity —
and the test pins that the exit works.

**Gadget, host and syscall compute one hash.** The gadget and the host are each
pinned to circomlib's published `poseidon([1,2])` vector rather than to each
other, so the two agreeing on a wrong answer would need circomlib's own vector to
be wrong. The syscall is then checked against the host on-chain: the end-to-end
suite asserts the root the deployed program builds equals the root the host
built. Several published Solana projects ship a gadget whose native and
in-circuit hashes differ; that only surfaces at proving time.

## The measurement

Three commands, two passes, and the split is the point:

```
mirror seeds   --program <program-id>  # member-weighted frame, one row per depositor
mirror collect --seeds seeds.txt    # the only networked step; writes sample.json
mirror analyze --sample sample.json # pure, offline, deterministic
```

`data/sample-privacycash-run4.json` is the committed artifact behind the
headline, and `data/sample-privacycash.json` is the earlier run it is compared
against. Both are committed, so anyone holding them recomputes the numbers
without RPC access and without trusting that our endpoint behaved the same way
on their machine — including the run that came back above the 1% RPC-failure
limit and so yielded no headline at all. `docs/MEASUREMENT_LOG.md` records every
run, and Run 4's budget and predictions were committed to git *before* the
collection started, so the parameters are a declaration rather than a
description.

### Design choices that exist to avoid specific published defects

- **Edges come from balance deltas**, not instruction parsing, which is blind to
  every program that moves lamports by direct account mutation.
- **The birth edge is the oldest credit.** A competing tracer scans the six most
  *recent* transactions — the wrong end of the history for anything with more
  than six, which manufactures "unresolved" for active wallets.
- **The hub threshold is decoupled from the paging cap.** In a competing tracer
  the two are one number, so "reaches an attributable origin" there means "hit
  the RPC page cap" — admitting every DEX program and bot.
- **RPC failures are never evidence.** Counted separately, excluded from the
  distribution, and above a 1% failure rate the run refuses to print a headline
  rather than printing a warning above one.
- **The endpoint is checked first.** A truncated endpoint returns `null` rather
  than an error for pruned history, so a collection against one looks healthy
  and reports every old funding event as absent. `check-endpoint` refuses.
- **The frame is member-weighted.** Each depositor counts once. Sampling
  addresses because they appear in recent blocks is size-biased toward the
  highest-frequency actors.

### The folklore formula is inverted

`2^H(C)` — entropy over the class-size distribution — is widely quoted as the
effective anonymity set. It is the **leakage**: it is maximised when every member
stands alone, which is total deanonymisation. The anonymity is `2^H(X|C)`.

Our headline is the loss factor `ρ = 2^−H(C)`, because it is independent of `k`
and therefore comparable across pools. Effective-k measured at small `k`
systematically understates the steady-state loss and cannot be extrapolated
upward.

And **"worst case is 1" is not a finding.** Under any heavy-tailed provenance
prior somebody is always alone. It describes provenance in general, not the pool
being measured.

## Deployment

Live on **devnet** at `8H3cYoiAA9LM36cyPr4UEv38dhHasSu2XPSdiBfyrLEa`. The whole
lifecycle ran there against a real validator — pool creation, deposits, spends
each carrying a Groth16 proof verified by the deployed program's own syscall, and
a settlement that closed the vault to its rent-exempt minimum to the lamport.
Every signature is in `docs/PROOF.md`.

**Not on mainnet, and that is a decision rather than an omission.** The trusted
setup here is reproducible, not secure: the seed is public, so the toxic waste is
public, so proofs are forgeable by anyone who runs the setup. A live pool with
that property would be inviting deposits it cannot protect. Publishing a threat
model that says proofs are forgeable and simultaneously advertising a mainnet
address would be incoherent.

Devnet demonstrates everything mainnet would: same runtime, same `alt_bn128`
syscall, same verifier, same bytes. What mainnet would add is a claim about
readiness that this setup does not support yet. The prerequisite is a real
multi-party ceremony, not more SOL.

## What we do not claim

- Not "unlinkable", not "untraceable", not "anonymous" without a named adversary
  and a stated population.
- **Not that the funding-provenance channel is closed.** It is closed on the
  action side and measured on the membership side.
- Not that the trusted setup is secure. It is *reproducible*, which is a
  different and lesser property: the seed is public, so the toxic waste is
  public, so proofs are forgeable. `mirror verify-setup` re-derives the key from
  the public seed and compares it element by element against the one compiled
  into the program — expected digest
  `b0165d5eac6fe8273b6564c78e8ba548c97e6050ae785e9142de63c81aa905b7`. A competing submission
  publishes its entropy string *and* gitignores its proving key, so its setup is
  insecure and unreproducible at once — no third party can produce a valid proof
  for its deployed program at all.
- The on-chain `k_floor` bounds **program-visible membership** only. That is all
  a program can check.
- Not audited.

`docs/MEASUREMENT_LOG.md` records every collection run, including the one that
produced nothing. A measurement project that keeps only its successful runs is
selecting rather than reporting.

## Documentation

| | |
|---|---|
| `docs/ARCHITECTURE.md` | The design, and why each decision is what it is. |
| `docs/PROVENANCE_METHOD.md` | Adversary model, metrics, sampling, the honest-claims analysis. |
| `docs/GROTH16_INTEGRATION.md` | The arkworks-to-Solana byte layout, verified by execution. |
| `docs/MEASUREMENT_LOG.md` | Every run. |
| `docs/THREAT_MODEL.md` | The adversary, what holds, and every place it stops. |
| `docs/PROOF.md` | Devnet signatures for every flow, and the rejections. |
| `docs/PLAN.md` | What was planned, and what was cut. |
