# mirror-pool

A behavioural anonymity set for Solana — privacy for **what you do**, not for
what you hold.

Members join a pool by depositing a fixed denomination. Later, a member proves in
zero knowledge that they own some note in the set and directs the pool to perform
an *action*: an arbitrary instruction on an arbitrary program. The pool performs
it, signed by the pool, batched with everyone else's at one timestamp. An
observer sees that a stake, a swap or a vote happened and cannot say which member
asked for it.

Moving lamports is the degenerate case, selector zero. The interesting cases are
the other two, and the end-to-end suite runs both against a real deployed
program: four members performing one indistinguishable action in a single slot,
and the pool **signing a call as the member's authority** — which is what a stake
delegation or a governance vote needs and what a transfer cannot do.
[See below](#synchronised-actions-are-the-point).

Rust end to end. MIT. No Anchor, no Circom, no JavaScript anywhere in the
proving path.

## Getting it running

Two prerequisites, and only two.

**Rust.** `rust-toolchain.toml` pins 1.97.1 with `rustfmt` and `clippy`, so
[rustup](https://rustup.rs) installs the right version by itself the first time
you build — no manual step.

**The Agave (Solana) toolchain**, for `cargo-build-sbf`. The on-chain program is
compiled to SBF and the test suite loads that `.so` into a real SVM, so without
this the suite cannot run at all:

```
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"
solana --version          # confirms it is on PATH
```

Then:

```
git clone https://github.com/solanabr/mirror-pool && cd mirror-pool
make verify               # fmt, clippy -D warnings, build-sbf, 256 tests
```

Nothing in that command needs a network, an API key or an account with anybody,
and it is the whole check — there is no second suite, no optional extra, and no
step that only works on one CPU architecture. If you would rather look than
build, the program is live on devnet at
[`8H3cYoiAA9LM36c…`](https://explorer.solana.com/address/8H3cYoiAA9LM36cyPr4UEv38dhHasSu2XPSdiBfyrLEa?cluster=devnet)
and every claim below links to the transaction that backs it.

`make verify` is the whole check, and it is the same command CI runs — see
[`.github/workflows/ci.yml`](.github/workflows/ci.yml), which invokes `make
verify` and nothing else, so the two cannot drift apart. On a cold clone it
takes a few minutes, most of it compiling arkworks. Nothing here needs a
network, a validator or a funded wallet.

To get the member-facing tool as a binary:

```
cargo build --release     # target/release/mirror
```

[`docs/USAGE.md`](docs/USAGE.md) walks the whole journey from there — creating a
note through to settling a batch — with real devnet output for every command.
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) is the design and why each
decision is what it is.

## The problem this takes as its subject

There is one channel that no deposit-based anonymity set on a public ledger
closes, and it is not something a better circuit can fix:

> An anonymity set on a public ledger can be partitioned by **where each member's
> capital came from**. Learning a member's funding class leaves only that class to
> guess within, so what survives is the size of the class, not `k`.

No pool controls where its users' money came from. What that channel can be is
*measured*, and measured honestly — which as far as we can tell nobody has done
against live Solana data with a published method.

So this project claims exactly two things:

1. **The action side is closed.** Actions execute from the pool's vault PDA, so an
   action's on-chain funding trace leads to the pool and is identical for every
   member.
2. **The membership side is measured**, from real mainnet data, with the method
   and its limits published beside the number.

   **The pool measured is not this one.** It is Privacy Cash
   (`9fhQBbumKEFuXtMBDw8AaQyAjCorLGJQiS3skWZdQyQD`), an unrelated and live Solana
   mixer, because `mirror-pool` has no depositors and a measurement of our own
   empty pool would be a measurement of nothing. So this is a tool pointed at
   somebody else's protocol, and every number below describes theirs.

   **`ρ = 0.0955`**, inside an unresolved bracket of `0.0350 … 0.1136` and a 95%
   sampling interval of `0.0848 … 0.1790`, from a clean census with **zero**
   infrastructure failures and **zero** members whose funding we claim not to
   exist — 54 of 83 reached a provenance class. Knowing a member's funding class
   costs that pool roughly an order of magnitude of its nominal anonymity.

   The sample's class distribution is heavy-tailed and most of it was never
   observed — Good–Turing coverage 0.65, with Chao1 estimating 108 classes
   against 23 seen — and that is reported as a finding rather than buried as a
   caveat. One earlier run resolved ten *more* members and came back with
   coverage *worse*, because the new members landed in fresh singleton classes
   rather than in the observed ones. There is no budget at which this
   distribution becomes well-observed.

   `docs/MEASUREMENT_LOG.md` has all eight runs, including the three the tool
   itself refused to publish a headline from and the one whose pre-registered
   prediction turned out wrong.

Anything we cannot support with a measurement whose method is published, we do
not say. There is a section below of things we deliberately do not claim.

## What is here

| | |
|---|---|
| `programs/mirror-pool` | The on-chain program. `submit_spend`, proof and all, costs about **101,000 CU** on a real SVM — half the default budget for one instruction. |
| `crates/mirror-core` | Field, Poseidon, Merkle accumulator, notes. Linked on-chain. |
| `crates/mirror-circuit` | R1CS gadget, membership circuit, prover, key export. |
| `crates/mirror-provenance` | The funding-provenance measurement. |
| `crates/mirror-cli` | The tool. `init-pool`, `note-new`, `deposit`, `tree`, `spend`, `settle`, `disclose`, `disclose-verify` for members; `setup`, `verify-setup`, `soak`, `crowd`, `close-table` for operators; `check-endpoint`, `seeds`, `collect`, `analyze`, `compare`, `selection` for the measurement. |

**256 tests.** The end-to-end suite loads the `.so` that `make build-sbf`
produces into a real SVM, sends real transactions, and verifies a real Groth16
proof through the actual syscall — so a divergence between what the host believes
and what the chain does cannot pass unnoticed.

## Using it

```
export P=<program-id>   # every command needs it; D is the pool's denomination

mirror note-new --denomination D --out m1.json         # a note is a local secret
mirror deposit  --program $P --note m1.json            # escrow it, join the set
mirror tree     --program $P --denomination D          # rebuild the accumulator
mirror spend    --program $P --note m1.json \
                --to <addr> --relay relay.json         # the relay signs, never you
mirror settle   --program $P --denomination D          # permissionless
```

`docs/USAGE.md` is the walkthrough, and every line of output in it was produced
by running the command against devnet. It covers three more a member may want:
`init-pool`, which anyone can run, and `disclose` / `disclose-verify`, which
prove a settled action was yours to one verifier you choose.

The other eleven subcommands operate a pool (`setup`, `verify-setup`, `soak`,
`crowd`, `close-table`) or run the measurement (`check-endpoint`, `seeds`,
`collect`, `analyze`, `compare`, `selection`); `--help` documents each, and
`docs/MEASUREMENT_LOG.md` gives the exact invocation for every published
number.

**No server, no indexer, no account with anybody.** The program stores only the
accumulator's frontier — enough to append a leaf, not enough to prove one is
there — so a client needs the whole leaf set. `mirror tree` recovers it from the
transaction history, rebuilds the accumulator, and checks the root against the
one the program holds:

```
  leaves recovered  2
  pool reports      2
  rebuilt root      0c77cb909067c1a57811be8c05237aff2715c65c6250fdde764ed768096cd732
  on-chain root     0c77cb909067c1a57811be8c05237aff2715c65c6250fdde764ed768096cd732
```

A matching root proves the recovered set is complete and correctly ordered, which
is the precondition a membership proof needs. It means a member can act from a
machine that has never seen the pool, carrying nothing but their note file — and
that no operator, including us, sits between a member and their own money.

The relay signs and the member never does, so no member key appears on chain
after the deposit. Below the crowd floor, `settle` says what it is waiting for
and why rather than returning an error code.

### Giving up your anonymity, on purpose, to one person

Anonymity you cannot surrender deliberately is a liability rather than a feature:
at some point a member has to show an exchange or an accountant that a particular
action was theirs. The usual answer is a viewing key or an auditor role, and both
are standing capabilities somebody else holds — a compel path that exists is a
compel path that can be used.

There is none here. `mirror disclose` writes a file the member hands to **one
verifier they chose**, and `mirror disclose-verify` checks it by recomputing
every value in it: the commitment and the nullifier from the member's secrets,
the record's address from that recomputed nullifier, the accumulator from chain
history checked against the pool's own root, and the action read out of the
record. Ten checks, each reported separately, and one that cannot be completed is
a failure rather than a silence. A file whose stated nullifier disagrees with
what the secrets produce fails on that check while the others still pass, which
tells the verifier what was tampered with rather than merely that something was.

No on-chain component, no auditor key, no protocol path that can be made to
produce one. Two things guard the member instead:

- **It refuses before settlement.** The disclosure carries the note's secrets,
  and before the nullifier is burnt those secrets *are* the deposit — anyone
  holding them can prove membership and redirect the payout. After settlement
  they authorise nothing and only demonstrate the link, which is the thing being
  disclosed on purpose.
- **It refuses when it would cost the others too much.** Naming one action as
  yours removes you as a candidate for every other action in the pool. If that
  leaves the remaining set below the pool's floor, the command stops and says
  what it would cost, and the override is a flag the member has to type out. The
  gate is advisory — nobody can be stopped from disclosing out of band — and it
  exists so the cost is visible at the moment it is paid, by the person not
  paying it.

Thirty-one tests cover it, one per tamper case, and each asserts *which* check
failed rather than merely that verification did.

### How large a crowd fits in one settlement

Two answers, and the difference between them is a transaction format rather than
anything about the program.

**With no setup at all**, settlement is a legacy transaction that names every
account by its full 32 bytes, and the 1232-byte packet is what stops it. That
floor is measured for each shape of action, not estimated:

| the batch | members | bytes | taken |
|---|---|---|---|
| plain payments | **10** | 1228 | settled in litesvm |
| stake delegations, everyone to the same validator | **7** | 1140 | settled in litesvm |
| stake delegations, a different validator each | **6** | 1194 | settled in litesvm **and on devnet** |

```
settled 10 spends in one transaction: 1228 bytes (4 to spare), 19545 CU of 200000
11 spends: rejected by the wire at 1327 bytes, 95 over the 1232-byte limit —
while the SVM settled the same batch in 24158 CU, so compute is not the constraint
```

**With an address lookup table**, the same accounts are named by one byte each,
and the packet stops being the thing in the way at all. `mirror settle` publishes
a table automatically for any batch that will not fit legacy. Twenty members
settled that way on devnet:

```
20 spends do not fit a legacy transaction: 2218 bytes, 986 over the 1232-byte packet.
Settling through a lookup table instead.
  settlement is 332 bytes of 1232, one signature
```

[`enxa9fztmzEHM…`](https://explorer.solana.com/tx/enxa9fztmzEHMLsvhfzJwFVRsNWha7WiSEUEeNn7UPk8KAdpnWQDzEHarGL8d4ckyBvtWrEGCkuvgW7UFgFAHzp?cluster=devnet)
— twenty payouts, `numRequiredSignatures: 1`, 2 static keys and 62 resolved
through the table, 35,895 CU. Twenty recipients and twenty relays are named in
that transaction and **not one of them signed it**.

Nothing in the program changes for this. `settle_epoch` requires a signature from
the settler and from nobody else, and a lookup table can serve any account that
is not a signer — so the ceiling was always a property of what the client chose
to build, and ten is the floor rather than the maximum.

**What binds instead is the account-lock limit**, and that number was taken the
hard way: a batch of 24 was refused by devnet with `TooManyAccountLocks` at 77
accounts. `solana-transaction` exports `MAX_TX_ACCOUNT_LOCKS = 128`, but that is
the raised limit and it is not live here, so a client must plan against 64 until
it can see otherwise. Three accounts per member plus three for the pool puts
twenty members at exactly 64 locks — the transaction above sits on the limit.

Settlement adds members while the batch still fits and defers the rest, so a
settler is never handed a transaction the cluster will refuse, and `init-pool`
refuses a crowd floor higher than one settlement can carry rather than letting a
pool be created that can only ever settle on its timeout.

**The table is not free, and it is taken back down.** It costs four extra
transactions, a slot of latency, and rent — and, more to the point, while it
exists it is a public durable account listing every address the settlement is
about to touch, published *before* the settlement lands. Leaving one behind per
batch would turn a one-transaction event into a permanent on-chain index of the
batch, which is a strange thing for a privacy pool to accumulate. So settlement
deactivates it immediately and `mirror close-table` reclaims the rent and removes
the list once the runtime's cooldown has passed.

Under legacy, the **packet size** binds in all three rows. Each spend brings accounts nobody else
shares — its record, its beneficiary, its relay — so a payment costs about 99
bytes a member, and a call costs more because it also names its callee and the
callee's accounts.

The interesting row is the last one. **Actions that diverge in content cost
anonymity-set size**: a member who picks their own validator names a vote
account nobody else in the batch names, one extra key per spend, and the batch
loses a member. That is a real trade-off in the design, so it is measured from
two sides that share no code path — `a_crowd_that_agrees_on_its_validator_carries_one_more_member`
in litesvm and a live devnet run in [`docs/CROWD.md`](docs/CROWD.md) — and both
land on 1194 bytes.

Compute is not close for payments: ten settle in under 10% of the default
instruction budget. It is much closer for delegations. On devnet the six-member
divergent batch burned 142,856 of the 200,000 CU a single instruction gets, and
`CROWD.md` reports where that leaves a settler: **at that ceiling both exits are
shut**. Asking for a larger compute budget costs a second instruction, measured
at 40 bytes against that very batch, and a legacy batch has 38 to spare — so in a
legacy transaction, raising the budget means dropping a member. A lookup table
lifts that too, and how far for a batch of delegations is unmeasured: `CROWD.md`
says so rather than assuming the legacy number carries over.

`ten_spends_fit_in_one_settlement_and_the_packet_is_what_stops_the_eleventh`
demonstrates the payment limit from both sides: it measures the eleventh batch
at 1327 bytes *and* replays the identical batch into a second pool, where the
SVM settles it without complaint. If compute ever became the binding constraint,
that test fails rather than quietly reporting the wrong reason.
`every_account_an_action_names_costs_the_batch_a_member` does the same for the
two delegation rows, against the real Stake program.

Every figure is a worst case, and the tests say so: a batch whose members shared
a relay would name fewer distinct keys and fit more. They are also ceilings per
transaction, not per epoch — settlement is permissionless and a busy pool
settles in several batches, at the cost of several timestamps rather than one.

## Synchronised actions are the point

The brief asks for an anonymity set for *behaviour*, not for funds. That
distinction is load-bearing here, so it is tested rather than asserted:

```
settled 4 real CPI actions in one transaction, 40251 CU
```

That figure moves by a few thousand between runs — the accounts are generated
fresh each time and `find_program_address` searches a different number of bumps
to derive each record's address — so what the test *asserts* is the property
rather than the number: four CPI actions and their payouts stay under 60,000 CU,
comfortably inside one instruction's default budget.

`a_crowd_of_members_perform_a_real_protocol_action_together` seeds a pool,
loads the **real SPL Memo program** into the SVM, and has four members each
prove membership and request the identical memo. Settlement invokes Memo four
times in a single transaction, every invocation made by the pool and funded from
its vault.

What an observer holds afterwards is four identical memos, one timestamp, one
signer, and no field anywhere in the transaction that distinguishes which member
asked for which. The actions are indistinguishable **by content** because the
payloads match, and indistinguishable **by timing** because settlement gives them
one clock.

Three properties make this a behavioural tool rather than a mixer with a CPI
bolted on:

- **The target is arbitrary.** Selector one invokes any program with any payload.
  Memo is used in the test because it is small, real, and validates its own
  account list.
- **The pool can sign as your authority.** Selector two hands the pool's vault to
  the callee as a *signer*, which is what a stake delegation or a governance vote
  needs and what a transfer does not: somebody must sign as the authority, and
  for a member who must never appear on chain, that somebody can only be the
  pool. `the_pool_signs_an_action_as_its_own_authority` proves it under
  **50,000 CU** against real SPL Memo — a program that refuses any account handed to it that
  has not signed, and that names its signers in its logs. The test reads that log
  for the vault's own key, so the claim rests on someone else's program. Devnet
  carries the case this exists for: a **real stake delegation**, with the pool as
  the staker authority — [see Deployment](#deployment).
- **Moving lamports is the degenerate case.** Selector zero is a plain transfer,
  kept only because expressing "pay this account" should not require a target
  program.

The two invoke selectors exist because of an ordering constraint that was
measured rather than assumed. Selector one funds the beneficiary *before* the
call, so the target sees the value; the runtime then refuses to let the vault
cross the CPI boundary, because this program has already moved its lamports by
direct mutation. Selector two pays *after*, leaving the balance untouched at the
moment of the call, which is exactly what lets the vault be a signer. Both
orderings cannot hold at once, so the member chooses, and the selector is inside
the action binding — a settler cannot obtain the pool's signature for a proof
that did not ask for it.

The constraint turned out to be about the **batch**, not the spend, and a live
cluster is what found it. The runtime objects to any lamport this program moved
anywhere in the same instruction, so three transfers settled ahead of a signed
call are enough to break it — a case a single-spend test cannot reach.
Settlement runs every signed call first and pays everybody afterwards.
`a_signed_action_settles_inside_a_batch_of_plain_transfers` pins it, because a
signed action that could only settle alone would have to wait out the timeout
rather than join a batch, giving up the shared timestamp that is the reason to
stand in a crowd.

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

**Every input is somebody else's chain data.** Nothing here is simulated, and
that distinction is load-bearing rather than stylistic: an anonymity number
computed from a protocol's own parameters is a statement about arithmetic — it
comes out however the formula says it must, and a hostile reviewer can derive it
without running anything. The interesting question is what a real funding graph
does to a real anonymity set, and the only way to answer it is to go and read
one. So `mirror collect` is the one networked command, it points at pools this
project does not control, and the artifacts it produced are committed so the
analysis can be rerun offline against exactly the bytes that produced the
headline.

Four commands, two passes, and the split is the point:

```
mirror seeds   --program <program-id>  # member-weighted frame, one row per depositor
mirror collect --seeds seeds.txt    # the only networked step; writes sample.json
mirror analyze --sample sample.json # pure, offline, deterministic
mirror compare --sample a.json --against b.json   # is the difference real?
mirror selection --earlier lo.json --later hi.json # are the unresolved missing at random?
```

`data/sample-privacycash-run6.json` is the committed artifact behind the
headline. Seven samples are committed in all, and they are what make each
correction checkable rather than merely described:

| file | run | what it is |
|---|---|---|
| `sample-smoke.json` | 1 | the size-biased frame. 12 attempted, **0 resolved** |
| `sample-privacycash.json` | 3 | member-weighted, smaller budget |
| `sample-privacycash-run4.json` | 4 | bigger budget, broken tracer |
| `sample-privacycash-run6.json` | **6** | **the headline**, tracer fixed |
| `sample-marinade.json` | 5 | the control, broken tracer |
| `sample-marinade-run7.json` | 7 | the control, tracer fixed |
| `sample-marinade-run8.json` | 8 | the control at double the page cap |

`mirror analyze` on any of them reproduces that run's numbers offline, and
`mirror selection` on a pair reproduces the missing-at-random check. Run 1's
sample is committed too, and it is the one that resolved nothing — a failed run
is evidence about the method and is kept as such.

**Run 2 is the one exception**: it came back above the 1% RPC-failure limit and
its sample is not committed. Saying so is part of the record.

**Every run since Run 4 had its budget and its prediction committed to git
before the collection started**, so the parameters are declarations rather than
descriptions, and the predictions are on the record including the ones that
turned out wrong.

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
- **The oldest transaction is where that search starts, not where it ends.** An
  address often appears in someone else's transaction — an ATA creation, a
  multisig setup — before it is ever funded, so its oldest transaction credits it
  nothing. We got this wrong: the tracer read that one transaction and reported
  "no incoming edge", turning *we stopped reading* into *there is nothing there*
  for wallets with eleven thousand transactions. Found while building a control
  population, fixed, every affected number re-collected, and written up in
  `docs/MEASUREMENT_LOG.md` rather than quietly repaired.
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

### Two uncertainties, and the bias that runs against us

A `ρ` is reported with **two** intervals, because they answer different
questions and neither covers the other:

- the **unresolved bracket** — what if the members we could not resolve had all
  been one class, or all been distinct? An exact bound.
- the **sampling interval** — these depositors are a draw from a much larger
  population, so how much of `ρ` is the draw? A bootstrap over members, 10,000
  replicates at a published seed, so a reader recomputes our interval and not
  merely a similar one.

And a bias that no interval fixes: plug-in entropy is biased low when classes
are many and members few, which is every run here. Since `ρ = 2^−H(C)`, that
means **every ρ we publish is biased high — the pools plausibly leak less than
we report.** It is stated because it is the direction that makes our own number
look worse, and a bias disclosed only when it flatters is not a disclosure.

That bias is also why `compare` exists rather than a subtraction. It falls the
same way on every population measured the same way, so a *difference* survives
what neither absolute number does — and when the interval on the difference
contains zero, the tool says the two are indistinguishable and declines to rank
them.

### The assumption underneath all of it, tested

Dropping unresolved members is only harmless if the ones that resolve are a fair
draw of the classes present. Every bracket and every comparison rests on that,
and it is normally asserted and left alone. It is testable: collect one frame at
two budgets, split the resolved members into *cheap to trace* and *expensive to
trace*, and ask whether the two groups have the same class distribution.

| population | pair | cheap | expensive | difference, 95% |
|---|---|---|---|---|
| privacy pool | depth 8/cap 8 → 16/24 | 39, ρ 0.1179 | 15, ρ 0.1250 | −0.1507 … +0.0704 |
| staking control | same budget, tracer fixed | 16, ρ 0.0743 | 22, ρ 0.0585 | −0.0144 … +0.0786 |

Neither separates: at this margin, being resolvable does not pick out particular
provenance classes.

**Only the first row is a budget margin**, and the second is weaker than it
looks. The two staking runs used identical parameters — depth 16, page cap 24 —
and differ by the tracer fix rather than by budget, so its *expensive* group is
"members the broken tracer failed on" rather than "members a smaller budget
could not reach". It is a real check on whether that bug selected for particular
classes, which is worth knowing, and it is not a second budget margin. The
manifests in `data/` carry the parameters, so this is checkable rather than
taken on trust.

Evidence, not proof, in either case — the first row speaks for the members just
beyond a cheaper budget, not for those beyond the larger one.

Had it separated, that would have been the more important result, and it would
have invalidated the cross-population comparison outright rather than merely
delaying it.

### The comparison we ran, and refused

`ρ` is the headline because it is comparable across populations, so we measured
one that is **not seeking privacy at all** — Marinade staking depositors,
identical pipeline, identical parameters — to give the number a scale.

```
population       members   rho      resampled 2.5-97.5%   bias
privacy pool          54   0.0955   0.0848 .. 0.1790     +0.0276
staking control       38   0.0362   0.0463 .. 0.0708     +0.0203

difference: +0.0592   95% +0.0259 .. +0.1229
```

The interval on the difference excludes zero, and it points the way this
project's argument wants: the privacy pool reads as the more concentrated
population. **The tool refuses to report it**, because the control resolves 42
of 92 members and what survives is its *traceable* subset.

The gate that refuses this did not exist when `compare` was written, and was
added after seeing the control fall on the wrong side of it. So: **we built the
check that refuses our own favourable result, after learning the result was
favourable.** Both the code and that sentence are in the repository.

Then we spent a run trying to clear it, having predicted in advance — in the
committed log, knowing which answer suited us — that a doubled budget would get
the control above half. **It did not.** Resolution went 38 → 42 against the 46
needed, and the prediction is on the record as wrong.

That failed run produced the more interesting number anyway: **311 RPC calls per
additional resolved member, against 50 for the population as a whole.** The
control is not under-resolved because we were stingy. Its remaining members have
genuinely long funding chains, and the *staking* pool — where nobody wants
deniability — turns out to be markedly harder to trace than the privacy pool,
which resolved at 65% for two-thirds of the cost per member (33 RPC calls
per resolved member against 50). We do not claim to know
why, and the explanation that would flatter us is one of the two candidates,
which is exactly why we are not asserting it.

The comparison is reported as **unachieved**. There was no further run: the
stopping rule was fixed before collection, and a stopping rule that bends when
the result is close is not one.

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

Live on **devnet** at
[`8H3cYoiAA9LM36cyPr4UEv38dhHasSu2XPSdiBfyrLEa`](https://explorer.solana.com/address/8H3cYoiAA9LM36cyPr4UEv38dhHasSu2XPSdiBfyrLEa?cluster=devnet).
Every signature below is a link — the claims in this section are meant to be
read off the cluster rather than off this page. The whole
lifecycle ran there against a real validator — pool creation, deposits, spends
each carrying a Groth16 proof verified by the deployed program's own syscall, and
a settlement that closed the vault to its rent-exempt minimum to the lamport.

That settlement carried five spends, and **two of them were not transfers**. One
was a memo the pool signed. The other was a **real stake delegation**:

```
Delegated Stake:        1.09771712 SOL, activating
Delegated Vote Account: 2f9C9AU8nFRKUub8NHToNiZzcwmYiNeipVuP8akKgRVv
Stake Authority:        CWxsJdxBLm3LC6dEnBco68a6T4QNEF31N3qQRy95wN3Q   ← the pool's vault
Withdraw Authority:     H9DRVAD42eiQqmXeYrX5AWwoYRr4wie4MzvJDC6Wkxmn   ← the operator
```

`DelegateStake` requires the staker authority to sign, and no member can be that
authority without appearing on chain and undoing the point. So the pool was, and
the pool signed. That is the whole design in one transaction: an observer sees
that a delegation happened, to which validator, for how much, and **cannot say
which of the five members asked for it**.

The withdraw authority is deliberately *not* the pool, and that is the threat
model applied rather than restated: the pool's signature is available to every
member, so an authority the vault holds is one every member holds. Delegation
survives that — the worst a member can do is re-delegate somebody's stake.
Withdrawal does not.

Evidence never rests on our own word. Memo names its signers and named the
vault; the stake account is read back after settlement and only reaches the
`Stake` variant by being delegated. The soak checks both against the cluster and
fails the run if either is absent, so `docs/PROOF.md` cannot carry these claims
without them being true.

That file has every signature *and* the lamports, because "closed to the
rent-exempt minimum" is the interesting part of that sentence and a list of
signatures does not show it: 100,000,095 owed against 100,000,095 paid out, and a
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
  and insecure is a coherent position for an unaudited protocol; the incoherent
  one is a setup that is *both* insecure and unreproducible, which is what
  publishing the entropy while withholding the proving key produces — nobody can
  verify the key, and nobody can regenerate it either.
- The on-chain `k_floor` bounds **program-visible membership** only. That is all
  a program can check.
- **Not that privacy pools attract more concentrated funding than ordinary
  users.** We measured a control to find out, the point estimates say they do,
  and the sample does not support saying it. `ρ`'s comparability across
  populations is demonstrated as machinery and unproven as a finding.
- Not audited.

`docs/MEASUREMENT_LOG.md` records every collection run, including the three the
tool itself refused to publish a headline from — twice on the failure gate, once
on the bracket — and the one whose pre-registered prediction turned out wrong. A
measurement project that keeps only its successful runs is selecting rather than
reporting.

## Documentation

| | |
|---|---|
| `docs/ARCHITECTURE.md` | The design, and why each decision is what it is. |
| `docs/PROVENANCE_METHOD.md` | Adversary model, metrics, sampling, the honest-claims analysis. |
| `docs/GROTH16_INTEGRATION.md` | The arkworks-to-Solana byte layout, verified by execution. |
| `docs/MEASUREMENT_LOG.md` | Every run. |
| `docs/THREAT_MODEL.md` | The adversary, what holds, and every place it stops. |
| `docs/PROOF.md` | Devnet signatures for every flow, and the rejections. |
| `docs/CROWD.md` | Six members delegating to six different validators in one devnet transaction, and what divergence costs. |
| `docs/USAGE.md` | The member-facing commands, end to end, with real devnet output. |
| `docs/PLAN.md` | The design as decided, and where the shipped protocol departs from it. |
