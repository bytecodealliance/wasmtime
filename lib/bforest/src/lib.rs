//! A forest of B+-trees.
//!
//! This crate provides a data structures representing a set of small ordered sets or maps.
//! It is implemented as a forest of B+-trees all allocating nodes out of the same pool.
//!
//! **These are not general purpose data structures that are somehow magically faster that the
//! standard library's `BTreeSet` and `BTreeMap` types.**
//!
//! The tradeoffs are different:
//!
//! - Keys and values are expected to be small and copyable. We optimize for 32-bit types.
//! - A comparator object is used to compare keys, allowing smaller "context free" keys.
//! - Empty trees have a very small 32-bit footprint.
//! - All the trees in a forest can be cleared in constant time.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "std", warn(unstable_features))]
#![cfg_attr(
    feature = "clippy",
    plugin(clippy(conf_file = "../../clippy.toml"))
)]
#![cfg_attr(
    feature = "cargo-clippy",
    allow(new_without_default, new_without_default_derive)
)]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        float_arithmetic,
        mut_mut,
        nonminimal_bool,
        option_map_unwrap_or,
        option_map_unwrap_or_else,
        print_stdout,
        unicode_not_nfc,
        use_self
    )
)]
// Turns on no_std and alloc features if std is not available.
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc))]

/// This replaces `std` in builds with `core`.
#[cfg(not(feature = "std"))]
mod std {
    extern crate alloc;
    pub use self::alloc::{boxed, string, vec};
    pub use core::*;
}

#[macro_use]
extern crate cranelift_entity as entity;
use entity::packed_option;

use std::borrow::BorrowMut;
use std::cmp::Ordering;

mod map;
mod node;
mod path;
mod pool;
mod set;

pub use self::map::{Map, MapCursor, MapForest, MapIter};
pub use self::set::{Set, SetCursor, SetForest, SetIter};

use self::node::NodeData;
use self::path::Path;
use self::pool::NodePool;

/// The maximum branching factor of an inner node in a B+-tree.
/// The minimum number of outgoing edges is `INNER_SIZE/2`.
const INNER_SIZE: usize = 8;

/// Given the worst case branching factor of `INNER_SIZE/2` = 4, this is the
/// worst case path length from the root node to a leaf node in a tree with 2^32
/// entries. We would run out of node references before we hit `MAX_PATH`.
const MAX_PATH: usize = 16;

/// Key comparator.
///
/// Keys don't need to implement `Ord`. They are compared using a comparator object which
/// provides a context for comparison.
pub trait Comparator<K>
where
    K: Copy,
{
    /// Compare keys `a` and `b`.
    ///
    /// This relation must provide a total ordering or the key space.
    fn cmp(&self, a: K, b: K) -> Ordering;

    /// Binary search for `k` in an ordered slice.
    ///
    /// Assume that `s` is already sorted according to this ordering, search for the key `k`.
    ///
    /// Returns `Ok(idx)` if `k` was found in the slice or `Err(idx)` with the position where it
    /// should be inserted to preserve the ordering.
    fn search(&self, k: K, s: &[K]) -> Result<usize, usize> {
        s.binary_search_by(|x| self.cmp(*x, k))
    }
}

/// Trivial comparator that doesn't actually provide any context.
impl<K> Comparator<K> for ()
where
    K: Copy + Ord,
{
    fn cmp(&self, a: K, b: K) -> Ordering {
        a.cmp(&b)
    }
}

/// Family of types shared by the map and set forest implementations.
trait Forest {
    /// The key type is present for both sets and maps.
    type Key: Copy;

    /// The value type is `()` for sets.
    type Value: Copy;

    /// An array of keys for the leaf nodes.
    type LeafKeys: Copy + BorrowMut<[Self::Key]>;

    /// An array of values for the leaf nodes.
    type LeafValues: Copy + BorrowMut<[Self::Value]>;

    /// Splat a single key into a whole array.
    fn splat_key(key: Self::Key) -> Self::LeafKeys;

    /// Splat a single value inst a whole array
    fn splat_value(value: Self::Value) -> Self::LeafValues;
}

/// A reference to a B+-tree node.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Node(u32);
entity_impl!(Node, "node");

/// Empty type to be used as the "value" in B-trees representing sets.
#[derive(Clone, Copy)]
struct SetValue();

/// Insert `x` into `s` at position `i`, pushing out the last element.
fn slice_insert<T: Copy>(s: &mut [T], i: usize, x: T) {
    for j in (i + 1..s.len()).rev() {
        s[j] = s[j - 1];
    }
    s[i] = x;
}

/// Shift elements in `s` to the left by `n` positions.
fn slice_shift<T: Copy>(s: &mut [T], n: usize) {
    for j in 0..s.len() - n {
        s[j] = s[j + n];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use entity::EntityRef;

    /// An opaque reference to an extended basic block in a function.
    #[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct Ebb(u32);
    entity_impl!(Ebb, "ebb");

    #[test]
    fn comparator() {
        let ebb1 = Ebb::new(1);
        let ebb2 = Ebb::new(2);
        let ebb3 = Ebb::new(3);
        let ebb4 = Ebb::new(4);
        let vals = [ebb1, ebb2, ebb4];
        let comp = ();
        assert_eq!(comp.search(ebb1, &vals), Ok(0));
        assert_eq!(comp.search(ebb3, &vals), Err(2));
        assert_eq!(comp.search(ebb4, &vals), Ok(2));
    }

    #[test]
    fn slice_insertion() {
        let mut a = ['a', 'b', 'c', 'd'];

        slice_insert(&mut a[0..1], 0, 'e');
        assert_eq!(a, ['e', 'b', 'c', 'd']);

        slice_insert(&mut a, 0, 'a');
        assert_eq!(a, ['a', 'e', 'b', 'c']);

        slice_insert(&mut a, 3, 'g');
        assert_eq!(a, ['a', 'e', 'b', 'g']);

        slice_insert(&mut a, 1, 'h');
        assert_eq!(a, ['a', 'h', 'e', 'b']);
    }

    #[test]
    fn slice_shifting() {
        let mut a = ['a', 'b', 'c', 'd'];

        slice_shift(&mut a[0..1], 1);
        assert_eq!(a, ['a', 'b', 'c', 'd']);

        slice_shift(&mut a[1..], 1);
        assert_eq!(a, ['a', 'c', 'd', 'd']);

        slice_shift(&mut a, 2);
        assert_eq!(a, ['d', 'd', 'd', 'd']);
    }
}
