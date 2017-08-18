//! Densely numbered entity references as mapping keys.
//!
//! This module defines an `EntityRef` trait that should be implemented by reference types wrapping
//! a small integer index.
//!
//! Various data structures based on the entity references are defined in sub-modules.

mod keys;
mod list;
mod map;
mod primary;
mod sparse;

pub use self::keys::Keys;
pub use self::list::{EntityList, ListPool};
pub use self::map::EntityMap;
pub use self::primary::PrimaryMap;
pub use self::sparse::{SparseSet, SparseMap, SparseMapValue};

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
                assert!(index < (::std::u32::MAX as usize));
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
    }
}
