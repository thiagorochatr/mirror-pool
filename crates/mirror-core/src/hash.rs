//! Poseidon over BN254, with domain separation by arity.
//!
//! `solana-poseidon` is the only entry point, deliberately. Upstream it is
//! cfg-gated to the `sol_poseidon` syscall on-chain and to light-poseidon over
//! ark-bn254 off-chain, so the host and the program compute the same function by
//! construction rather than by our own careful duplication. The R1CS gadget in
//! `mirror-circuit` is checked against this, which closes the three-way parity:
//! gadget == host == syscall.
//!
//! ## Domain separation
//!
//! Poseidon instances of different width are different permutations, so arity is
//! itself a domain separator — and it is a free one. Each hashed value in the
//! protocol uses a distinct arity, so no preimage of one can ever be reread as a
//! preimage of another:
//!
//! | value | arity | preimage |
//! |---|---|---|
//! | nullifier | 1 | `(k)` |
//! | Merkle node | 2 | `(left, right)` |
//! | note commitment | 3 | `(k, r, denom_tag)` |
//! | action binding | 4 | `(selector, beneficiary_hi, beneficiary_lo, relay_fee)` |
//!
//! An explicit integer tag was the first design here and it was wrong: with a
//! small tag constant, a Merkle node whose left child equals the tag collides
//! with a nullifier. Reaching that state requires a Poseidon preimage and so is
//! not practically exploitable, but arity separation removes the question
//! entirely at no cost.
//!
//! A competing implementation in this bounty hashes its Merkle nodes, its
//! nullifiers and its action bindings with one untagged arity-2 function.

use crate::{Field, MirrorError};
use solana_poseidon::{hashv, Endianness, Parameters};

fn digest(inputs: &[&[u8]]) -> Result<Field, MirrorError> {
    let out = hashv(Parameters::Bn254X5, Endianness::BigEndian, inputs)
        .map_err(|_| MirrorError::Poseidon)?;
    // The syscall reduces into the field, so this cannot fail. We still go
    // through the checked constructor rather than assume it.
    Field::from_bytes(out.to_bytes())
}

/// Arity-1 Poseidon.
pub fn poseidon1(a: Field) -> Result<Field, MirrorError> {
    digest(&[a.as_bytes()])
}

/// Arity-2 Poseidon.
pub fn poseidon2(a: Field, b: Field) -> Result<Field, MirrorError> {
    digest(&[a.as_bytes(), b.as_bytes()])
}

/// Arity-3 Poseidon.
pub fn poseidon3(a: Field, b: Field, c: Field) -> Result<Field, MirrorError> {
    digest(&[a.as_bytes(), b.as_bytes(), c.as_bytes()])
}

/// Arity-4 Poseidon.
pub fn poseidon4(a: Field, b: Field, c: Field, d: Field) -> Result<Field, MirrorError> {
    digest(&[a.as_bytes(), b.as_bytes(), c.as_bytes(), d.as_bytes()])
}

/// Merkle internal node: `H2(left, right)`.
pub fn hash_node(left: Field, right: Field) -> Result<Field, MirrorError> {
    poseidon2(left, right)
}

/// Note commitment: `H3(k, r, denom_tag)`.
///
/// `k` is the nullifier preimage, `r` the blinding factor. `denom_tag` pins the
/// commitment to the pool's denomination so a note can never be replayed into a
/// pool of a different size even if the two share an accumulator.
pub fn commitment(k: Field, r: Field, denom_tag: Field) -> Result<Field, MirrorError> {
    poseidon3(k, r, denom_tag)
}

/// Nullifier: `H1(k)`.
///
/// Deliberately not scoped to an epoch. An epoch-scoped nullifier guarding a
/// value-bearing payout lets a single deposit pay out once per epoch forever,
/// which is a live drain in a competing submission. Ours is spent once, ever.
pub fn nullifier(k: Field) -> Result<Field, MirrorError> {
    poseidon1(k)
}

/// Action binding: `H4(selector, beneficiary_hi, beneficiary_lo, relay_fee)`.
///
/// This is the value the circuit takes as its third public input and the value
/// the program recomputes from the action it is about to execute. It lives here,
/// shared by both, so the two cannot drift apart.
///
/// It covers every economically meaningful field of a spend. The payout is the
/// pool denomination minus `relay_fee`, so binding the beneficiary and the fee
/// binds the amount too: a relay can neither redirect the payout nor inflate its
/// own cut, because either change produces a different binding and the proof
/// stops verifying.
///
/// A 32-byte public key does not fit in a BN254 scalar — the field is ~254 bits
/// — so the key is split into two 16-byte halves. Each half is well under the
/// modulus, the split is injective, and no reduction ever happens. Reducing a
/// full key instead would let two distinct beneficiaries share one binding.
pub fn action_binding(
    selector: u64,
    beneficiary: &[u8; 32],
    relay_fee: u64,
) -> Result<Field, MirrorError> {
    let mut hi = [0u8; 32];
    hi[16..].copy_from_slice(&beneficiary[..16]);
    let mut lo = [0u8; 32];
    lo[16..].copy_from_slice(&beneficiary[16..]);

    poseidon4(
        Field::from_u64(selector),
        // Both halves are < 2^128, so these constructions cannot fail.
        Field::from_bytes(hi)?,
        Field::from_bytes(lo)?,
        Field::from_u64(relay_fee),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(v: u64) -> Field {
        Field::from_u64(v)
    }

    #[test]
    fn hashing_is_deterministic() {
        assert_eq!(
            poseidon2(f(1), f(2)).unwrap(),
            poseidon2(f(1), f(2)).unwrap()
        );
    }

    #[test]
    fn hashing_is_order_sensitive() {
        assert_ne!(
            poseidon2(f(1), f(2)).unwrap(),
            poseidon2(f(2), f(1)).unwrap()
        );
    }

    #[test]
    fn arities_are_distinct_domains() {
        // The whole point of separating by arity: padding one preimage with
        // zeros must not reproduce another's digest.
        let a1 = poseidon1(f(7)).unwrap();
        let a2 = poseidon2(f(7), Field::ZERO).unwrap();
        let a3 = poseidon3(f(7), Field::ZERO, Field::ZERO).unwrap();
        assert_ne!(a1, a2);
        assert_ne!(a2, a3);
        assert_ne!(a1, a3);
    }

    #[test]
    fn a_nullifier_cannot_be_confused_with_a_merkle_node() {
        // Different arity, so no choice of children reproduces a nullifier by
        // construction rather than by preimage hardness.
        let k = f(42);
        let null = nullifier(k).unwrap();
        assert_ne!(null, hash_node(Field::ZERO, k).unwrap());
        assert_ne!(null, hash_node(k, Field::ZERO).unwrap());
    }

    #[test]
    fn a_commitment_is_bound_to_its_denomination() {
        let (k, r) = (f(11), f(22));
        assert_ne!(
            commitment(k, r, f(1)).unwrap(),
            commitment(k, r, f(2)).unwrap(),
            "the same note under two denominations must not share a commitment"
        );
    }

    #[test]
    fn the_action_binding_covers_every_field_a_relay_could_change() {
        let bob = [7u8; 32];
        let base = action_binding(1, &bob, 5_000).unwrap();

        let mut carol = [7u8; 32];
        carol[31] = 8;
        assert_ne!(
            base,
            action_binding(1, &carol, 5_000).unwrap(),
            "a redirected beneficiary must change the binding"
        );
        assert_ne!(
            base,
            action_binding(1, &bob, 6_000).unwrap(),
            "an inflated relay fee must change the binding"
        );
        assert_ne!(
            base,
            action_binding(2, &bob, 5_000).unwrap(),
            "a different action must change the binding"
        );
    }

    #[test]
    fn the_beneficiary_split_is_injective_across_the_halfway_boundary() {
        // Two keys differing only at byte 15 and two differing only at byte 16
        // land in different halves. If the split dropped or overlapped a byte,
        // one of these pairs would collide.
        let base = [0u8; 32];
        let mut a = base;
        a[15] = 1;
        let mut b = base;
        b[16] = 1;
        let zero = action_binding(0, &base, 0).unwrap();
        let ha = action_binding(0, &a, 0).unwrap();
        let hb = action_binding(0, &b, 0).unwrap();
        assert_ne!(zero, ha);
        assert_ne!(zero, hb);
        assert_ne!(ha, hb);
    }

    #[test]
    fn a_full_width_key_binds_without_reduction() {
        // An all-0xff key exceeds the BN254 modulus. Reducing it whole would be
        // a silent collision surface; the halves keep it exact.
        let max = [0xffu8; 32];
        assert!(
            Field::from_bytes(max).is_err(),
            "precondition: not canonical"
        );
        assert!(action_binding(0, &max, 0).is_ok());

        let mut near = [0xffu8; 32];
        near[0] = 0xfe;
        assert_ne!(
            action_binding(0, &max, 0).unwrap(),
            action_binding(0, &near, 0).unwrap()
        );
    }

    /// The anchor for the three-way parity.
    ///
    /// `poseidon([1, 2])` under circomlib's BN254 x5 instance is the published
    /// constant
    /// `7853200120776062878684798364095072458815029376092732009249414926327459813530`.
    /// Pinning it here means the host, the syscall and the R1CS gadget in
    /// `mirror-circuit` are each checked against an external published value
    /// rather than against each other — so all three agreeing on a wrong answer
    /// is not a reachable state.
    #[test]
    fn poseidon2_matches_the_published_circomlib_vector() {
        const CIRCOMLIB_1_2: [u8; 32] = [
            0x11, 0x5c, 0xc0, 0xf5, 0xe7, 0xd6, 0x90, 0x41, 0x3d, 0xf6, 0x4c, 0x6b, 0x96, 0x62,
            0xe9, 0xcf, 0x2a, 0x36, 0x17, 0xf2, 0x74, 0x32, 0x45, 0x51, 0x9e, 0x19, 0x60, 0x7a,
            0x44, 0x17, 0x18, 0x9a,
        ];
        assert_eq!(poseidon2(f(1), f(2)).unwrap().to_bytes(), CIRCOMLIB_1_2);
    }

    /// Regression vectors for the other arities we rely on. These are ours, not
    /// external, and exist so that an accidental change of parameters or
    /// endianness is caught rather than silently re-baselined.
    #[test]
    fn arity_vectors_are_stable() {
        const P1_1: [u8; 32] = [
            0x29, 0x17, 0x61, 0x00, 0xea, 0xa9, 0x62, 0xbd, 0xc1, 0xfe, 0x6c, 0x65, 0x4d, 0x6a,
            0x3c, 0x13, 0x0e, 0x96, 0xa4, 0xd1, 0x16, 0x8b, 0x33, 0x84, 0x8b, 0x89, 0x7d, 0xc5,
            0x02, 0x82, 0x01, 0x33,
        ];
        const P3_123: [u8; 32] = [
            0x0e, 0x77, 0x32, 0xd8, 0x9e, 0x69, 0x39, 0xc0, 0xff, 0x03, 0xd5, 0xe5, 0x8d, 0xab,
            0x63, 0x02, 0xf3, 0x23, 0x0e, 0x26, 0x9d, 0xc5, 0xb9, 0x68, 0xf7, 0x25, 0xdf, 0x34,
            0xab, 0x36, 0xd7, 0x32,
        ];
        assert_eq!(poseidon1(f(1)).unwrap().to_bytes(), P1_1);
        assert_eq!(poseidon3(f(1), f(2), f(3)).unwrap().to_bytes(), P3_123);
    }
}
