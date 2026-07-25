//! mirror-pool: a behavioral anonymity set. A member's protocol action is executed
//! by the pool PDA, gated by a Groth16 membership proof verified on-chain, so an
//! observer sees that an action happened but cannot attribute it to a member.
#![forbid(unsafe_code)]
