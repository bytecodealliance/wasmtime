//! egraph library.
//!
//! This library is heavily inspired from the `egg` crate, and has an
//! implementation based on the algorithms there and as described in
//! the associated paper [1].
//!
//! The main goal of this library is to be explicitly memory-efficient
//! and light on allocations. We need to be as fast and as small as
//! possible in order to minimize impact on compile time in a
//! production compiler.

use cranelift_entity::{entity_impl, packed_option::ReservedValue};
use std::fmt::Debug;
use std::hash::Hash;

mod bumpvec;
mod ctxhash;
mod egraph;
mod unionfind;

pub use bumpvec::{BumpArena, BumpSlice, BumpVec};
pub use ctxhash::{CtxEq, CtxHash, CtxHashMap};
pub use egraph::{EClass, EGraph, NewOrExisting, NodeIter};
pub use unionfind::UnionFind;

/// An eclass ID.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id(u32);
entity_impl!(Id, "eclass");

impl Id {
    pub fn invalid() -> Id {
        Self::reserved_value()
    }
}

/// A trait implemented by all "languages" (types that can be enodes).
pub trait Language: CtxEq<Self::Node, Self::Node> + CtxHash<Self::Node> {
    type Node: Debug;
    fn children<'a>(&'a self, node: &'a Self::Node) -> &'a [Id];
    fn children_mut<'a>(&'a mut self, ctx: &'a mut Self::Node) -> &'a mut [Id];
    fn needs_dedup(&self, node: &Self::Node) -> bool;
}
