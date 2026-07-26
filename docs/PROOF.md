# Live-cluster evidence

Every line here is a transaction that landed. Nothing is simulated: the proof is produced by the host prover and verified by the deployed program's own Groth16 syscall.

- cluster: `devnet`
- program: `8H3cYoiAA9LM36cyPr4UEv38dhHasSu2XPSdiBfyrLEa`
- pool: `J68VyjjKQTvg7tsZJHoCEQ5nMYGnUVs4esA7EF1nP8Ui`
- vault: `54PT56YwQWJA6UAYNsnqKPJq4SrCW2gnF7HVvU8QDk7a`

## Flows

| step | signature | note |
|---|---|---|
| init_pool | [`2i2dSth52MNMboBjL7s28MaArrh9Vmj2z7Rm21CYpig79czESepPUyqkDPiyvM8UvV2NP587kyZUqxhZYB2CJrdB`](https://explorer.solana.com/tx/2i2dSth52MNMboBjL7s28MaArrh9Vmj2z7Rm21CYpig79czESepPUyqkDPiyvM8UvV2NP587kyZUqxhZYB2CJrdB?cluster=devnet) | denomination 20000007, k_floor 4 |
| deposit | [`3uFCoMKsmeyWFbvRFE2qiwWm3VRTZmdMW4tnsSxLMCMuXfhzrt7irdjE9S1R86HmzC3vc7j1BKpNbj893CrzW8RX`](https://explorer.solana.com/tx/3uFCoMKsmeyWFbvRFE2qiwWm3VRTZmdMW4tnsSxLMCMuXfhzrt7irdjE9S1R86HmzC3vc7j1BKpNbj893CrzW8RX?cluster=devnet) | note 1 |
| deposit | [`4dtLcdt9CdCdoimcgNLouG8Kh72oMfy1jrB2p5e8muAUXc4asGk2SsSgsERhzUjXNHZSmr2xMWfWXcnJvX9sav2H`](https://explorer.solana.com/tx/4dtLcdt9CdCdoimcgNLouG8Kh72oMfy1jrB2p5e8muAUXc4asGk2SsSgsERhzUjXNHZSmr2xMWfWXcnJvX9sav2H?cluster=devnet) | note 2 |
| deposit | [`4nRS8qTSW9h58kimVCX8D7YWHHw4uicBnCfZcPqqLdcHcUCxnhGbX7GTjRKuy9RLSeHbrALGp7Gu3RAgfRMaJkSX`](https://explorer.solana.com/tx/4nRS8qTSW9h58kimVCX8D7YWHHw4uicBnCfZcPqqLdcHcUCxnhGbX7GTjRKuy9RLSeHbrALGp7Gu3RAgfRMaJkSX?cluster=devnet) | note 3 |
| deposit | [`4F5nMtQTdEHiBb2yDcZhwMtscsdFbxEZXxJvAt3ZAxYvTLJsC2bgQbiiw8pCsMYizuFUivHgQcz3JXG75x5efYEs`](https://explorer.solana.com/tx/4F5nMtQTdEHiBb2yDcZhwMtscsdFbxEZXxJvAt3ZAxYvTLJsC2bgQbiiw8pCsMYizuFUivHgQcz3JXG75x5efYEs?cluster=devnet) | note 4 |
| submit_spend | [`48RWESn6bf5ipZi9uYhUo31XJw4RcqZyxT3vk3U8g2UZ8bceZihUmUJrZCJTt9e18Mi2ymfQW8j26Vvoo2fKgYZu`](https://explorer.solana.com/tx/48RWESn6bf5ipZi9uYhUo31XJw4RcqZyxT3vk3U8g2UZ8bceZihUmUJrZCJTt9e18Mi2ymfQW8j26Vvoo2fKgYZu?cluster=devnet) | note 0, relay-signed |
| submit_spend | [`4QFM6p4cniHTmATLJPuMR89DyYohFM5N3wrXYGJJpiAQ3xzNLgz8hCD5pN5BEoyysRYDc6FcQD2xbRZggrw2Nsmi`](https://explorer.solana.com/tx/4QFM6p4cniHTmATLJPuMR89DyYohFM5N3wrXYGJJpiAQ3xzNLgz8hCD5pN5BEoyysRYDc6FcQD2xbRZggrw2Nsmi?cluster=devnet) | note 1, relay-signed |
| submit_spend | [`FHHgBrh8ytutD58XtSTGUKBJpgqq5rV9krBgq9VheaBJQEpQcucZkU85mHX9KnuvEiazDA753Se2azDSPb4RmtZ`](https://explorer.solana.com/tx/FHHgBrh8ytutD58XtSTGUKBJpgqq5rV9krBgq9VheaBJQEpQcucZkU85mHX9KnuvEiazDA753Se2azDSPb4RmtZ?cluster=devnet) | note 2, relay-signed |
| submit_spend | [`4ZFy3xaCokZoqWYvEw3iwRYQQr6obpMqdKjUvtUqhaPZqRW3RFSzX3Eva7QdvJLpnpMRXhvw9cEB26NY9Tbh8o6P`](https://explorer.solana.com/tx/4ZFy3xaCokZoqWYvEw3iwRYQQr6obpMqdKjUvtUqhaPZqRW3RFSzX3Eva7QdvJLpnpMRXhvw9cEB26NY9Tbh8o6P?cluster=devnet) | note 3, relay-signed |
| settle_epoch | [`67SdKHQ2fHcJ8nommceaaJnnFaCL5mtKwtuyy2uTHB2Lz8aauyDtGDTQ8SJFC28LMM33jZQHjb3cm3x9BXRiPM8W`](https://explorer.solana.com/tx/67SdKHQ2fHcJ8nommceaaJnnFaCL5mtKwtuyy2uTHB2Lz8aauyDtGDTQ8SJFC28LMM33jZQHjb3cm3x9BXRiPM8W?cluster=devnet) | 4 spends |
| deposit | [`4fcgHaFewWfoy2QMqpGTChxuhfFg8fTeWts6sW45rR1ZhJMk9fR4X6AQnZXBxiZW3Si55VNpSzuxh8FtL1p8ENtt`](https://explorer.solana.com/tx/4fcgHaFewWfoy2QMqpGTChxuhfFg8fTeWts6sW45rR1ZhJMk9fR4X6AQnZXBxiZW3Si55VNpSzuxh8FtL1p8ENtt?cluster=devnet) | note 5 |

## Vault accounting

The accounting invariant is a statement about the vault's lamports, so here are the lamports. The vault holds escrow and carries no data, which is what makes its floor the rent-exempt minimum for a zero-byte account — read from the cluster during the run, not assumed.

| quantity | lamports |
|---|---|
| denomination | 20000007 |
| notes settled | 4 |
| relay fee (taken out of the denomination, not added) | 200000 |
| vault before settlement | 80890908 |
| vault after settlement | 890880 |
| rent-exempt minimum, 0 bytes | 890880 |
| **paid out** | **80000028** |
| **owed** (denomination × notes) | **80000028** |

Paid out equals owed, and the vault came to rest on its floor with a remainder of 0 lamports. The soak asserts both and fails the run otherwise, so this table cannot record a discrepancy and still exit successfully.

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
