# Live-cluster evidence

Every line here is a transaction that landed. Nothing is simulated: the proof is produced by the host prover and verified by the deployed program's own Groth16 syscall.

- cluster: `devnet`
- program: `8H3cYoiAA9LM36cyPr4UEv38dhHasSu2XPSdiBfyrLEa`
- pool: `CoxF5A4YG6bgtZ55WTYpLkXXJWC8oFbpsxbmfhvMe14W`
- vault: `HCkWvxyREJiLByGHbKt47uXJxjTubDM3V2Fs1zjmPu2p`

## Flows

| step | signature | note |
|---|---|---|
| deposit | [`4GCouwQ6xCierPJSoWWcpi7gq91k15YnuAbsQXMFXgK9XcSThsZ6xTamBcdt5R43o2LWUNNUDqosNpQFr48jtE9f`](https://explorer.solana.com/tx/4GCouwQ6xCierPJSoWWcpi7gq91k15YnuAbsQXMFXgK9XcSThsZ6xTamBcdt5R43o2LWUNNUDqosNpQFr48jtE9f?cluster=devnet) | note 5 |
| deposit | [`NidbwQVPYXtQ65pSs7i5GujuALQVbiKvwMz4mPahtURz9PQuz1qQ7K6ioYGwkYEKZ662LBo92NNHiQYCiJzbrXX`](https://explorer.solana.com/tx/NidbwQVPYXtQ65pSs7i5GujuALQVbiKvwMz4mPahtURz9PQuz1qQ7K6ioYGwkYEKZ662LBo92NNHiQYCiJzbrXX?cluster=devnet) | note 6 |
| deposit | [`2tkuqJAivXhhdqzgwHk432Pmz5WF7qUZbax5Pps4ufxeppnuiGSFYgxKXfvzfKht6phUEZ5pMDTKqMrY8SPwHz3j`](https://explorer.solana.com/tx/2tkuqJAivXhhdqzgwHk432Pmz5WF7qUZbax5Pps4ufxeppnuiGSFYgxKXfvzfKht6phUEZ5pMDTKqMrY8SPwHz3j?cluster=devnet) | note 7 |
| deposit | [`4amq6HyxccQtKPXcvpy6jNSQ8EhwxEybFA1f3sy8mFCBXy2PSrYcA5nnrQigiAMguEhVNMEsai3566DWP4UzhUuA`](https://explorer.solana.com/tx/4amq6HyxccQtKPXcvpy6jNSQ8EhwxEybFA1f3sy8mFCBXy2PSrYcA5nnrQigiAMguEhVNMEsai3566DWP4UzhUuA?cluster=devnet) | note 8 |
| submit_spend | [`2PDhAtCvDRFCW3u3nUaXvWSNjZkS72cdbNGoH1hAbpXoeGRnv4UcUCvnFMkscHnnW3M2kcFe722WieZApHmHrx8H`](https://explorer.solana.com/tx/2PDhAtCvDRFCW3u3nUaXvWSNjZkS72cdbNGoH1hAbpXoeGRnv4UcUCvnFMkscHnnW3M2kcFe722WieZApHmHrx8H?cluster=devnet) | note 4, relay-signed |
| submit_spend | [`2MoYdaFjZE3GjW6QRdARzrQjprRyHALzC5uEyN7cY7PK4qyCsr2j5AJ7LVc17Ldydk4UNhMR6Eh7TZnKCiydyS4L`](https://explorer.solana.com/tx/2MoYdaFjZE3GjW6QRdARzrQjprRyHALzC5uEyN7cY7PK4qyCsr2j5AJ7LVc17Ldydk4UNhMR6Eh7TZnKCiydyS4L?cluster=devnet) | note 5, relay-signed |
| submit_spend | [`2d5D67BzdtuPzpXEAZFqK6MCQr92Zn7TH5ynCvuVP5mhacNX7cpZ7o3w4CfzfK1DbRrzHnqiR1xN4az1xtHRu6Zw`](https://explorer.solana.com/tx/2d5D67BzdtuPzpXEAZFqK6MCQr92Zn7TH5ynCvuVP5mhacNX7cpZ7o3w4CfzfK1DbRrzHnqiR1xN4az1xtHRu6Zw?cluster=devnet) | note 6, relay-signed |
| submit_spend | [`413vqoRynW8WzAFaHuhGx8hrxTz9GcwWf5c5EPjig8Lkx4DUJyXA2RdoF7HhaMRgubciy8fr7tT1zM4UHe8rRdGd`](https://explorer.solana.com/tx/413vqoRynW8WzAFaHuhGx8hrxTz9GcwWf5c5EPjig8Lkx4DUJyXA2RdoF7HhaMRgubciy8fr7tT1zM4UHe8rRdGd?cluster=devnet) | note 7, relay-signed |
| settle_epoch | [`5WrDG3EoU2kwVe1rEdkiQ8tNP1YbxAkKobquV8QSQoJLJn6qCQg1pbAixSmpzDvJxPcXXtFx3fUWnpdrprbkrAZj`](https://explorer.solana.com/tx/5WrDG3EoU2kwVe1rEdkiQ8tNP1YbxAkKobquV8QSQoJLJn6qCQg1pbAixSmpzDvJxPcXXtFx3fUWnpdrprbkrAZj?cluster=devnet) | 4 spends |

## Rejections

A negative case is only evidence if the program's own error code is what rejected it. A bare runtime failure proves nothing about this program.

| case | error code |
|---|---|
| replayed nullifier | `0xf` |

## Scope

Devnet is devnet. This is a live-cluster functional proof, not a claim of mainnet operation or of an anonymity crowd: the run is scripted by one operator with a handful of notes. What it establishes is that the circuit, the prover, the byte layout and the deployed program agree on a real validator.
