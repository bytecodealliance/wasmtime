//! Array-based data structures using densely numbered entity references as mapping keys.
//!
//! This module defines a number of data structures based on arrays. The arrays are not indexed by
//! `usize` as usual, but by *entity references* which are integers wrapped in new-types. This has
//! a couple advantages:
//!
//! - Improved type safety. The various map and set types accept a specific key type, so there is
//!   no confusion about the meaning of an array index, as there is with plain arrays.
//! - Smaller indexes. The normal `usize` index is often 64 bits which is way too large for most
//!   purposes. The entity reference types can be smaller, allowing for more compact data
//!   structures.
//!
//! The `EntityRef` trait should be implemented by types to be used as indexed. The `entity_impl!`
//! macro provides convenient defaults for types wrapping `u32` which is common.
//!
//! - [`PrimaryMap`](struct.PrimaryMap.html) is used to keep track of a vector of entities,
//!   assigning a unique entity reference to each.
//! - [`EntityMap`](struct.EntityMap.html) is used to associate secondary information to an entity.
//!   The map is implemented as a simple vector, so it does not keep track of which entities have
//!   been inserted. Instead, any unknown entities map to the default value.
//! - [`SparseMap`](struct.SparseMap.html) is used to associate secondary information to a small
//!   number of entities. It tracks accurately which entities have been inserted. This is a
//!   specialized data structure which can use a lot of memory, so read the documentation before
//!   using it.
//! - [`EntitySet`](struct.EntitySet.html) is used to represent a secondary set of entities.
//!   The set is implemented as a simple vector, so it does not keep track of which entities have
//!   been inserted into the primary map. Instead, any unknown entities are not in the set.
//! - [`EntityList`](struct.EntityList.html) is a compact representation of lists of entity
//!   references allocated from an associated memory pool. It has a much smaller footprint than
//!   `Vec`.

mod iter;
mod keys;
mod list;
mod map;
mod primary;
mod set;
mod sparse;

pub use self::iter::{Iter, IterMut};
pub use self::keys::Keys;
pub use self::list::{EntityList, ListPool};
pub use self::map::EntityMap;
pub use self::primary::PrimaryMap;
pub use self::set::EntitySet;
pub use self::sparse::{SparseMap, SparseMapValue, SparseSet};

/// A type wrapping a small integer index should implement `EntityRef` so it can be used as the key
/// of an `EntityMap` or `SparseMap`.
pub trait EntityRef: Copy + Eq {
    /// Create a new entity reference from a small integer.
    /// This should crash if the requested index is not representable.
    fn new(usize) -> Self;

    /// Get the index that was used to create this entity reference.
    fn index(self) -> usize;
}

/// Macro which provides the common implementation of a 32-bit entity reference.
#[macro_export]
macro_rules! entity_impl {
    // Basic traits.
    ($entity:ident) => {
        impl $crate::entity::EntityRef for $entity {
            fn new(index: usize) -> Self {
                debug_assert!(index < (::std::u32::MAX as usize));
                $entity(index as u32)
            }

            fn index(self) -> usize {
                self.0 as usize
            }
        }

        impl $crate::packed_option::ReservedValue for $entity {
            fn reserved_value() -> $entity {
                $entity(::std::u32::MAX)
            }
        }
    };

    // Include basic `Display` impl using the given display prefix.
    // Display an `Ebb` reference as "ebb12".
    ($entity:ident, $display_prefix:expr) => {
        entity_impl!($entity);

        impl ::std::fmt::Display for $entity {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                write!(f, "{}{}", $display_prefix, self.0)
            }
        }

        impl ::std::fmt::Debug for $entity {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                (self as &::std::fmt::Display).fmt(f)
            }
        }
    };
}
