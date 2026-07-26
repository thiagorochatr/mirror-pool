# Live-cluster evidence

Every line here is a transaction that landed. Nothing is simulated: the proof is produced by the host prover and verified by the deployed program's own Groth16 syscall.

- cluster: `devnet`
- program: `8H3cYoiAA9LM36cyPr4UEv38dhHasSu2XPSdiBfyrLEa`
- pool: `ABb58741sCZV6nWtwfK1dEqonLBp2dGDiAe85jBnWa22`
- vault: `5vWyEoKQjoYMUnyRe1yzSnGGUBfhs654ssxiFadwxB5t`

## Flows

| step | signature | note |
|---|---|---|
| init_pool | [`a1Um5GdackChmKVUNUgneYCEFiqQ4hjcqVQFhb9tTeAiYNQBRKCbgCJf8TAYXWP877b95ZDAxcebJWrbrnSVRFm`](https://explorer.solana.com/tx/a1Um5GdackChmKVUNUgneYCEFiqQ4hjcqVQFhb9tTeAiYNQBRKCbgCJf8TAYXWP877b95ZDAxcebJWrbrnSVRFm?cluster=devnet) | denomination 20000003, k_floor 4 |
| deposit | [`5ZJhBMLfajJBxfxeHgArQ25uHx6XZDp8PH44W4wuE4g15yexTmiC6oJJXfaLBHu9MLXRD3ggwEpB6FVpTMWvV8Py`](https://explorer.solana.com/tx/5ZJhBMLfajJBxfxeHgArQ25uHx6XZDp8PH44W4wuE4g15yexTmiC6oJJXfaLBHu9MLXRD3ggwEpB6FVpTMWvV8Py?cluster=devnet) | note 1 |
| deposit | [`3cXFiZSM2Z3onk31w8Q7Q21d56ZqrNACEQh4VdzLg2ub8aXURRVuZLL9TKqcnFWAtiftYRoGn9xxERx3RF21zVmX`](https://explorer.solana.com/tx/3cXFiZSM2Z3onk31w8Q7Q21d56ZqrNACEQh4VdzLg2ub8aXURRVuZLL9TKqcnFWAtiftYRoGn9xxERx3RF21zVmX?cluster=devnet) | note 2 |
| deposit | [`4HSKcvxtBnxSeECCEc2UPCfc4xoUcbiMminMET73A5mAwqPtjR5u2prLgRaxPMcWS18EZB5pbktKhAZdGLXB45LG`](https://explorer.solana.com/tx/4HSKcvxtBnxSeECCEc2UPCfc4xoUcbiMminMET73A5mAwqPtjR5u2prLgRaxPMcWS18EZB5pbktKhAZdGLXB45LG?cluster=devnet) | note 3 |
| deposit | [`4vXba4TPxyGJeZosLnaDddJom7yerWCVAGoMHQsT1RMwCS7NQ34rEbX1TqA6DUYUV3Pugj7SDY34DN8MaruHMVpP`](https://explorer.solana.com/tx/4vXba4TPxyGJeZosLnaDddJom7yerWCVAGoMHQsT1RMwCS7NQ34rEbX1TqA6DUYUV3Pugj7SDY34DN8MaruHMVpP?cluster=devnet) | note 4 |
| submit_spend | [`2AxhzG9wa3nVsuxNJf7WmhVYHNQw1QimrFvSdBKgz3ZyyTJbr9D4iDtzVq8Ap7f4vqVrKCcsWLabVUpEhiPmCu6R`](https://explorer.solana.com/tx/2AxhzG9wa3nVsuxNJf7WmhVYHNQw1QimrFvSdBKgz3ZyyTJbr9D4iDtzVq8Ap7f4vqVrKCcsWLabVUpEhiPmCu6R?cluster=devnet) | note 0, relay-signed |
| submit_spend | [`4n5zRhNC3GpmDAxoauNMsb9xm9FWDgqMMFGSKuB8176xBrAVPCUSx54uBZphVdHbUu3LZeNgXCgQCaUDorynKU73`](https://explorer.solana.com/tx/4n5zRhNC3GpmDAxoauNMsb9xm9FWDgqMMFGSKuB8176xBrAVPCUSx54uBZphVdHbUu3LZeNgXCgQCaUDorynKU73?cluster=devnet) | note 1, relay-signed |
| submit_spend | [`5TTZhv3kJ1EQpVjdh3nUZt5rgPHSXzfqWfyeGx97vj8XM4BJz1orxc6pYqiSirqGUWUbVkX6zvmiN65RcGcRp71v`](https://explorer.solana.com/tx/5TTZhv3kJ1EQpVjdh3nUZt5rgPHSXzfqWfyeGx97vj8XM4BJz1orxc6pYqiSirqGUWUbVkX6zvmiN65RcGcRp71v?cluster=devnet) | note 2, relay-signed |
| submit_spend | [`eiczViayoZXPJ5VbEoFSQT6Pu9udpZByo532RA7YTWfKSqoL7GqcuhiCxZmeyekSeyyRW8GKstzNCFLdajW4M7G`](https://explorer.solana.com/tx/eiczViayoZXPJ5VbEoFSQT6Pu9udpZByo532RA7YTWfKSqoL7GqcuhiCxZmeyekSeyyRW8GKstzNCFLdajW4M7G?cluster=devnet) | note 3, relay-signed |
| settle_epoch | [`44qJhqRE1rxkZqqbgnKoy1vZBoqa2P3dmH6zuwbQy4RgCp72XgUkWqqjiaNR9iUXp2gZndtcxMuLWsH4so6d2TiF`](https://explorer.solana.com/tx/44qJhqRE1rxkZqqbgnKoy1vZBoqa2P3dmH6zuwbQy4RgCp72XgUkWqqjiaNR9iUXp2gZndtcxMuLWsH4so6d2TiF?cluster=devnet) | 4 spends |

## Vault accounting

The accounting invariant is a statement about the vault's lamports, so here are the lamports. The vault holds escrow and carries no data, which is what makes its floor the rent-exempt minimum for a zero-byte account — read from the cluster during the run, not assumed.

| quantity | lamports |
|---|---|
| denomination | 20000003 |
| notes settled | 4 |
| relay fee (taken out of the denomination, not added) | 200000 |
| vault before settlement | 80890892 |
| vault after settlement | 890880 |
| rent-exempt minimum, 0 bytes | 890880 |
| **paid out** | **80000012** |
| **owed** (denomination × notes) | **80000012** |

Paid out equals owed, and the vault came to rest on its floor with a remainder of 0 lamports. The soak asserts both and fails the run otherwise, so this table cannot record a discrepancy and still exit successfully.

## Rejections

A negative case is only evidence if the program's own error code is what rejected it. A bare runtime failure proves nothing about this program.

| case | error code |
|---|---|
| replayed nullifier | `0xf` |

## Scope

Devnet is devnet. This is a live-cluster functional proof, not a claim of mainnet operation or of an anonymity crowd: the run is scripted by one operator with a handful of notes. What it establishes is that the circuit, the prover, the byte layout and the deployed program agree on a real validator.
