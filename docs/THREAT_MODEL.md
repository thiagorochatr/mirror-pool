# Threat model

## What this protects

An observer sees that an action happened and cannot say which member asked for
it. That is the whole claim, and everything below is either how it is achieved or
where it stops.

## The adversary

Passive, global, retrospective, chain-only. They read every transaction ever
made, run any analysis they like over it, and never need to compromise a key or
a machine. They may also label addresses using off-chain knowledge — exchange
deposit addresses, published attributions, their own records.

They cannot break BN254 discrete log, invert Poseidon or find keccak collisions.

## What holds

**Deposit and action are unlinkable through the proof.** A spend proves
membership in the accumulator without naming a leaf. Against an adversary who
sees only the chain, the posterior over which member acted is uniform on the
pool's notes.

**No member key appears on chain.** Spends are relay-signed and settlements are
settler-signed. A member's wallet never touches the protocol after depositing.

**A relay cannot alter what was authorised.** The action binding covers the
selector, the target program, the beneficiary, the relay fee, the declared
account count and the payload, under a domain tag, and is recomputed on-chain
rather than transmitted.

**Actions carry one caller.** Every action is invoked by the pool program on a
member's behalf and funded out of the pool's vault, so the on-chain trace of a
stake made through the pool is identical whoever asked for it. The vault
authorises the CPI through its seeds but is never one of the callee's accounts —
see below.

**Payouts share a timestamp.** Settlement batches, so arrival time does not
separate members within a batch.

**Escrow cannot be drained.** The denomination is a pool constant and nullifiers
are spend-once, so `vault ≥ denomination × outstanding` is a function of two
counters that nothing a prover supplies can influence.

**Nothing can hold a member's funds.** A member spends as their own relay and
settles their own batch after the timeout. No key's absence freezes anything.

## What does not hold

### Funding provenance — the open channel

An adversary can partition members by where their capital came from. Learning a
member's class leaves only that class to guess within, so the anonymity that
survives is the size of the class rather than `k`.

**This is not closed and cannot be by a better circuit.** No deposit pool
controls where its users' money came from. What we do instead:

- the *action* side is closed, because actions execute from the pool's vault PDA;
- the *membership* side is measured from real chain data, and the method, the
  failure census and the sampling frame are published beside the number.

`k_floor` bounds program-visible membership only. That is all a program can check.

### The trusted setup is not secure

It is *reproducible*, which is a different and lesser property. The seed is
public, so the toxic waste is public, so **proofs are forgeable by anyone who
runs the setup**. `mirror verify-setup` lets a third party re-derive the deployed
key and confirm it matches this circuit — that is what reproducibility buys, and
it is worth having, but it is not security.

Production needs a multi-party ceremony. The scaffolding for one is not in this
submission and we do not claim it is.

### The program is upgradeable

Whoever holds the upgrade authority can replace the code, including with code
that steals escrow. This is a named trust assumption, not a property.
`solana program set-upgrade-authority --final` removes it and correspondingly
removes the ability to fix anything.

### The pool cannot be one of its own action's accounts

The vault authorises an action through its seeds, but it is never one of the
callee's accounts. It cannot be: settlement moves the payout out of the vault by
direct mutation before invoking, and handing that same account to a callee makes
the runtime reconcile those lamports across the CPI boundary and reject the whole
instruction as unbalanced — whether the account is marked writable or not.

So an action whose target needs the pool itself as an account is not expressible
in this version. Value reaches an action through the beneficiary instead. A
settler that tries to place the vault in the action's account list is refused
rather than left to fail later.

### The action's account list is chosen by the settler

The proof binds the selector, the target program, the beneficiary, the relay
fee, the payload and **how many** accounts the action takes. It does not bind **which** accounts fill
those slots — settlement is permissionless, so whoever settles picks them.

For a target whose destination is instruction data this changes nothing. For a
target whose destination is an *account* — an SPL token transfer, for instance —
a settler could point the action at accounts the member did not choose. The
member's own escrow is not at risk, because only this program can debit the
vault, but anything the action itself would move is.

Binding an account-list commitment into the proof would close this. It is not
implemented, and the claim elsewhere that "a relay cannot redirect an action" is
about the selector, target, fee and payload, not about the account list.

### Timing at submission

Settlement batches payouts, but `submit_spend` is a transaction at a time of the
relay's choosing. A relay that submits immediately on request leaks the member's
timing. This is relay policy, not a protocol guarantee, and we do not claim
otherwise.

### Amounts are public

Fixed denominations mean the amount is a pool constant rather than a secret. A
member who needs an unusual amount is identifiable by the pool they chose. There
is no confidential-value layer here.

### Small crowds

`k_floor` is enforced against notes in the tree, and a pool whose deposits are
mostly Sybils of one actor has a large nominal `k` and a small real one. The
entry fee prices set inflation; it does not prevent it. The provenance
measurement is the honest reading of what the set is worth.

### Not audited

No external review. The end-to-end suite runs against the compiled program on a
real SVM and every negative case but one carries this program's own error code —
the truncated-account-list case is caught by the runtime before the program can
rule on it — which is
evidence of behaviour and not a substitute for an audit.

## Deliberate non-goals

**Hiding funds.** The brief asks for behavioural deniability, not a mixer, and a
value-mixing layer is not here.

**Defeating an adversary with off-chain data.** Someone who knows a member
deposited — because they watched them do it, or because the member told them —
is not in scope. The proof hides which member acted, not that a given person is
a member.

**Compute-optimal proving.** The circuit is three public inputs at about 91k CU
because that is what binding everything that matters costs. It has not been
tuned further.

## Claim language

Words this project does not use about itself: *anonymous*, *untraceable*,
*unlinkable* without a named adversary and a stated population. Every quantitative
claim names the data it came from, and `docs/MEASUREMENT_LOG.md` records the runs
that produced nothing alongside the ones that did.
