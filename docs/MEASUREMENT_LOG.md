# Measurement log

Every collection run, including the ones that produced nothing. A measurement
project that only records its successful runs is not reporting, it is selecting.

---

## Run 1 — 2026-07-25, pipeline validation, no publishable result

**Purpose.** End-to-end validation of collect → analyze against live mainnet.

**Frame.** 12 addresses credited at least 0.001 SOL in a recent finalised block,
filtered to accounts owned by the system program and currently existing.

**Endpoint.** `api.mainnet-beta.solana.com`, first available block 0, archival
probe at slot 50,000,000 passed. 263 RPC calls, 0 failures.

**Result.**

```
attempted 12 | resolved 0 | evidence-unresolved 0 | budget-unresolved 12
             | scope-unresolved 0 | rpc failures 0 (0.00%)
```

No headline. The tool reported nothing rather than reporting something.

### What went wrong, and it was the frame

Eleven of twelve seeds hit the signature page cap at 20,000 lifetime
transactions. They are system-owned wallets, so they passed the frame filter, but
they are not people: at that volume they are market makers, arbitrage bots and
exchange hot wallets.

The cause is the sampling design, not the collector. **Sampling addresses because
they appear credited in a block is transaction-weighted, not member-weighted.**
An address that transacts a thousand times a day is a thousand times more likely
to appear in any given block than one that transacts daily, so the frame is
size-biased toward exactly the addresses whose funding history is most expensive
to walk. It is the same error as estimating how often people travel by asking
people at a bus stop.

For measuring a pool, the frame is the pool's depositors, each counted once —
which is member-weighted by construction.

### What this run does establish

- The endpoint precondition, the collector, the two-pass split and the metric
  ladder all work against live mainnet.
- Page-capped addresses are reported as **budget-unresolved**, kept distinct from
  evidence-unresolved and from RPC failure. The distinction is what makes this
  run diagnosable instead of a number.
- The tool refused to print a headline from a run where nothing resolved.

### A defect this run caught

An earlier attempt returned 14 RPC calls for 12 seeds and reported everything
unresolved. Frame validation fetches each seed's owner before tracing, and the
collector's observation cache treated "we know the owner" as "we have observed
this address", so it skipped signature paging entirely and never looked for a
birth edge.

The bug was visible only because the call count was reported and the census was
honest. A collector that defaulted a failed or skipped lookup to "no history" —
as a published tracer in this space does — would have produced a full set of
unresolved members and a plausible-looking headline, with nothing to indicate
anything was wrong.

---

## Run 2 — 2026-07-25, member-weighted frame, refused on failure rate

**Frame.** 100 depositors of Privacy Cash (`9fhQBbumKEFuXtMBDw8AaQyAjCorLGJQiS3skWZdQyQD`),
enumerated by `mirror seeds` from the program's own deposit transactions and
strided across signature pages so the sample spans the pool's history rather than
its most recent minute. 13 excluded as non-wallets before tracing; 87 attempted.

**Endpoint.** Alchemy, archival probes passed. Depth 8, page cap 60. 4,852 calls.

```
attempted 87 | resolved 47 | evidence-unresolved 8 | budget-unresolved 19
             | scope-unresolved 0 | rpc failures 13 (14.94%)
```

**No headline.** The failure rate is 14.94%, so `analyze` refused. This was the
frame working and the endpoint not: 47 of 87 members reached a class, against
zero under Run 1's transaction-weighted frame.

**What the failures actually were.** The collector reported every failure as
`RpcFailure` with no cause, which made the census honest and the run
undiagnosable. Instrumenting it showed HTTP 429 arriving at **0.7 requests per
second** — far below any documented rate limit. Providers meter by compute units,
not request count, and a 1,000-signature page is expensive in those terms. So the
cause was not the request rate, and lowering it would not have helped; the cause
was a page cap of 60, set on the mistaken belief that paging deeper would resolve
more chains.

It would not have. The volume-hub rule needs enough history to clear its
threshold and estimate an age — about eight pages. The other fifty-two were
waste that bought nothing and spent the budget that made the endpoint refuse.

## Run 3 — 2026-07-25, clean census, honest bracket

**Frame.** The same 100 depositors. 16 excluded as non-wallets; 84 attempted.

**Endpoint.** Alchemy, archival probes passed. Depth 8, **page cap 8**. 1,029
calls — a fifth of Run 2 — at 1.5 requests per second.

```
attempted 84 | resolved 40 | evidence-unresolved 5 | budget-unresolved 39
             | scope-unresolved 0 | rpc failures 0 (0.00%)
```

**Zero infrastructure failures.** Nothing in the unresolved bucket is ours.

| quantity | value |
|---|---|
| resolved members | 40 |
| provenance classes | 17 |
| **loss factor ρ (point)** | **0.1219** |
| **ρ (unresolved bracket)** | **0.0253 … 0.1837** |
| effective-k, Shannon | 4.88 |
| effective-k, min-entropy | 2.35 |
| Shannon leakage | 3.04 bits |
| Good–Turing coverage | 0.65 |
| Chao1 richness | 108 classes, against 17 observed |

**The tool declines to call this a result, and it is right.** Two gates fire:

*Not informative.* 40 of 84 members reached a class — just under half. The
bracket's two readings, unresolved merged into one class versus split into
singletons, differ by more than sevenfold. Quoting either alone would describe
the sampling budget rather than the pool.

*Under-sampled.* Chao1 estimates 108 provenance classes against 17 observed, and
Good–Turing coverage is 0.65. Most of the class distribution was never seen, and
effective-k measured at small k understates the steady-state loss and does not
extrapolate upward.

**What this run does establish.** The pipeline works end to end against live
mainnet with a clean census; the member-weighted frame resolves a near-majority
where the transaction-weighted one resolved nothing; and the honesty machinery is
load-bearing rather than decorative — it refused three consecutive runs, twice on
the failure gate and once on the bracket, and each refusal was correct.

**What a publishable ρ would need.** More resolution, not more members: 39 of the
44 unresolved are budget outcomes, chains that ran out of depth or paging before
reaching a class. That is a bigger call budget spent on depth rather than on
sample size, which is a straightforward thing to buy and not something this
submission claims to have bought.

## Run 4 — pre-registered before collection, 2026-07-25

This section was written and committed **before the run started**, so the budget
below is a declaration and not a description. Run 3 missed the informativeness
gate by two members — 40 resolved against a threshold of 42 — and a gate that
close is exactly where the temptation to tune until it opens is strongest.
Declaring the budget in advance and publishing whatever comes out is what makes
this a fourth measurement rather than a search for a fourth answer.

**Frame.** Unchanged: the same 100 Privacy Cash depositors as Runs 2 and 3, from
`data/seeds-privacycash.txt`. Not re-drawn, not extended. Changing the frame and
the budget together would make the two effects impossible to separate.

**What changes, and only this.** Run 3 left 39 members in the budget bucket —
chains that ran out of depth or pages before reaching a class. That is the one
bucket a bigger budget can move. So:

| parameter | Run 3 | Run 4 |
|---|---|---|
| depth | 8 | 16 |
| signature page cap | 8 | 24 |
| endpoint | Alchemy | Helius |
| request rate | 1.5/s | 5/s |

**Prediction, recorded in advance.** The budget bucket shrinks and resolution
clears the 42-member gate. The under-sampling flag most likely still fires:
Good–Turing coverage was 0.65 and Chao1 estimated 108 classes against 17
observed, and resolving more members raises coverage slowly. A publishable ρ
carrying an explicit sampling caveat is the expected outcome, not a clean one.

**Stopping rule.** This is the last run against this frame. Whatever it returns
is what this document reports — including a refusal, and including a ρ that sits
worse for the argument than Run 3's. If it fails on the RPC gate, that is
recorded as a failed run and not retried into success.

### Result

Helius, archival probes passed. 1,613 calls at 3.5 requests per second, 456
seconds wall clock.

```
attempted 84 | resolved 50 | evidence-unresolved 6 | budget-unresolved 28
             | scope-unresolved 0 | rpc failures 0 (0.00%)
```

| quantity | Run 3 | Run 4 |
|---|---|---|
| resolved members | 40 | **50** |
| budget-unresolved | 39 | **28** |
| rpc failures | 0 | 0 |
| provenance classes observed | 17 | 22 |
| **loss factor ρ (point)** | 0.1219 | **0.1032** |
| **ρ (unresolved bracket)** | 0.0253 … 0.1837 | **0.0316 … 0.1318** |
| bracket width (ratio) | 7.3× | **4.2×** |
| effective-k, Shannon | 4.88 | 5.16 |
| Good–Turing coverage | 0.65 | **0.62** |
| Chao1 richness | 108 | **193** |

**The informativeness gate clears.** 50 of 84 resolved, against a threshold of
42. The bracket narrows from a 7.3-fold span to a 4.2-fold one, and both
readings now sit on the same side of 0.15. This is the first run in this
document that produces a headline.

**The under-sampling flag still fires, and it got worse.** This is the part
worth reading twice. Resolving ten more members did not improve coverage — it
*degraded* it, from 0.65 to 0.62, and pushed the Chao1 richness estimate from
108 classes up to 193.

That is not a defect and not noise. The additional members did not land in the
classes already observed; they landed in new ones, mostly alone. Under
Good–Turing, coverage falls when the share of singletons rises, so a sample that
keeps discovering fresh singleton classes reports *less* confidence as it grows.

The consequence matters for the conclusion. Under-sampling here is **a property
of the provenance distribution, not an artifact of our budget.** Run 3 left it
open whether more resolution would close the tail; Run 4 answers that, and the
answer is no. Spending more calls would raise the resolved count and lower the
coverage further. There is no budget at which this frame's class distribution
becomes well-observed, because the tail is where the mass is.

So the honest statement of what was measured is: **ρ ≈ 0.10 on this pool, inside
0.032 … 0.132, from a sample whose class distribution is demonstrably
heavy-tailed and two-thirds unobserved — and the second fact is now a finding
rather than a caveat.**

**Stopping rule honoured.** No further run against this frame. The Run 3
artifact stays committed beside the Run 4 one; both are in `data/`, and the
numbers above are recomputable from either without RPC access.

## Run 5 — pre-registered before collection, 2026-07-26

Committed before the collection started, like Run 4.

**Why a second population, and why this one.** Run 4 produced ρ = 0.1032 for a
privacy pool's depositors. On its own that number has no scale: a reader cannot
tell whether 0.10 is concentrated or ordinary, because there is nothing to
compare it against. ρ was chosen as the headline precisely because it is
independent of `k` and therefore comparable across populations — and so far that
property has been asserted and never exercised.

The comparison that gives the number a scale is not a second privacy pool. It is
a population that **is not seeking privacy at all**, measured by the identical
pipeline. If ordinary users show the same provenance concentration, then 0.10
describes Solana rather than the pool. If the privacy pool is more concentrated,
that is a finding about who privacy tools attract. Either answer is worth more
than a second point on the same curve.

**Frame.** Depositors of Marinade liquid staking,
`MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD`, enumerated by `mirror seeds` from
the program's own transactions, member-weighted and strided across signature
pages exactly as the Privacy Cash frame was. Target 100 depositors.

This is a **control, not a privacy pool**, and it is labelled as one everywhere.
Staking SOL is an ordinary, publicly-attributable act; nobody does it for
deniability.

**What a feasibility probe already established, and what it did not.** A probe of
178 transactions returned 5 distinct depositors — a yield near 3%, against about
12% for the privacy pool, because most Marinade program traffic is unstaking and
bot activity rather than deposits. So the frame is constructible but costs
roughly four times as many fetches per member. That is a fact about the budget.
The probe was not run through `analyze` and nothing about the outcome is known
at the time of writing.

**Prediction, recorded in advance.** Genuinely uncertain, which is the reason to
write it down. The expectation is that the control is **less** concentrated —
lower ρ — because a privacy pool is plausibly reached through a narrower set of
funding routes. A control that came back at or above 0.10 would say the
concentration is a property of Solana's funding graph rather than of the pool,
and that would be the more interesting result of the two.

The under-sampling flag is expected to fire again. Nothing about the tail found
in Run 4 suggests a different population would have a lighter one.

**Stopping rule.** One run at this frame size. Whatever it returns is reported,
including a result that undercuts the argument for measuring privacy pools at
all, and including a refusal.

## Runs 6 and 7 — pre-registered re-collections, forced by a bug in our own tracer

Committed before either collection started.

**Why these exist.** Run 5's control produced an unresolved bucket six times
larger than the privacy pool's, in the *evidence* category. Checking two of those
seeds by hand found the cause, and it was ours: the tracer implemented "the
oldest transaction, if it is a credit" while the documented rule — in the README,
in the method, and in the module's own comment — is **the oldest credit**. An
address whose first transaction merely references it, which is common, was
reported as having no funding at all.

That is an infrastructure limit reported as evidence about the chain, which §6 of
the method forbids in as many words. It is fixed, with the exhausted case given
its own outcome so it counts as budget and never as evidence.

**Every `ρ` published before this point was computed with the broken tracer**,
including the Run 4 headline. So both populations are collected again.

| | frame | parameters |
|---|---|---|
| **Run 6** | the same 100 Privacy Cash depositors as Runs 2–4 | depth 16, page cap 24, birth scan 24 |
| **Run 7** | the same 100 Marinade depositors as Run 5 | depth 16, page cap 24, birth scan 24 |

Identical parameters to Run 4 and Run 5 respectively, save the birth scan the fix
introduces. Same frames, not re-drawn.

**Prediction, recorded in advance.** Resolution rises in both, and by more in the
control, because that is where the misdiagnosed bucket was concentrated. The
direction of the *comparison* is genuinely unknown, and that is the point of
running both: the bug suppressed resolution in whichever population had more
addresses whose first transaction was not their funding, and there is no reason
to assume that is the one that flatters us.

`ρ` itself may move in either direction. Members who were previously dropped will
land in classes, and whether they land in the crowded ones or in new singletons
decides the sign.

**Stopping rule.** One collection per frame at these parameters. Both are
reported whatever they say — including a corrected headline worse than the one
already published, and including the comparison failing to separate.

**What is not being re-run.** Runs 1 and 2, which produced no headline: one on a
size-biased frame, one above the RPC-failure limit. Neither's conclusion depends
on the birth-edge rule, and both remain in this document as they were.

### Run 6 — the privacy pool, re-collected

Helius, archival probes passed. 1,776 calls, 364 seconds.

```
attempted 83 | resolved 54 | evidence-unresolved 0 | budget-unresolved 29
             | scope-unresolved 0 | rpc failures 0 (0.00%)
```

| quantity | Run 4 (broken) | Run 6 (fixed) |
|---|---|---|
| resolved | 50 | **54** |
| **evidence-unresolved** | 6 | **0** |
| budget-unresolved | 28 | 29 |
| **ρ (point)** | 0.1032 | **0.0955** |
| ρ (unresolved bracket) | 0.0316 … 0.1318 | **0.0350 … 0.1136** |
| bracket width | 4.2× | **3.2×** |
| ρ (95% sampling interval) | — | 0.0848 … 0.1790 |
| classes observed | 22 | 23 |
| Good–Turing coverage | 0.62 | 0.65 |

**The evidence bucket went to zero, and that is the finding.** Chain stop reasons
before and after:

| stop reason | Run 4 | Run 6 |
|---|---|---|
| `PageCapHit` | 69 | 76 |
| `NoIncomingEdge` | **10** | **0** |
| `LocalTerminal` | 4 | 5 |
| `BirthScanExhausted` | — | 1 |
| `DepthExceeded` | 1 | 1 |

Ten addresses were previously reported as having **no funding credit at all** — a
claim about the chain, not about us — and every one of them was wrong. One
address now honestly reports that we stopped reading before finding its credit,
which is a claim about us and is counted as budget.

**The corrected headline is ρ = 0.0955**, lower than the number published before,
and the bracket is a third narrower. The correction happened to move our own
figure in the flattering direction; it was made because the code disagreed with
its own documentation, and the direction was not known until the run finished.

The under-sampling flag still fires, and Run 4's finding about the tail survives
the fix: coverage remains near 0.65 with Chao1 estimating 108 classes against 23
observed.

## What we do not conclude

**Nothing about how private that pool is, and the headline does not become that
claim by clearing a gate.** ρ = 0.1032 is a point estimate inside a bracket
spanning 0.032 to 0.132, drawn from one frame of one pool at one moment, and its
class distribution is demonstrably heavy-tailed and two-thirds unobserved. It is
published because the method is published, not because it settles anything.

Three things it specifically does not support:

- **A comparison between pools.** ρ is designed to be comparable across pools —
  that is why it is the headline — but only one pool has been measured. The
  property is asserted here and not yet exercised.
- **An extrapolation to larger `k`.** Effective-k measured at small `k`
  understates the steady-state loss, and the heavy tail found in Run 4 is a
  reason to expect that gap to widen rather than close.
- **A statement about any individual member.** The metric is a property of a
  distribution. Nothing here identifies anyone, and the worst-case class of size
  one is a feature of heavy-tailed provenance in general, not a finding about a
  person.

What the four runs do support is narrower and, we think, more useful: that this
channel is measurable from live Solana data with a published method, that the
measurement is expensive and the tail does not close, and that a pipeline built
to refuse is one that refuses — three times, correctly, before it produced a
number.
