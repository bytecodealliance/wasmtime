//! Array-based data structures using densely numbered entity references as mapping keys.
//!
//! This crate defines a number of data structures based on arrays. The arrays are not indexed by
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
//! - [`SecondaryMap`](struct.SecondaryMap.html) is used to associate secondary information to an
//!   entity. The map is implemented as a simple vector, so it does not keep track of which
//!   entities have been inserted. Instead, any unknown entities map to the default value.
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

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::new_without_default))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        clippy::float_arithmetic,
        clippy::mut_mut,
        clippy::nonminimal_bool,
        clippy::option_map_unwrap_or,
        clippy::option_map_unwrap_or_else,
        clippy::print_stdout,
        clippy::unicode_not_nfc,
        clippy::use_self
    )
)]
#![no_std]

extern crate alloc;

// Re-export core so that the macros works with both std and no_std crates
#[doc(hidden)]
pub extern crate core as __core;

/// A type wrapping a small integer index should implement `EntityRef` so it can be used as the key
/// of an `SecondaryMap` or `SparseMap`.
pub trait EntityRef: Copy + Eq {
    /// Create a new entity reference from a small integer.
    /// This should crash if the requested index is not representable.
    fn new(_: usize) -> Self;

    /// Get the index that was used to create this entity reference.
    fn index(self) -> usize;
}

/// Macro which provides the common implementation of a 32-bit entity reference.
#[macro_export]
macro_rules! entity_impl {
    // Basic traits.
    ($entity:ident) => {
        impl $crate::EntityRef for $entity {
            fn new(index: usize) -> Self {
                debug_assert!(index < ($crate::__core::u32::MAX as usize));
                $entity(index as u32)
            }

            fn index(self) -> usize {
                self.0 as usize
            }
        }

        impl $crate::packed_option::ReservedValue for $entity {
            fn reserved_value() -> $entity {
                $entity($crate::__core::u32::MAX)
            }

            fn is_reserved_value(&self) -> bool {
                self.0 == $crate::__core::u32::MAX
            }
        }

        impl $entity {
            /// Return the underlying index value as a `u32`.
            #[allow(dead_code)]
            pub fn from_u32(x: u32) -> Self {
                debug_assert!(x < $crate::__core::u32::MAX);
                $entity(x)
            }

            /// Return the underlying index value as a `u32`.
            #[allow(dead_code)]
            pub fn as_u32(self) -> u32 {
                self.0
            }
        }
    };

    // Include basic `Display` impl using the given display prefix.
    // Display a `Block` reference as "block12".
    ($entity:ident, $display_prefix:expr) => {
        entity_impl!($entity);

        impl $crate::__core::fmt::Display for $entity {
            fn fmt(&self, f: &mut $crate::__core::fmt::Formatter) -> $crate::__core::fmt::Result {
                write!(f, concat!($display_prefix, "{}"), self.0)
            }
        }

        impl $crate::__core::fmt::Debug for $entity {
            fn fmt(&self, f: &mut $crate::__core::fmt::Formatter) -> $crate::__core::fmt::Result {
                (self as &dyn $crate::__core::fmt::Display).fmt(f)
            }
        }
    };
}

pub mod packed_option;

mod boxed_slice;
mod iter;
mod keys;
mod list;
mod map;
mod primary;
mod set;
mod sparse;

pub use self::boxed_slice::BoxedSlice;
pub use self::iter::{Iter, IterMut};
pub use self::keys::Keys;
pub use self::list::{EntityList, ListPool};
pub use self::map::SecondaryMap;
pub use self::primary::PrimaryMap;
pub use self::set::EntitySet;
pub use self::sparse::{SparseMap, SparseMapValue, SparseSet};
