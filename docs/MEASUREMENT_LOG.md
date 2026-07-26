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

## What we do not conclude

Nothing about how private Privacy Cash is. ρ = 0.1219 is a point estimate inside
a bracket that spans an order of magnitude, from a sample whose class
distribution is two-thirds unobserved. The number is published because the method
is published, not because it settles anything.
