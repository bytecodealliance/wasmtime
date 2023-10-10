//! Definitions for "memory types" in CLIF.
//!
//! A memory type is a struct-like definition -- fields with offsets,
//! each field having a type and possibly an attached fact -- that we
//! can use in proof-carrying code to validate accesses to structs and
//! propagate facts onto the loaded values as well.

use crate::ir::entities::MemoryType;
use crate::ir::pcc::Fact;
use crate::ir::Type;
use alloc::vec::Vec;

#[cfg(feature = "enable-serde")]
use serde_derive::{Deserialize, Serialize};

/// Data defining a memory type.
#[derive(Clone, PartialEq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub enum MemoryTypeData {
    /// An aggregate consisting of certain fields at certain offsets.
    Struct {
        /// Size of this type.
        size: u64,

        /// Fields in this type. Sorted by offset.
        fields: Vec<MemoryTypeField>,
    },

    /// An aggregate consisting of a single element repeated at a
    /// certain stride, with a statically-known length (element
    /// count). Layout is assumed to be contiguous, with a stride
    /// equal to the element type's size (so, if the stride is greater
    /// than the struct's packed size, be sure to include padding in
    /// the memory-type definition).
    StaticArray {
        /// The element type. May be another array, a struct, etc.
        element: MemoryType,

        /// Number of elements.
        length: u64,
    },

    /// A single Cranelift primitive of the given type stored in
    /// memory.
    Primitive {
        /// The primitive type.
        ty: Type,
    },

    /// A type with no size.
    Empty,
}

impl std::default::Default for MemoryTypeData {
    fn default() -> Self {
        Self::Empty
    }
}

impl std::fmt::Display for MemoryTypeData {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Struct { size, fields } => {
                write!(f, "struct {size} {{")?;
                let mut first = true;
                for field in fields {
                    if first {
                        first = false;
                    } else {
                        write!(f, ",")?;
                    }
                    write!(f, " {}: {}", field.offset, field.ty)?;
                    if field.readonly {
                        write!(f, " readonly")?;
                    }
                    if let Some(fact) = &field.fact {
                        write!(f, " ! {}", fact)?;
                    }
                }
                write!(f, " }}")?;
                Ok(())
            }
            Self::StaticArray { element, length } => {
                write!(f, "static_array {element} * {length:#x}")
            }
            Self::Primitive { ty } => {
                write!(f, "primitive {ty}")
            }
            Self::Empty => {
                write!(f, "empty")
            }
        }
    }
}

/// One field in a memory type.
#[derive(Clone, PartialEq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct MemoryTypeField {
    /// The offset of this field in the memory type.
    pub offset: u64,
    /// The type of the value in this field. Accesses to the field
    /// must use this type (i.e., cannot bitcast/type-pun in memory).
    pub ty: MemoryType,
    /// A proof-carrying-code fact about this value, if any.
    pub fact: Option<Fact>,
    /// Whether this field is read-only, i.e., stores should be
    /// disallowed.
    pub readonly: bool,
}
