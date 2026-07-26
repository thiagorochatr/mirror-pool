# Live-cluster evidence

Every line here is a transaction that landed. Nothing is simulated: the proof is produced by the host prover and verified by the deployed program's own Groth16 syscall.

- cluster: `devnet`
- program: `8H3cYoiAA9LM36cyPr4UEv38dhHasSu2XPSdiBfyrLEa`
- pool: `AM7MrJGDWj83poTTRK4gVpCuGgybUKzNvZRW41x3yCfJ`
- vault: `3x8NUzCzQdbjEX2T7gapDAgg6nJU8Y3ZsbUFPnqfhyLV`

## Flows

| step | signature | note |
|---|---|---|
| init_pool | [`nbk64A6kCYWh3yJKvZYrqKzNjQfFHnMREDqnbBMZGPRiJMch4TZXVMd3LLYozVKSbPYoMsgtUvJbo1yrKShQFQh`](https://explorer.solana.com/tx/nbk64A6kCYWh3yJKvZYrqKzNjQfFHnMREDqnbBMZGPRiJMch4TZXVMd3LLYozVKSbPYoMsgtUvJbo1yrKShQFQh?cluster=devnet) | denomination 20000017, k_floor 4 |
| deposit | [`4rh44zFpUrdyPaoNaiLDoKddzszpKwUGM3PmGX7XSF2StbJUYkjzu5k9NpCXhY9NWFuN4Ww1jxpGuSs5m6bU9QFu`](https://explorer.solana.com/tx/4rh44zFpUrdyPaoNaiLDoKddzszpKwUGM3PmGX7XSF2StbJUYkjzu5k9NpCXhY9NWFuN4Ww1jxpGuSs5m6bU9QFu?cluster=devnet) | note 1 |
| deposit | [`3mogP7EotT5F6s6B9vrdWzDSxs2WEskDk6GSzWK8vw8N1hpaEmX1KJ4nBrEgzBaNUCkqcxNdkEnAx6Td54SgRNa4`](https://explorer.solana.com/tx/3mogP7EotT5F6s6B9vrdWzDSxs2WEskDk6GSzWK8vw8N1hpaEmX1KJ4nBrEgzBaNUCkqcxNdkEnAx6Td54SgRNa4?cluster=devnet) | note 2 |
| deposit | [`4rj33jkYepFCZfEpncBBjeNBp7DS1LA9priRMsdPWpSMBiJL218g9rMwAD8WUwnX1pJ5GuvQEsve6Q417poK3CCL`](https://explorer.solana.com/tx/4rj33jkYepFCZfEpncBBjeNBp7DS1LA9priRMsdPWpSMBiJL218g9rMwAD8WUwnX1pJ5GuvQEsve6Q417poK3CCL?cluster=devnet) | note 3 |
| deposit | [`3YUQ7upP47RjUujKPe1Tcgv4ods1W8mCsV7jpWcqmRSEaU5GgRiivxHX8pFJmmgSgJ9ZpTfNGH9RCdeXhhYGpL47`](https://explorer.solana.com/tx/3YUQ7upP47RjUujKPe1Tcgv4ods1W8mCsV7jpWcqmRSEaU5GgRiivxHX8pFJmmgSgJ9ZpTfNGH9RCdeXhhYGpL47?cluster=devnet) | note 4 |
| submit_spend | [`4xkzxNGCqUo3Qu2TqoG15e6ZDWR1H9edad3V88ox3oYtmAAEKbJ4uzP5qkg7cfJaHbA6uREiLpmQWFU4h8L12CpH`](https://explorer.solana.com/tx/4xkzxNGCqUo3Qu2TqoG15e6ZDWR1H9edad3V88ox3oYtmAAEKbJ4uzP5qkg7cfJaHbA6uREiLpmQWFU4h8L12CpH?cluster=devnet) | note 0, relay-signed |
| submit_spend | [`49H8PgT13fUC9gNXJWaJ7ui3hBqwngPunwvTH4wFicNGe8BdGGKmbivorApnSE8JKT3VBfhwFc2nY8XMJ5RczmdV`](https://explorer.solana.com/tx/49H8PgT13fUC9gNXJWaJ7ui3hBqwngPunwvTH4wFicNGe8BdGGKmbivorApnSE8JKT3VBfhwFc2nY8XMJ5RczmdV?cluster=devnet) | note 1, relay-signed |
| submit_spend | [`2c5AyyspfzfBsJ8mW2LYKuUE8zTDx5wxFxheWZxjVyU6Lc7LtqqBcsgT8vJvurMAoPQetpiELBnxVGbMRnvqqoCk`](https://explorer.solana.com/tx/2c5AyyspfzfBsJ8mW2LYKuUE8zTDx5wxFxheWZxjVyU6Lc7LtqqBcsgT8vJvurMAoPQetpiELBnxVGbMRnvqqoCk?cluster=devnet) | note 2, relay-signed |
| submit_spend | [`4129ftZy7XoZ5vZLMtCZgoXsfphBAQr5fNtKNNJaViVZMfmbUS9SN5F8owbqdYaGTMBZTNVx7e3hCeyKXHcbERou`](https://explorer.solana.com/tx/4129ftZy7XoZ5vZLMtCZgoXsfphBAQr5fNtKNNJaViVZMfmbUS9SN5F8owbqdYaGTMBZTNVx7e3hCeyKXHcbERou?cluster=devnet) | note 3, relay-signed, action: pool-signed CPI to SPL Memo |
| settle_epoch | [`3wPUYgdYdC9AXfL1XYXeHP9YMWMJeZvGhBpq91qWRH6ZWQc4fmJkykjgzAR2GdUrUFumswNHKLjyEdewHW14EJPj`](https://explorer.solana.com/tx/3wPUYgdYdC9AXfL1XYXeHP9YMWMJeZvGhBpq91qWRH6ZWQc4fmJkykjgzAR2GdUrUFumswNHKLjyEdewHW14EJPj?cluster=devnet) | 4 spends in one transaction, 1 of them a CPI the pool signed |
| deposit | [`3L3qxxDpNf9rHRqXhf62xRfrDEZQEDeY52vmAti6gn3dokUk2HnmrpXoiUPHS7RKt1Fnz8vPPBfcNRBHmtGjDV2w`](https://explorer.solana.com/tx/3L3qxxDpNf9rHRqXhf62xRfrDEZQEDeY52vmAti6gn3dokUk2HnmrpXoiUPHS7RKt1Fnz8vPPBfcNRBHmtGjDV2w?cluster=devnet) | note 5 |

## Vault accounting

The accounting invariant is a statement about the vault's lamports, so here are the lamports. The vault holds escrow and carries no data, which is what makes its floor the rent-exempt minimum for a zero-byte account — read from the cluster during the run, not assumed.

| quantity | lamports |
|---|---|
| denomination | 20000017 |
| notes settled | 4 |
| relay fee (taken out of the denomination, not added) | 200000 |
| vault before settlement | 80890948 |
| vault after settlement | 890880 |
| rent-exempt minimum, 0 bytes | 890880 |
| **paid out** | **80000068** |
| **owed** (denomination × notes) | **80000068** |

Paid out equals owed, and the vault came to rest on its floor with a remainder of 0 lamports. The soak asserts both and fails the run otherwise, so this table cannot record a discrepancy and still exit successfully.

## The pool signed an action, and the callee said so

The settlement above carried four spends, and one of them was not a transfer: the pool invoked SPL Memo as that member's **authority**, in the same transaction as the other three. That is the capability a stake delegation or a governance vote needs and a payment does not.

A signature only proves the transaction landed. It says nothing about who signed the instruction the pool made *inside* it, so the evidence has to come from the callee. SPL Memo refuses any account handed to it that has not signed, and names the ones that did:

```
Program log: Signed by 3x8NUzCzQdbjEX2T7gapDAgg6nJU8Y3ZsbUFPnqfhyLV
```

That is the pool's vault, `3x8NUzCzQdbjEX2T7gapDAgg6nJU8Y3ZsbUFPnqfhyLV`, which has no private key — it signed through seeds only the program holds. The soak reads this line back from the cluster and fails the run if it is absent, so this section cannot appear without the callee having said it.

## Rejections

A negative case is only evidence if the program's own error code is what rejected it. A bare runtime failure proves nothing about this program.

| case | error code | what the rejection establishes |
|---|---|---|
| replayed nullifier | `0xf` | a nullifier is spent once, ever -- not once per epoch |
| redirected beneficiary | `0x12` | a relay cannot send a member's payout somewhere the member did not choose |
| inflated relay fee | `0x12` | a relay cannot re-price the work after the member authorised it |

The last two attack a note that is still live, deposited after settlement precisely so that they would have to. Against an already-spent note the replay guard fires first and the rejection would say nothing about the check under test — which is how a negative case comes to pass for the wrong reason.

## Scope

Devnet is devnet. This is a live-cluster functional proof, not a claim of mainnet operation or of an anonymity crowd: the run is scripted by one operator with a handful of notes. What it establishes is that the circuit, the prover, the byte layout and the deployed program agree on a real validator.
