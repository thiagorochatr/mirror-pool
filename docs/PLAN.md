# mirror-pool — implementation plan

## The thesis

`mirror-pool` is meant to be a crowd-sourced privacy set: users pool their
activity so that an observer sees an action happened but cannot say who
initiated it. The hard part is not the proof system. It is that **an anonymity
set on a public ledger can be partitioned by where each member's capital came
from**, and once you sort a set of `k` members into funding-provenance classes,
the anonymity that remains is the size of the actor's class, not `k`.

This is the one problem every serious submission across all three repos of this
bounty concedes and none closes. So it is the problem this contribution takes as
its subject.

We do not claim to hide provenance. Value moves one way on a ledger, and no
deposit pool controls where its users' money came from. What we claim is
narrower and defensible:

1. **The action side is closed.** Actions are executed by the pool's vault PDA, so the
   on-chain funding trace of an action leads to the pool and is identical for
   every member.
2. **The membership side is measured, not asserted.** Effective anonymity is
   computed from real mainnet chain data at several label resolutions, and the
   method and its limits are published with the number.
3. **The measurement is load-bearing.** A pool that cannot demonstrate an
   effective-`k` above a floor refuses to settle. The metric is a protocol
   parameter, not a marketing line.
   *(Withdrawn during implementation — see "What actually got cut". A program
   cannot check provenance, so this claim was not deliverable.)*

If we cannot support a claim with a measurement whose method we publish, we do
not make the claim.

## What we will not claim

- Not "unlinkable", not "untraceable", not "anonymous" without a named adversary
  and a stated population.
- Not that the funding-provenance channel is closed. It is reduced on the action
  side and measured on the membership side.
- Not that a single-contributor trusted setup is secure. It is reproducible,
  which is a different and lesser property, and we say which one we have.
- No number in any document may come from a synthetic fixture presented as a
  measurement. Every reported figure names its data source.

## Architecture

### Pool model — fixed denomination, note-based

One pool instance serves exactly one denomination `D`. A deposit escrows exactly
`D` plus an entry fee and inserts a note commitment into an incremental Poseidon
Merkle accumulator. A spend proves membership of some note, publishes its
nullifier, and directs the pool to execute one action disbursing exactly
`D − relay_fee`.

```
note        = (k, r)                 secret nullifier preimage, blinding
commitment  = Poseidon(k, r)         the Merkle leaf
                                     (shipped as Poseidon(k, r, denom_tag) —
                                      binding the denomination in is cheap
                                      defence in depth)
nullifier   = Poseidon(k)            revealed on spend, spent once ever
```

The denomination is a pool constant, so it never needs to be bound into the
commitment and can never disagree with the escrowed amount.

**Accounting invariant**, asserted in tests:
`vault.lamports >= D × (deposits − spends) + rent_exempt_minimum`.

This is deliberate, and it targets two drains that a shielded pool reaches by
building in the obvious direction. Carry the amount as a note field without
binding it to the hidden commitment, and a depositor of one lamport withdraws
the pool holding a valid proof. Scope the nullifier to an epoch — natural once
epochs are how batches form — and a single deposit pays out once per epoch,
forever. A fixed denomination with spend-once nullifiers makes both
unrepresentable rather than merely untested.

### Circuit — 3 public inputs

The statement: *I know `(k, r)` such that `Poseidon(k, r)` is a leaf of the tree
with root `R`, my nullifier is `Poseidon(k)`, and this proof is bound to
`action`.*

| Public input | Why it must be public |
|---|---|
| `root` | the program checks it against its own root history |
| `nullifier` | the program records it to prevent replay |
| `action_binding` | the program recomputes it, so a relay cannot redirect or re-price |

Measured cost is `74,179 + 5,661 × N` CU, so three inputs land near 91k CU. That
is the going rate for on-chain Groth16 on Solana, and the third input buys the
relay binding rather than being spent on a value that could have been folded in.
Any further value we need to commit to gets folded into `action_binding` rather
than added as a fourth input. See `GROTH16_INTEGRATION.md` for the verified
conversion path and its pitfalls.

The action binding covers the action selector, its parameters, the beneficiary
and the relay fee, so every economically meaningful field is inside the proof.

### Two-phase epochs — synchronization without a CU explosion

The bounty's premise is synchronized crowds: many identical actions landing
together so that timing and ordering carry no signal. Verifying `N` proofs in one
transaction does not fit — three proofs exhaust the transaction size limit long
before the compute limit.

So the protocol splits it:

- **Phase 1, any time during the epoch.** A member submits their proof in its own
  relay-signed transaction. The program verifies it, burns the nullifier and
  writes a ticket PDA recording the bound action. Nothing executes yet.
- **Phase 2, at epoch close.** The pool executes every ticket's action in one
  settlement, sharing a single timestamp and ordering.

The two phases also close a structural leak that a single-phase design walks
into: if settlement is one transaction that every participant must sign, then
that transaction publishes the entire membership set by pubkey, and the
anonymity set is disclosed by the very step meant to protect it. Here no member
key ever appears on chain.

### Relays, and never holding users hostage

A member who pays their own fee signs with their own wallet and destroys their
own anonymity, so spends are relay-signed and the relay is paid out of `D`.

Two rules follow, and both are easy to lose by accident:

- **Relaying is permissionless.** Any key may relay. There is no single immutable
  authority whose loss freezes the pool. Baking a relay authority into the PDA
  seeds is the tempting shortcut, and it is a trap: seeds cannot be changed, so
  without a rotation instruction the authority is permanent and its loss is
  terminal.
- **There is always an exit.** A member may always self-spend, paying their own
  fee and accepting the privacy loss, so funds are never hostage to a relay's
  liveness.

### Incentives

Entry fees accumulate in a reward pool paid pro-rata to *dwell* — how long a note
stayed unspent — which is the behaviour that makes everyone else's set larger.
Payouts are capped per epoch and gated on the `k` floor, so a lone participant
cannot recycle their own fee back to themselves — a reward scheme that pays out
without a crowd gate is not an incentive, it is a refund with extra steps.

### Provenance measurement

A separate crate collects real mainnet funding data and reports effective
anonymity at several label resolutions, with the raw sample committed so results
reproduce without RPC access. The methodology is `docs/PROVENANCE_METHOD.md`; it
targets
the specific weaknesses found in the two published attempts: selection bias that
inflates the untraceable bucket, a sample spanning four seconds of chain time
presented as a population, and a class key defined at raw-address resolution so
that two members funded from the same exchange land in different classes.

## Schedule

92 hours remain at the time of writing. Deadline is 2026-07-28 23:59 BRT.

### Day 1 — primitives and circuit
`mirror-core`: Poseidon over BN254, the incremental Merkle accumulator with its
zero ladder, note commitments and nullifiers, and the byte layouts shared with
the program. `mirror-circuit`: the R1CS statement, key generation, the host
prover, and export of the verifying key in the on-chain layout.

*Done when* a proof generated on the host verifies against the exported
verifying-key bytes, and an in-circuit Merkle root equals one computed natively.

### Day 2 — the on-chain program
Instructions: `init_pool`, `deposit`, `open_epoch`, `submit_spend` (verify, burn
nullifier, write ticket), `settle_epoch`, `claim_reward`, `self_spend`. State:
pool, epoch, frontier accumulator with root history, nullifier PDAs, tickets.

*Done when* the suite runs against the built `.so` and covers the accounting
invariant, replay rejection, `k`-floor rejection, action-binding mismatch,
malformed input on every handler, and a drain attempt of each shape described
above. CI green on fmt, clippy, tests and `build-sbf`.

### Day 3 — measurement, CLI, setup ceremony
`mirror-provenance` against real mainnet data with the sample committed. The CLI
end to end: keygen, deposit, prove, spend, claim, and an `audit` command that
recomputes effective-`k` from committed data. Setup with a transcript that binds
the **full** verifying key and a circuit digest, and a `verify-setup` any third
party can run — checking every element, because a verifier that compares only
`delta` would green-light a verifying key belonging to a different circuit.

### Day 4 — deploy, evidence, submission
Devnet soak producing signatures for every flow plus on-chain negative cases with
their error codes, then mainnet deployment. Documentation: README, ARCHITECTURE,
THREAT_MODEL with honest limits, PROOF, EFFECTIVE_K. Open the PR with time for
CI to finish, and submit on Earn.

### Cut lines, in the order things get dropped

Multi-party ceremony support degrades to a reproducible single-contributor setup
with the multi-party path documented. Dwell rewards degrade to a flat entry fee.
The provenance sample shrinks before its method weakens. **The accounting
invariant, the negative tests, the honest limitations section and a green CI are
never cut** — they are the difference between a privacy tool and a demo of one.

### What actually got cut, and why

This plan is as written on day zero. Six things in it did not ship, and each was
a decision rather than an overrun:

- **`open_epoch` and epoch state.** Folded into `submit_spend` and
  `settle_epoch`, which need none: a spend records its own timestamp and
  settlement reads the clock. Four instructions instead of seven.
- **`claim_reward` and dwell rewards.** Cut to a flat entry fee, as the cut line
  above anticipated. The fees accrue on the pool account and no instruction pays
  them out, which is a known loose end rather than a feature.
- **`self_spend`.** Not built because it is not needed: a member relays for
  themselves at zero fee and settles their own batch after the timeout. Same
  exit, one fewer instruction, less attack surface. Pinned by
  `a_member_can_always_exit_without_any_relay`.
- **The `audit` command.** Shipped as `analyze`.
- **"The measurement is load-bearing."** Withdrawn rather than descoped. A
  program cannot check funding provenance, so the on-chain `k_floor` bounds
  program-visible membership only and the measurement lives beside it. Claiming
  otherwise would have been exactly the marketing line this plan disavows.
- **Mainnet.** Deliberately not deployed: the setup is reproducible rather than
  secure, and a live pool with public toxic waste would invite deposits it cannot
  protect. The README states the reasoning.

`EFFECTIVE_K` shipped as `docs/PROVENANCE_METHOD.md`.

### Stretch, only if the above is complete

A composability contribution to `account-cooker`, funding an agent fleet through
mirror-pool so that every agent wallet shares one provenance class. An agent
fleet is defeated by its common-funder graph long before its behaviour looks
wrong, and a shielded funding path is the piece that attacks it. It is a second
prize with a separate champion, and the work is small once the pool exists.

## Risks

**The circuit costs more than a day.** Mitigated: the two integration unknowns
that usually eat that day — the arkworks-to-Solana byte layout and the
Poseidon-to-gadget agreement — were resolved before implementation started.

**RPC capacity for the provenance sample.** Mitigated by committing the raw
sample so the result reproduces offline, and by sizing the sample to the access
we actually have rather than to the number we would like to report.

**Mainnet deployment cost.** A program of this size is a few SOL to deploy.
Devnet evidence is the fallback and is on the critical path regardless.

**The field moves while we build.** The response is not to race anyone's line
count. It is to ship the thing this space declares open and unsolved, and to
make every number in it checkable by someone who does not trust us.
