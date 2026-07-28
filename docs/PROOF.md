# Live-cluster evidence

Every line here is a transaction that landed. Nothing is simulated: the proof is produced by the host prover and verified by the deployed program's own Groth16 syscall.

- cluster: `devnet`
- program: `8H3cYoiAA9LM36cyPr4UEv38dhHasSu2XPSdiBfyrLEa`
- pool: `JBvD5u5foKCThTfSx1TozGHGCcfWu5gDNy1u2FphP51q`
- vault: `EwiXhCnLcg6jEaHMumo5H4tZnVyoCtBPHU5R6hE798R5`

## Flows

| step | signature | note |
|---|---|---|
| init_pool | [`4Ru5RVJ1pyoFSYKrf58Rj43LsawpHQymEwW9zQgKNPpGuv5MdSBzT1Fyxpo8dtdXmcCxCZZN2SwEZVvnkSQXM5dn`](https://explorer.solana.com/tx/4Ru5RVJ1pyoFSYKrf58Rj43LsawpHQymEwW9zQgKNPpGuv5MdSBzT1Fyxpo8dtdXmcCxCZZN2SwEZVvnkSQXM5dn?cluster=devnet) | denomination 20000023, k_floor 4 |
| deposit | [`2z4zzRhA98HJcMHCMF9wzqYMjYCbBNX8K52ASWjhcFAQ887hCzHd1woURxLt2REgsTsLRMn32NubEiXqpGRTCHhZ`](https://explorer.solana.com/tx/2z4zzRhA98HJcMHCMF9wzqYMjYCbBNX8K52ASWjhcFAQ887hCzHd1woURxLt2REgsTsLRMn32NubEiXqpGRTCHhZ?cluster=devnet) | note 1 |
| deposit | [`45uwVLTFs81kU5rv4LgKHWS4apiDoThVRMrq8LziKr5iX9papXySRC3S1fXumZ8UEn4otMzG6iyT6LhcgRSYxu5d`](https://explorer.solana.com/tx/45uwVLTFs81kU5rv4LgKHWS4apiDoThVRMrq8LziKr5iX9papXySRC3S1fXumZ8UEn4otMzG6iyT6LhcgRSYxu5d?cluster=devnet) | note 2 |
| deposit | [`4QaHJBxGD1St4hiLyqmrmJmztTbmam1ssGrSABFE3S6sM6Q4JfSY1oqG5onXty17RT7KNaARXjTM2HYJMsyvur6D`](https://explorer.solana.com/tx/4QaHJBxGD1St4hiLyqmrmJmztTbmam1ssGrSABFE3S6sM6Q4JfSY1oqG5onXty17RT7KNaARXjTM2HYJMsyvur6D?cluster=devnet) | note 3 |
| deposit | [`5ZtoW7nNqVwSYJuzmQqwBiEm52tRgcQdYYGLbGMmD8cN1cqtbqh3mdjRqQzKBqWeetnnoQhTHBaUebTq8SxmBovW`](https://explorer.solana.com/tx/5ZtoW7nNqVwSYJuzmQqwBiEm52tRgcQdYYGLbGMmD8cN1cqtbqh3mdjRqQzKBqWeetnnoQhTHBaUebTq8SxmBovW?cluster=devnet) | note 4 |
| deposit | [`5TdTHdRJXddC1JP6uW4C993LY7ifSoi4ZxbavwhK95Wq96MyGUmHuQm88tkRRykhz4pzpaCQhJC3TR2A7Y843jxB`](https://explorer.solana.com/tx/5TdTHdRJXddC1JP6uW4C993LY7ifSoi4ZxbavwhK95Wq96MyGUmHuQm88tkRRykhz4pzpaCQhJC3TR2A7Y843jxB?cluster=devnet) | note 5 |
| create stake account | [`WmqHSUXx71nm81RzbhjgRZir1hzTMzGxg6k45fgPCi8cbyt1D3FVR5DKAETSapCmeQojM1TzMgAGbotReLBW4Kz`](https://explorer.solana.com/tx/WmqHSUXx71nm81RzbhjgRZir1hzTMzGxg6k45fgPCi8cbyt1D3FVR5DKAETSapCmeQojM1TzMgAGbotReLBW4Kz?cluster=devnet) | 1100000000 lamports, staker = the pool's vault, withdrawer = the operator |
| submit_spend | [`42bLZCNNwSo82r3Zwq1VVB4PqQCFm5ETjiFBai9QZYjNvzrkeJBwiYWb47RRMyXJ3Q9w22yV88GdSUFZ2CRKfGMK`](https://explorer.solana.com/tx/42bLZCNNwSo82r3Zwq1VVB4PqQCFm5ETjiFBai9QZYjNvzrkeJBwiYWb47RRMyXJ3Q9w22yV88GdSUFZ2CRKfGMK?cluster=devnet) | note 0, relay-signed |
| submit_spend | [`4jJwmRta9miD3yQgvhWSzZysZTS5wsKPM4E1ZPnHMnUCa3pPxXew4DQQAG3Y7Luk1bECD9V3A39dSBDrXxY3Wegw`](https://explorer.solana.com/tx/4jJwmRta9miD3yQgvhWSzZysZTS5wsKPM4E1ZPnHMnUCa3pPxXew4DQQAG3Y7Luk1bECD9V3A39dSBDrXxY3Wegw?cluster=devnet) | note 1, relay-signed |
| submit_spend | [`4fpZU88zDAeu2ySvjdit7My1FTzp9P7MZbRHHzUDD3V53nBLMruw5vb1WPG2zmGUnt9AFD1j6uimGmzKsYiLDF8A`](https://explorer.solana.com/tx/4fpZU88zDAeu2ySvjdit7My1FTzp9P7MZbRHHzUDD3V53nBLMruw5vb1WPG2zmGUnt9AFD1j6uimGmzKsYiLDF8A?cluster=devnet) | note 2, relay-signed |
| submit_spend | [`3gEhEPza2ryHiJuxPo8fZktqyhwyh2fwCwx18Bw7q95NPrZaszqpLtZq6KfrXefD7RR5eSbiR4Di5etv6HAWGaYL`](https://explorer.solana.com/tx/3gEhEPza2ryHiJuxPo8fZktqyhwyh2fwCwx18Bw7q95NPrZaszqpLtZq6KfrXefD7RR5eSbiR4Di5etv6HAWGaYL?cluster=devnet) | note 3, relay-signed, action: pool-signed CPI to SPL Memo |
| submit_spend | [`XLZJ3wDgJXe9YhiVD2akpEKHhwMziExkMntUtbW86TkKn6ktw5hWNrmqYG7FZ51jiYuGr6PhnE87v4kvEacViBA`](https://explorer.solana.com/tx/XLZJ3wDgJXe9YhiVD2akpEKHhwMziExkMntUtbW86TkKn6ktw5hWNrmqYG7FZ51jiYuGr6PhnE87v4kvEacViBA?cluster=devnet) | note 4, relay-signed, action: pool-signed stake delegation |
| settle_epoch | [`4T6MYo8rMW6wYZb6FdKSQUSrinLGFoo6gpq5YMQ4RAqCnFbHdeSueeMosEkoBf5uYEhcQRtiCJA16BtvKAKZ58km`](https://explorer.solana.com/tx/4T6MYo8rMW6wYZb6FdKSQUSrinLGFoo6gpq5YMQ4RAqCnFbHdeSueeMosEkoBf5uYEhcQRtiCJA16BtvKAKZ58km?cluster=devnet) | 5 spends in one transaction, 2 of them a CPI the pool signed |
| deposit | [`2jXf1wQ3J3And4TjAS47oUggAzwGFc2wVGy4v4xDyBeoVopNfa6RdW4SCBrNcva3ayjQQcFSZLdRtA8FJockosZ5`](https://explorer.solana.com/tx/2jXf1wQ3J3And4TjAS47oUggAzwGFc2wVGy4v4xDyBeoVopNfa6RdW4SCBrNcva3ayjQQcFSZLdRtA8FJockosZ5?cluster=devnet) | note 6 |

## Vault accounting

The accounting invariant is a statement about the vault's lamports, so here are the lamports. The vault holds escrow and carries no data, which is what makes its floor the rent-exempt minimum for a zero-byte account — read from the cluster during the run, not assumed.

| quantity | lamports |
|---|---|
| denomination | 20000023 |
| notes settled | 5 |
| relay fee (taken out of the denomination, not added) | 200000 |
| vault before settlement | 100890995 |
| vault after settlement | 890880 |
| rent-exempt minimum, 0 bytes | 890880 |
| **paid out** | **100000115** |
| **owed** (denomination × notes) | **100000115** |

Paid out equals owed, and the vault came to rest on its floor with a remainder of 0 lamports. The soak asserts both and fails the run otherwise, so this table cannot record a discrepancy and still exit successfully.

## The pool signed an action, and the callee said so

The settlement above carried 5 spends, and 2 of them were not a transfer: the pool invoked SPL Memo as one member's **authority**, in the same transaction as the other 4. That is the capability a stake delegation or a governance vote needs and a payment does not.

A signature only proves the transaction landed. It says nothing about who signed the instruction the pool made *inside* it, so the evidence has to come from the callee. SPL Memo refuses any account handed to it that has not signed, and names the ones that did:

```
Program log: Signed by EwiXhCnLcg6jEaHMumo5H4tZnVyoCtBPHU5R6hE798R5
```

That is the pool's vault, `EwiXhCnLcg6jEaHMumo5H4tZnVyoCtBPHU5R6hE798R5`, which has no private key — it signed through seeds only the program holds. The soak reads this line back from the cluster and fails the run if it is absent, so this section cannot appear without the callee having said it.

## The pool delegated stake, as a member's authority

The same settlement carried a second signed action, and this one is the case the design exists for: a **real stake delegation**. Stake account [`CMMwt1SgrJyEfdBsxxUNSwsPwNiU94ZNkVGMUzvMU1Ri`](https://explorer.solana.com/address/CMMwt1SgrJyEfdBsxxUNSwsPwNiU94ZNkVGMUzvMU1Ri?cluster=devnet) is now delegated to validator [`2f9C9AU8nFRKUub8NHToNiZzcwmYiNeipVuP8akKgRVv`](https://explorer.solana.com/address/2f9C9AU8nFRKUub8NHToNiZzcwmYiNeipVuP8akKgRVv?cluster=devnet).

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
| front-run relay | `0x12` | a proof is spendable only by the relay the member made it for |

The last three attack a note that is still live, deposited after settlement precisely so that they would have to. Against an already-spent note the replay guard fires first and the rejection would say nothing about the check under test — which is how a negative case comes to pass for the wrong reason.

## Scope

Devnet is devnet. This is a live-cluster functional proof, not a claim of mainnet operation or of an anonymity crowd: the run is scripted by one operator with a handful of notes. What it establishes is that the circuit, the prover, the byte layout and the deployed program agree on a real validator.
