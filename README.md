# mirror-pool

A behavioural anonymity set for Solana — privacy for **what you do**, not for
what you hold.

Members join a pool by depositing a fixed denomination. Later, a member proves in
zero knowledge that they own some note in the set and directs the pool to perform
an *action*: an arbitrary instruction on an arbitrary program. The pool performs
it, signed by the pool, batched with everyone else's at one timestamp. An
observer sees that a stake, a swap or a vote happened and cannot say which member
asked for it.

Moving lamports is the degenerate case, selector zero. The interesting case is
selector one, and the end-to-end suite runs it against a real deployed program:
four members, four memos, one slot, one signer — [see below](#synchronised-actions-are-the-point).

Rust end to end. MIT. No Anchor, no Circom, no JavaScript anywhere in the
proving path.

```
make verify          # fmt, clippy -D warnings, tests, build-sbf
```

## The problem this takes as its subject

There is one channel that no deposit-based anonymity set on a public ledger
closes, and it is not something a better circuit can fix:

> An anonymity set on a public ledger can be partitioned by **where each member's
> capital came from**. Learning a member's funding class leaves only that class to
> guess within, so what survives is the size of the class, not `k`.

No pool controls where its users' money came from. What that channel can be is
*measured*, and measured honestly — which as far as we can tell nobody has done
against live Solana data with a published method.

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
| `programs/mirror-pool` | The on-chain program. `submit_spend`, proof and all, measured at **101,123 CU** on a real SVM. |
| `crates/mirror-core` | Field, Poseidon, Merkle accumulator, notes. Linked on-chain. |
| `crates/mirror-circuit` | R1CS gadget, membership circuit, prover, key export. |
| `crates/mirror-provenance` | The funding-provenance measurement. |
| `crates/mirror-cli` | `setup`, `verify-setup`, `check-endpoint`, `seeds`, `collect`, `analyze`, `soak`. |

**183 tests.** The end-to-end suite loads the `.so` that `make build-sbf`
produces into a real SVM, sends real transactions, and verifies a real Groth16
proof through the actual syscall — so a divergence between what the host believes
and what the chain does cannot pass unnoticed.

## Synchronised actions are the point

The brief asks for an anonymity set for *behaviour*, not for funds. That
distinction is load-bearing here, so it is tested rather than asserted:

```
settled 4 real CPI actions in one transaction, 39,644 CU
```

`a_crowd_of_members_perform_a_real_protocol_action_together` seeds a pool,
loads the **real SPL Memo program** into the SVM, and has four members each
prove membership and request the identical memo. Settlement invokes Memo four
times in a single transaction, every invocation signed by the pool's vault PDA.

What an observer holds afterwards is four identical memos, one timestamp, one
signer, and no field anywhere in the transaction that distinguishes which member
asked for which. The actions are indistinguishable **by content** because the
payloads match, and indistinguishable **by timing** because settlement gives them
one clock.

Two properties make this a behavioural tool rather than a mixer with a CPI bolted
on:

- **The target is arbitrary.** Selector one invokes any program with any payload.
  Staking, voting and swapping are the same code path as the memo; Memo is used
  in the test because it is small, real, and validates its own account list.
- **Moving lamports is the degenerate case.** Selector zero is a plain transfer,
  kept only because expressing "pay this account" should not require a target
  program.

A separate test passes a **non-empty account list** through the CPI, which
matters because SPL Memo requires every account handed to it to have signed. If
the program dropped an account, mislabelled a signer flag, or miscounted, Memo
rejects — so the account plumbing is load-bearing in that test rather than
decorative.

The honest limit: the action binding commits to *how many* accounts an action
takes, not *which*. Settlement is permissionless, so a settler chooses them. For
a target whose destination lives in an account rather than in instruction data,
that is a real gap, and `docs/THREAT_MODEL.md` states it rather than working
around it.

## Properties, and how each is checked

**A note's value cannot disagree with its commitment.** The denomination is a
pool constant rather than a field in the note, so `vault ≥ denomination ×
outstanding` is a function of two counters that nothing a prover supplies can
influence. It is re-read from the vault after lamports move rather than inferred
from the arithmetic that moved them.

**A nullifier is spent once, ever** — never epoch-scoped.

Those two together make a class of drain *unrepresentable* rather than merely
untested, and the class is worth naming because a shielded pool built the
obvious way lands in it. Carry the amount as a field in the note and forget to
bind it to the commitment, and a depositor of one lamport withdraws the whole
pool holding a perfectly valid proof. Scope the nullifier to an epoch — natural
if epochs are how you batch — and one deposit pays out once per epoch, forever.

Neither is reachable here. The denomination is not in the note, so there is no
amount to bind; and the nullifier set is global, so an epoch boundary cannot
reopen a spend. Both are pinned by tests that assert the rejection rather than
assuming it.

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
built. A gadget whose native and in-circuit hashes disagree is the classic
expensive failure here, because nothing catches it until proving time and the
symptom — proofs that verify nowhere — points at everything except the hash.

## The measurement

Three commands, two passes, and the split is the point:

```
mirror seeds   --program <program-id>  # member-weighted frame, one row per depositor
mirror collect --seeds seeds.txt    # the only networked step; writes sample.json
mirror analyze --sample sample.json # pure, offline, deterministic
```

`data/sample-privacycash-run4.json` is the committed artifact behind the
headline, and `data/sample-privacycash.json` is Run 3, the earlier and smaller
budget it is compared against. Both are committed, so anyone holding them
recomputes both numbers without RPC access and without trusting that our
endpoint behaved the same way on their machine — including the comparison that
shows coverage *falling* as resolution rose.

`docs/MEASUREMENT_LOG.md` records every run, including the two that produced no
headline at all: one whose frame was size-biased and resolved nothing, and one
that came back above the 1% RPC-failure limit. Those two have no committed
sample, and saying so is part of the record — what is committed is what the
published numbers come from. Run 4's budget and its predictions were committed
to git *before* the collection started, so its parameters are a declaration
rather than a description.

### Design choices, and the failure each one exists to avoid

Every item here is a decision that a reasonable implementation gets wrong by
default. They are listed because a provenance number is only as good as the
weakest of them, and none of them is visible in the output.

- **Edges come from balance deltas**, not instruction parsing, which is blind to
  every program that moves lamports by direct account mutation.
- **The birth edge is the oldest credit**, so the walk goes to the *start* of a
  wallet's history. Reading the most recent transactions instead is the wrong end
  of the record for any wallet with more than a handful, and manufactures
  "unresolved" for precisely the active wallets worth resolving.
- **The hub threshold is decoupled from the paging cap.** Collapse them into one
  number — easy to do, since both are "how many signatures do we look at" — and
  "reaches an attributable origin" quietly becomes a synonym for "hit the RPC page
  cap", which admits every DEX program and trading bot as an origin.
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

`docs/PROOF.md` has every signature *and* the lamports, because "closed to the
rent-exempt minimum" is the interesting part of that sentence and a list of
signatures does not show it: 80,000,028 owed against 80,000,028 paid out, and a
vault resting on its floor with a remainder of zero. The soak asserts both and
fails the run otherwise, so that table cannot record a discrepancy and still
exit successfully.

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
  `b0165d5eac6fe8273b6564c78e8ba548c97e6050ae785e9142de63c81aa905b7`. Reproducible
  and insecure is a coherent position for an unaudited submission; the incoherent
  one is a setup that is *both* insecure and unreproducible, which is what
  publishing the entropy while withholding the proving key produces — nobody can
  verify the key, and nobody can regenerate it either.
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
