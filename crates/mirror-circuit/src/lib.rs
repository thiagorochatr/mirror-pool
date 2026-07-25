//! Groth16 membership circuit over BN254, its key generation, and the host
//! prover. Host-only: never linked into the on-chain program.
#![forbid(unsafe_code)]

pub mod poseidon_gadget;
