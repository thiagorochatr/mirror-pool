# Live-cluster evidence

Every line here is a transaction that landed. Nothing is simulated: the proof is produced by the host prover and verified by the deployed program's own Groth16 syscall.

- cluster: `devnet`
- program: `8H3cYoiAA9LM36cyPr4UEv38dhHasSu2XPSdiBfyrLEa`
- pool: `5R5mrXeginkRYwv5NmXcSVVLpUW2obuc7YwdEBt31x5n`
- vault: `CWxsJdxBLm3LC6dEnBco68a6T4QNEF31N3qQRy95wN3Q`

## Flows

| step | signature | note |
|---|---|---|
| init_pool | [`5x5wbVjg1g43KMCZQ8weY5eEjxfLizdjAiSvHVX7kA3bc1yRkKm4pAwWquzoCwwWzkxxaiTfAGoXyT2qi5n129hD`](https://explorer.solana.com/tx/5x5wbVjg1g43KMCZQ8weY5eEjxfLizdjAiSvHVX7kA3bc1yRkKm4pAwWquzoCwwWzkxxaiTfAGoXyT2qi5n129hD?cluster=devnet) | denomination 20000019, k_floor 4 |
| deposit | [`5NWeHxd2QLKskpPTPQhJJk5vuav8VojN1vw8PKwz8F3P2xXfjxfaAnScM57SqVRXv2HTNNsKpMdbZJCFVt63ppmZ`](https://explorer.solana.com/tx/5NWeHxd2QLKskpPTPQhJJk5vuav8VojN1vw8PKwz8F3P2xXfjxfaAnScM57SqVRXv2HTNNsKpMdbZJCFVt63ppmZ?cluster=devnet) | note 1 |
| deposit | [`2vNqo1GEEmRBpPCUUh8kiXuDUMz5ZkKNmJHKJfGPaHdW8jgDS9f3g3dTNUJsdEXtoRUXmSGCH8G7XURYqus3EBho`](https://explorer.solana.com/tx/2vNqo1GEEmRBpPCUUh8kiXuDUMz5ZkKNmJHKJfGPaHdW8jgDS9f3g3dTNUJsdEXtoRUXmSGCH8G7XURYqus3EBho?cluster=devnet) | note 2 |
| deposit | [`2KF8noWF6B4FCwDvrfGMwZkZoR7TR8ebDXwmAw4wCdFDxFQA9A5z2gsFjDDJm8YsC2pX7AQvsRPxjcLGCCa54wSg`](https://explorer.solana.com/tx/2KF8noWF6B4FCwDvrfGMwZkZoR7TR8ebDXwmAw4wCdFDxFQA9A5z2gsFjDDJm8YsC2pX7AQvsRPxjcLGCCa54wSg?cluster=devnet) | note 3 |
| deposit | [`2p8CVgPFQ1n36HTJAqebTNeRVxX958cKxnD1Xce3UhExd35DY7XLFDCMDT4mtKUsMKXStmaTuyDyiK93ZPqtQxqj`](https://explorer.solana.com/tx/2p8CVgPFQ1n36HTJAqebTNeRVxX958cKxnD1Xce3UhExd35DY7XLFDCMDT4mtKUsMKXStmaTuyDyiK93ZPqtQxqj?cluster=devnet) | note 4 |
| deposit | [`5udQNeabeCtLQ6KP3Kb4H1bmbiBfvLKfWsJ3Bg7oUivF4Kjqcn2Z3e8fh8mwYWE7PdcgLA3hJts8zyLM9xkgUd8K`](https://explorer.solana.com/tx/5udQNeabeCtLQ6KP3Kb4H1bmbiBfvLKfWsJ3Bg7oUivF4Kjqcn2Z3e8fh8mwYWE7PdcgLA3hJts8zyLM9xkgUd8K?cluster=devnet) | note 5 |
| create stake account | [`JWDRNZEWW78JRty7ARo8rJ3e4NtwKb4tdGkqBHzBSH52ruM1AA1oPQ31KBvhJTQjh4RXLxRFpVnmGhQfZR8q67E`](https://explorer.solana.com/tx/JWDRNZEWW78JRty7ARo8rJ3e4NtwKb4tdGkqBHzBSH52ruM1AA1oPQ31KBvhJTQjh4RXLxRFpVnmGhQfZR8q67E?cluster=devnet) | 1100000000 lamports, staker = the pool's vault, withdrawer = the operator |
| submit_spend | [`Ci9zMYy96rPowdJCWne1MVVb2p7jhcro3paKt6wJrvCsHZqziNKGCiiQpjV9Th9y7YLmnxRJcdLsJkviz7ztGXM`](https://explorer.solana.com/tx/Ci9zMYy96rPowdJCWne1MVVb2p7jhcro3paKt6wJrvCsHZqziNKGCiiQpjV9Th9y7YLmnxRJcdLsJkviz7ztGXM?cluster=devnet) | note 0, relay-signed |
| submit_spend | [`4nDV7VmAZHnLEEBNm2hgAX8yf4A59KL7NvMsnVpBevGtFKcadHGKPYRcPLK5XH4n29wTP8HUciCgYLKBUEqmYgYg`](https://explorer.solana.com/tx/4nDV7VmAZHnLEEBNm2hgAX8yf4A59KL7NvMsnVpBevGtFKcadHGKPYRcPLK5XH4n29wTP8HUciCgYLKBUEqmYgYg?cluster=devnet) | note 1, relay-signed |
| submit_spend | [`5uah1udmiCW1VHNiUi9SwqEwLYSMEP49WrxWzE178wpMHD1PFTjHiX8RuphKBtohQnVaThKJeEXkHtyxsPzT43C8`](https://explorer.solana.com/tx/5uah1udmiCW1VHNiUi9SwqEwLYSMEP49WrxWzE178wpMHD1PFTjHiX8RuphKBtohQnVaThKJeEXkHtyxsPzT43C8?cluster=devnet) | note 2, relay-signed |
| submit_spend | [`mPaXDDAQmMjCJmowSRwsJK2y4S38Y2NF9FAH4EniZJGcVtjomifSgJX5gaJqxjpS7XdKtvvQfQ6CJB3hX9PXp4q`](https://explorer.solana.com/tx/mPaXDDAQmMjCJmowSRwsJK2y4S38Y2NF9FAH4EniZJGcVtjomifSgJX5gaJqxjpS7XdKtvvQfQ6CJB3hX9PXp4q?cluster=devnet) | note 3, relay-signed, action: pool-signed CPI to SPL Memo |
| submit_spend | [`53ZdkNqVp7APhPKRQKVnUiKsCxKAKbaUrEK54EFSWuC3g99JzLqryzES6FM2TubdkgdY9sPVP9trSmcNVeHy9hBW`](https://explorer.solana.com/tx/53ZdkNqVp7APhPKRQKVnUiKsCxKAKbaUrEK54EFSWuC3g99JzLqryzES6FM2TubdkgdY9sPVP9trSmcNVeHy9hBW?cluster=devnet) | note 4, relay-signed, action: pool-signed stake delegation |
| settle_epoch | [`5VpoicNHu6m7YUvUSX7x2sztRzc751rmMRMg5SBR6qLKDxv5tFYG5pJmqCrDJajpD2sR8dCBd9BXqgooLw86iNqN`](https://explorer.solana.com/tx/5VpoicNHu6m7YUvUSX7x2sztRzc751rmMRMg5SBR6qLKDxv5tFYG5pJmqCrDJajpD2sR8dCBd9BXqgooLw86iNqN?cluster=devnet) | 5 spends in one transaction, 2 of them a CPI the pool signed |
| deposit | [`44bFc7PBkwMAMamkKVfMPWhiBHm7cRbCcJwfZed78Q2z2SyVrHzPXswWzoDAjf9wufGveEvTougb4f1E4DT7yWzr`](https://explorer.solana.com/tx/44bFc7PBkwMAMamkKVfMPWhiBHm7cRbCcJwfZed78Q2z2SyVrHzPXswWzoDAjf9wufGveEvTougb4f1E4DT7yWzr?cluster=devnet) | note 6 |

## Vault accounting

The accounting invariant is a statement about the vault's lamports, so here are the lamports. The vault holds escrow and carries no data, which is what makes its floor the rent-exempt minimum for a zero-byte account — read from the cluster during the run, not assumed.

| quantity | lamports |
|---|---|
| denomination | 20000019 |
| notes settled | 5 |
| relay fee (taken out of the denomination, not added) | 200000 |
| vault before settlement | 100890975 |
| vault after settlement | 890880 |
| rent-exempt minimum, 0 bytes | 890880 |
| **paid out** | **100000095** |
| **owed** (denomination × notes) | **100000095** |

Paid out equals owed, and the vault came to rest on its floor with a remainder of 0 lamports. The soak asserts both and fails the run otherwise, so this table cannot record a discrepancy and still exit successfully.

## The pool signed an action, and the callee said so

The settlement above carried four spends, and one of them was not a transfer: the pool invoked SPL Memo as that member's **authority**, in the same transaction as the other three. That is the capability a stake delegation or a governance vote needs and a payment does not.

A signature only proves the transaction landed. It says nothing about who signed the instruction the pool made *inside* it, so the evidence has to come from the callee. SPL Memo refuses any account handed to it that has not signed, and names the ones that did:

```
Program log: Signed by CWxsJdxBLm3LC6dEnBco68a6T4QNEF31N3qQRy95wN3Q
```

That is the pool's vault, `CWxsJdxBLm3LC6dEnBco68a6T4QNEF31N3qQRy95wN3Q`, which has no private key — it signed through seeds only the program holds. The soak reads this line back from the cluster and fails the run if it is absent, so this section cannot appear without the callee having said it.

## The pool delegated stake, as a member's authority

The same settlement carried a second signed action, and this one is the case the design exists for: a **real stake delegation**. Stake account [`53hDajhaYAvLZnazS4e86huMN8Uba8X6nAGNpAE9dL7R`](https://explorer.solana.com/address/53hDajhaYAvLZnazS4e86huMN8Uba8X6nAGNpAE9dL7R?cluster=devnet) is now delegated to validator [`2f9C9AU8nFRKUub8NHToNiZzcwmYiNeipVuP8akKgRVv`](https://explorer.solana.com/address/2f9C9AU8nFRKUub8NHToNiZzcwmYiNeipVuP8akKgRVv?cluster=devnet).

`DelegateStake` requires the **staker authority** to sign. No member can be that authority without appearing on chain and undoing the point, so the pool is, and the pool signed. The account state is read back after settlement: a stake account only reaches the `Stake` variant by being delegated — an initialised but undelegated one is a different variant — so the check distinguishes "the instruction landed" from "the delegation took", and the validator's key is read out of the account rather than assumed from what was requested.

**The withdraw authority is deliberately not the pool.** The pool's signature is available to every member, so an authority the vault holds is an authority every member holds. Delegation is safe on those terms — the worst a member can do is re-delegate to another validator. Withdrawal is not, and it stays with the operator. That is `docs/THREAT_MODEL.md` applied rather than repeated.

What is hidden here is precisely one thing: **which member asked**. The stake account, the validator, the amount and the timing are all public, and the anonymity set is the 5 members who settled together. What an observer cannot recover is which of them authorised this delegation rather than one of the plain transfers beside it.

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
