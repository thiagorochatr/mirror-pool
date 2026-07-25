//! mirror-pool: a behavioral anonymity set. A member's protocol action is
//! executed by the pool PDA, gated by a Groth16 membership proof verified
//! on-chain, so an observer sees that an action happened but cannot attribute it
//! to a member.
#![forbid(unsafe_code)]

pub mod error;
pub mod instruction;
pub mod state;

pub use error::MirrorProgramError;
pub use instruction::Instruction;
pub use state::{Pool, POOL_LEN, POOL_VERSION, ROOT_HISTORY};
