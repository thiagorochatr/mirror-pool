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

## Run 2 — planned

**Frame.** Depositors of a live pool, enumerated from its deposit instructions,
each address counted once regardless of how often it transacts.

This is the member-weighted frame, and it is the only one from which an
effective-k for that pool means anything.
