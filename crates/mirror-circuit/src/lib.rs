//! Groth16 membership circuit over BN254, its trusted-setup ceremony, and the
//! host-side prover. Verifying keys are exported in the byte layout the
//! on-chain `groth16-solana` verifier consumes.
#![forbid(unsafe_code)]
