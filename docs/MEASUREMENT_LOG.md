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

## What we do not conclude

Nothing about how private Privacy Cash is. ρ = 0.1219 is a point estimate inside
a bracket that spans an order of magnitude, from a sample whose class
distribution is two-thirds unobserved. The number is published because the method
is published, not because it settles anything.
