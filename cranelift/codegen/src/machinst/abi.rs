//! ABI definitions.

use smallvec::SmallVec;

/// A small vector of instructions (with some reasonable size); appropriate for
/// a small fixed sequence implementing one operation.
pub type SmallInstVec<I> = SmallVec<[I; 4]>;
