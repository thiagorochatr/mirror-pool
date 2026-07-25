//! Program errors.
//!
//! Every variant is a distinct code so that a failing transaction says which
//! invariant it violated. Devnet evidence records these codes alongside the
//! signature, which is what makes a negative test checkable by a third party
//! rather than a claim in a README.

use solana_program::program_error::ProgramError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MirrorProgramError {
    /// The instruction data was empty, truncated, or carried an unknown tag.
    MalformedInstruction = 1,
    /// An account was not the PDA the program derives for that role.
    InvalidPda = 2,
    /// A required signature was absent.
    MissingSignature = 3,
    /// An account was owned by the wrong program.
    InvalidOwner = 4,
    /// The pool account has the wrong length or an unrecognised version.
    InvalidPoolAccount = 5,
    /// The pool is already initialised.
    AlreadyInitialised = 6,
    /// A pool parameter was outside its permitted range.
    InvalidParameter = 7,
    /// A 32-byte value was not a canonical BN254 scalar.
    NonCanonicalField = 8,
    /// Poseidon hashing failed.
    PoseidonFailed = 9,
    /// The accumulator is full.
    TreeFull = 10,
    /// The deposit did not carry exactly the pool's denomination plus fee.
    WrongDepositAmount = 11,
    /// Arithmetic overflowed or underflowed.
    ArithmeticOverflow = 12,
    /// The vault does not hold enough to cover every unspent note.
    ///
    /// This is the accounting invariant. It is checked rather than assumed
    /// because the two competing implementations are both drainable at exactly
    /// this point.
    InsolventVault = 13,
    /// The spend account has the wrong length or an unrecognised version.
    InvalidSpendAccount = 14,
    /// This nullifier has already been recorded: the note is spent.
    NullifierAlreadySpent = 15,
    /// The spend has already been executed.
    AlreadySettled = 16,
    /// The proof references a root the pool does not retain.
    UnknownRoot = 17,
    /// The Groth16 proof did not verify.
    ProofVerificationFailed = 18,
    /// The pool holds fewer notes than its anonymity floor requires.
    BelowAnonymityFloor = 19,
    /// The relay fee is not less than the denomination.
    RelayFeeTooLarge = 20,
}

impl From<MirrorProgramError> for ProgramError {
    fn from(e: MirrorProgramError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl From<mirror_core::MirrorError> for MirrorProgramError {
    fn from(e: mirror_core::MirrorError) -> Self {
        use mirror_core::MirrorError as E;
        match e {
            E::NonCanonicalField | E::BadFieldLength => MirrorProgramError::NonCanonicalField,
            E::Poseidon => MirrorProgramError::PoseidonFailed,
            E::TreeFull => MirrorProgramError::TreeFull,
            E::BadPathLength | E::LeafIndexOutOfRange => MirrorProgramError::MalformedInstruction,
        }
    }
}
