# Using the pool

Every command below was run against devnet to produce the output shown. Nothing
here is illustrative.

The tool needs no server, no indexer and no account with anybody. It reads what
it needs from the chain each time, which is why a member can act from a machine
that has never seen this pool before, carrying nothing but their note file.

```
cargo build --release        # the binary is target/release/mirror
```

Every command defaults to `--url https://api.devnet.solana.com` and to the Solana
CLI's own keypair at `~/.config/solana/id.json`, so the common case needs neither
flag.

## The shape of it

```
note-new  →  deposit  →  ( wait for a crowd )  →  spend  →  settle
```

A note is a secret you generate locally. Depositing it puts its *commitment* into
the pool's tree and escrows the denomination. Later you prove you own some note
in that tree — without saying which — and the pool acts. Settlement executes
everybody's action together.

## 1. A pool

One pool per denomination, globally, and anybody can create one. Splitting
deposits of the same size across two pools splits the anonymity set, and a split
set is worse for everyone in it.

```
$ mirror init-pool --program 8H3cYoiAA9LM36cyPr4UEv38dhHasSu2XPSdiBfyrLEa \
                   --denomination 31000001 --k-floor 2
pool    DQ17r5reCu5P4efUzHQZq7Ye6D6vt72ThxatNdLBqcKt
vault   81odk482H6VpRQMk8tQaoThYr2ffJtugcJXjmKX2faco
signature 3Tbf4zMGDHW89bi3F7yhaGMAB6RqveDi5WTJAYKEe3Zr2F8eTAeT8zSTp6V3B4kWE4ZSXmpMdxpx6UayvdsiNNiw
```

`--k-floor` is the number of notes the pool must hold before it will act at all.
It bounds *program-visible* membership, which is all a program can check — see
`THREAT_MODEL.md` for what it does not bound.

## 2. A note

```
$ mirror note-new --denomination 31000001 --out m1.json
wrote m1.json
  denomination 31000001
  commitment   1ca8f208a35a6327c7871ee6d38268527260f53299c21b25133b32178989701c

This file is the note. Treat it exactly as you would a keypair:
anyone holding it can spend the deposit, and losing it loses the
deposit with no way back.

Next: mirror deposit --note m1.json
```

The file is plain JSON and unencrypted, deliberately. Encrypting it would put a
password between you and your funds and imply a security property this tool
cannot deliver by itself. Back it up the way you back up a keypair.

It will not overwrite an existing note file, and there is no `--force`: an
overwrite here destroys a deposit.

## 3. Deposit

```
$ mirror deposit --program 8H3cYoiAA9LM36cyPr4UEv38dhHasSu2XPSdiBfyrLEa --note m1.json
deposited 31000001 lamports into DQ17r5reCu5P4efUzHQZq7Ye6D6vt72ThxatNdLBqcKt
leaf      0
signature JpCxFU4ipaBap15AWRxNVFoy8rskWZnUTFoBTLx1JBs5V2TKDCx7HhhzQ4AtYxL46yvfS8U1yXzSVLNg2au89hS
```

The deposit is public and it is signed by you. That is fine and unavoidable —
what the pool hides is not that you joined, but which member later acted.

The escrowed amount is read from the pool, never from your instruction, so a
deposit cannot claim a size the pool did not set.

## 4. Check the tree — optional, and worth doing once

```
$ mirror tree --program 8H3cYoiAA9LM36cyPr4UEv38dhHasSu2XPSdiBfyrLEa --denomination 31000001
pool          DQ17r5reCu5P4efUzHQZq7Ye6D6vt72ThxatNdLBqcKt
vault         81odk482H6VpRQMk8tQaoThYr2ffJtugcJXjmKX2faco
denomination  31000001
k floor       2
notes         2 deposited, 0 settled, 2 outstanding

rebuilding the accumulator from chain history:
  3 transactions touched this pool

  leaves recovered  2
  pool reports      2
  rebuilt root      0c77cb909067c1a57811be8c05237aff2715c65c6250fdde764ed768096cd732
  on-chain root     0c77cb909067c1a57811be8c05237aff2715c65c6250fdde764ed768096cd732

  the rebuilt tree matches the chain — proofs built from it will verify
```

This is the piece the rest stands on. The program stores only the accumulator's
*frontier* — one node per level — which is enough to append a leaf and produce a
root, and not enough to prove any particular leaf is in the tree. The leaves are
not lost: every commitment was an argument to a `Deposit` instruction, so the
whole set is in the transaction history in insertion order.

Rebuilding it and finding the same root the program holds proves the recovered
set is complete and correctly ordered, which is exactly the precondition a
membership proof needs. `spend` does this rebuild itself; this command just lets
you watch it.

## 5. Spend

**The relay signs, never you.** A member who pays their own fee signs with their
own wallet, and `accountKeys[0]` is then the member — which publishes the link
the pool exists to break. Use a key that is not your wallet and has no history
with it.

```
$ mirror spend --program 8H3cYoiAA9LM36cyPr4UEv38dhHasSu2XPSdiBfyrLEa \
               --note m1.json --to BQ6piLqJD4CWn4V3VyDBh3RpU1FtsPh63P9y4wtwBxJg \
               --relay relay.json --relay-fee 100000
rebuilding the accumulator so this note can be proved a member:
  3 transactions touched this pool
  this note is leaf 0 of 2
deriving the proving key from the published seed (this takes a moment)
proving membership

submitted. The note is spent and the action is recorded.
  nullifier 02218a1c2f8fe73a88e856ee9cf3c7e01508107490dca23a56af54166566d54e
  record    59jcYijM67FYybt6MH36CU9YUMqQQ7Q5Lg6JiTXRXxfz
  signature d5Y6CxbrNxu74u88AGT5NU1KqfLb4JgjtBXzT2TyXyvAGqJq2qgf5NftQwKoAH5TfHdh3fTQcbjkwudSDX8tj7U

Nothing has been paid out yet — settlement executes the batch, which is
what gives every member's action one timestamp. Run `mirror settle`, or
wait for anyone else to.
```

The relay fee comes *out of* the denomination, never in addition to it, and a fee
at or above the denomination is refused on-chain.

### Doing something other than paying

The interesting case is not moving lamports. `--invoke` calls any program with
any payload:

```
mirror spend ... --invoke <program> --payload <hex> --accounts <n>
```

`--accounts` is how many accounts the call takes, and it is bound into the proof
so a settler cannot add or drop one. Which accounts fill those slots is chosen at
settlement — a real limit, stated in `THREAT_MODEL.md`.

Add `--pool-signs` to hand the pool's vault to the callee as a **signer**, so the
pool acts as your authority rather than only as your funder. That is what a stake
delegation or a governance vote needs and a payment does not; `PROOF.md` has a
real stake delegation done this way on devnet.

## 6. Settle

Permissionless. Anyone can settle, so no operator's absence can strand you, and
you can always settle your own batch.

Below the crowd floor it tells you what it is waiting for rather than failing
with an error code:

```
$ mirror settle --program 8H3cYoiAA9LM36cyPr4UEv38dhHasSu2XPSdiBfyrLEa --denomination 31000001
looking for spends waiting to settle:
  4 transactions touched this pool

  1 spend(s) pending, and this pool's floor is 2.
  A batch below the floor may settle once every spend in it has waited 3600s;
  the youngest has waited 16s, so 3584s remain.

  This is a liveness guarantee rather than a restriction: nobody can
  hold a member's funds waiting for a crowd that never arrives.
```

Once the crowd is there:

```
$ mirror settle --program 8H3cYoiAA9LM36cyPr4UEv38dhHasSu2XPSdiBfyrLEa --denomination 31000001
looking for spends waiting to settle:
  5 transactions touched this pool

settled 2 spends in one transaction
  signature 4EA7SszoGBWC1EKzgGn8XdLWvhMiWxp6v6Gbcmd7czzn7LfiUmoBvKwAJsM3XPt4Y2abm5P85M7w939ftRTvi6XC

Every payout in that batch shares one timestamp and one ordering, which
is what stops arrival time from telling the members apart.
```

Both beneficiaries received `31000001 − 100000 = 30900001` lamports, in one
transaction, at one timestamp.

`settle` executes plain transfers on its own. A pending CPI action is left alone
and reported, because the record binds *how many* accounts the call takes and
never *which*, so no tool can infer the account list its callee expects.

## Escaping without a relay

There is no `self-spend` command because none is needed. Relay for yourself with
`--relay-fee 0` and settle your own batch once the timeout passes. The cost is
the expected one — your own wallet signs, so you give up the anonymity the relay
path provides. It is an escape hatch, not a mode of operation, and it exists so
that nothing in the protocol can hold your escrow.

## What can go wrong

| | |
|---|---|
| `no pool exists at …` | Nobody has created that denomination yet. `init-pool` does, permissionlessly. |
| `this note is not a leaf of the pool` | The note was never deposited, or it belongs to a different denomination. |
| `this note file is inconsistent` | The file was edited or corrupted. Spending it would burn a nullifier against a leaf that is not in the tree, so it is refused first. |
| `the rebuilt root does not match the pool's` | The endpoint's history is incomplete — usually pruning. `mirror check-endpoint` tests for exactly that. |
| `this pool holds N notes and its floor is M` | The pool refuses to act below its floor. Wait for more members, or use a pool with a floor you can meet. |
