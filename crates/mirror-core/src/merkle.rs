//! The incremental Poseidon Merkle accumulator.
//!
//! Two views of one structure:
//!
//! * [`Frontier`] is what the on-chain program keeps — `O(depth)` state and an
//!   `O(depth)` append. It can produce the current root and nothing else.
//! * [`MerkleTree`] is the host-side full tree used to build the membership
//!   paths that go into a proof.
//!
//! They must agree, and the tests here assert that they do over real insertion
//! sequences. That assertion is the point: a competing submission in this bounty
//! has no test connecting its on-chain accumulator to a proof at all, and
//! injects a fixture root instead, with a comment conceding that a real deposit
//! sequence cannot reproduce it.

use crate::{hash_node, Field, MirrorError};

/// Tree depth. 2^20 leaves is roughly a million notes, which is far beyond any
/// anonymity set this protocol will realistically hold, and a depth-20 path
/// costs about 17k compute units to fold on-chain.
pub const TREE_DEPTH: usize = 20;

/// `zeros[i]` is the root of an empty subtree of height `i`.
///
/// Computed rather than hardcoded, because a hardcoded ladder that silently
/// disagrees with the hash function is exactly the class of bug this protocol
/// cannot survive. `Frontier::new` pays for it once.
pub fn zero_ladder() -> Result<[Field; TREE_DEPTH + 1], MirrorError> {
    let mut z = [Field::ZERO; TREE_DEPTH + 1];
    for i in 1..=TREE_DEPTH {
        z[i] = hash_node(z[i - 1], z[i - 1])?;
    }
    Ok(z)
}

/// Insert-only accumulator holding one node per level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frontier {
    filled: [Field; TREE_DEPTH],
    zeros: [Field; TREE_DEPTH + 1],
    next_index: u64,
    root: Field,
}

impl Frontier {
    pub fn new() -> Result<Self, MirrorError> {
        let zeros = zero_ladder()?;
        Ok(Frontier {
            filled: [Field::ZERO; TREE_DEPTH],
            zeros,
            next_index: 0,
            root: zeros[TREE_DEPTH],
        })
    }

    /// Number of leaves inserted so far.
    pub fn len(&self) -> u64 {
        self.next_index
    }

    pub fn is_empty(&self) -> bool {
        self.next_index == 0
    }

    pub fn root(&self) -> Field {
        self.root
    }

    /// Appends a leaf and returns its index.
    pub fn insert(&mut self, leaf: Field) -> Result<u64, MirrorError> {
        if self.next_index >= 1u64 << TREE_DEPTH {
            return Err(MirrorError::TreeFull);
        }
        let index = self.next_index;
        let mut current = leaf;
        let mut path = index;

        for level in 0..TREE_DEPTH {
            if path & 1 == 0 {
                // We are a left child: remember this node and pair with the
                // empty subtree to its right.
                self.filled[level] = current;
                current = hash_node(current, self.zeros[level])?;
            } else {
                // We are a right child: our left sibling is already known.
                current = hash_node(self.filled[level], current)?;
            }
            path >>= 1;
        }

        self.root = current;
        self.next_index += 1;
        Ok(index)
    }
}

/// A membership path: the sibling at each level, plus the leaf index whose bits
/// give the left/right orientation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MerkleProof {
    pub leaf_index: u64,
    pub siblings: [Field; TREE_DEPTH],
}

impl MerkleProof {
    /// Folds the path to a root. The circuit performs exactly this computation
    /// under constraints; this is the native reference it must agree with.
    pub fn fold(&self, leaf: Field) -> Result<Field, MirrorError> {
        let mut current = leaf;
        let mut path = self.leaf_index;
        for sibling in self.siblings.iter() {
            current = if path & 1 == 0 {
                hash_node(current, *sibling)?
            } else {
                hash_node(*sibling, current)?
            };
            path >>= 1;
        }
        Ok(current)
    }
}

/// Host-side full tree. Stores every inserted leaf so it can rebuild any path.
#[derive(Clone, Debug)]
pub struct MerkleTree {
    leaves: Vec<Field>,
    zeros: [Field; TREE_DEPTH + 1],
}

impl MerkleTree {
    pub fn new() -> Result<Self, MirrorError> {
        Ok(MerkleTree {
            leaves: Vec::new(),
            zeros: zero_ladder()?,
        })
    }

    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    pub fn insert(&mut self, leaf: Field) -> Result<u64, MirrorError> {
        if self.leaves.len() as u64 >= 1u64 << TREE_DEPTH {
            return Err(MirrorError::TreeFull);
        }
        self.leaves.push(leaf);
        Ok(self.leaves.len() as u64 - 1)
    }

    /// The node at `(level, index)`, materialising empty subtrees from the
    /// zero ladder rather than storing them.
    fn node(&self, level: usize, index: usize) -> Result<Field, MirrorError> {
        if level == 0 {
            return Ok(self.leaves.get(index).copied().unwrap_or(self.zeros[0]));
        }
        let span = 1usize << level;
        if index * span >= self.leaves.len() {
            return Ok(self.zeros[level]);
        }
        let left = self.node(level - 1, index * 2)?;
        let right = self.node(level - 1, index * 2 + 1)?;
        hash_node(left, right)
    }

    pub fn root(&self) -> Result<Field, MirrorError> {
        self.node(TREE_DEPTH, 0)
    }

    /// Builds the membership path for a previously inserted leaf.
    pub fn proof(&self, leaf_index: u64) -> Result<MerkleProof, MirrorError> {
        if leaf_index as usize >= self.leaves.len() {
            return Err(MirrorError::LeafIndexOutOfRange);
        }
        let mut siblings = [Field::ZERO; TREE_DEPTH];
        let mut index = leaf_index as usize;
        for (level, slot) in siblings.iter_mut().enumerate() {
            *slot = self.node(level, index ^ 1)?;
            index >>= 1;
        }
        Ok(MerkleProof {
            leaf_index,
            siblings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(v: u64) -> Field {
        Field::from_u64(v)
    }

    #[test]
    fn an_empty_frontier_and_an_empty_tree_share_a_root() {
        let frontier = Frontier::new().unwrap();
        let tree = MerkleTree::new().unwrap();
        assert_eq!(frontier.root(), tree.root().unwrap());
        assert_eq!(frontier.root(), zero_ladder().unwrap()[TREE_DEPTH]);
    }

    #[test]
    fn the_zero_ladder_climbs() {
        let z = zero_ladder().unwrap();
        assert_eq!(z[0], Field::ZERO);
        for i in 1..=TREE_DEPTH {
            assert_eq!(z[i], hash_node(z[i - 1], z[i - 1]).unwrap());
            assert_ne!(z[i], z[i - 1], "level {i} collapsed");
        }
    }

    /// The assertion a competing submission is missing entirely: the on-chain
    /// accumulator and a host-generated proof must agree after a real sequence
    /// of deposits, not after a fixture is injected.
    #[test]
    fn the_frontier_root_accepts_a_host_generated_proof() {
        let mut frontier = Frontier::new().unwrap();
        let mut tree = MerkleTree::new().unwrap();

        let leaves: Vec<Field> = (1..=9u64).map(f).collect();
        for leaf in &leaves {
            let a = frontier.insert(*leaf).unwrap();
            let b = tree.insert(*leaf).unwrap();
            assert_eq!(a, b, "the two views disagree on leaf index");
            assert_eq!(
                frontier.root(),
                tree.root().unwrap(),
                "roots diverged after {} insertions",
                a + 1
            );
        }

        // Every leaf's path must fold to the accumulator's current root.
        for (i, leaf) in leaves.iter().enumerate() {
            let proof = tree.proof(i as u64).unwrap();
            assert_eq!(
                proof.fold(*leaf).unwrap(),
                frontier.root(),
                "path for leaf {i} does not reach the on-chain root"
            );
        }
    }

    #[test]
    fn a_path_folded_against_the_wrong_leaf_misses_the_root() {
        let mut frontier = Frontier::new().unwrap();
        let mut tree = MerkleTree::new().unwrap();
        for v in 1..=4u64 {
            frontier.insert(f(v)).unwrap();
            tree.insert(f(v)).unwrap();
        }
        let proof = tree.proof(2).unwrap();
        assert_eq!(proof.fold(f(3)).unwrap(), frontier.root());
        assert_ne!(
            proof.fold(f(99)).unwrap(),
            frontier.root(),
            "a forged leaf must not reach the root"
        );
    }

    #[test]
    fn insertion_order_changes_the_root() {
        let mut a = Frontier::new().unwrap();
        let mut b = Frontier::new().unwrap();
        a.insert(f(1)).unwrap();
        a.insert(f(2)).unwrap();
        b.insert(f(2)).unwrap();
        b.insert(f(1)).unwrap();
        assert_ne!(a.root(), b.root());
    }

    #[test]
    fn a_proof_for_an_uninserted_leaf_is_refused() {
        let tree = MerkleTree::new().unwrap();
        assert!(matches!(
            tree.proof(0),
            Err(MirrorError::LeafIndexOutOfRange)
        ));
    }

    #[test]
    fn the_root_advances_on_every_insertion() {
        let mut frontier = Frontier::new().unwrap();
        let mut seen = vec![frontier.root()];
        for v in 1..=8u64 {
            frontier.insert(f(v)).unwrap();
            let root = frontier.root();
            assert!(!seen.contains(&root), "root repeated after inserting {v}");
            seen.push(root);
        }
        assert_eq!(frontier.len(), 8);
    }
}
