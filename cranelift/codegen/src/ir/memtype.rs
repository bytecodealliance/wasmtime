//! Definitions for "memory types" in CLIF.
//!
//! A memory type is a struct-like definition -- fields with offsets,
//! each field having a type and possibly an attached fact -- that we
//! can use in proof-carrying code to validate accesses to structs and
//! propagate facts onto the loaded values as well.

use crate::ir::pcc::Fact;
use crate::ir::Type;
use alloc::vec::Vec;

#[cfg(feature = "enable-serde")]
use serde_derive::{Deserialize, Serialize};

/// Data defining a memory type.
#[derive(Clone, PartialEq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct MemoryTypeData {
    /// Size of this type.
    pub size: u64,

    /// Fields in this type. Sorted by offset.
    pub fields: Vec<MemoryTypeField>,
}

/// One field in a memory type.
#[derive(Clone, PartialEq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct MemoryTypeField {
    /// The offset of this field in the memory type.
    pub offset: usize,
    /// The type of the value in this field. Accesses to the field
    /// must use this type (i.e., cannot bitcast/type-pun in memory).
    pub ty: Type,
    /// A proof-carrying-code fact about this value, if any.
    pub fact: Option<Fact>,
    /// Whether this field is read-only, i.e., stores should be
    /// disallowed.
    pub readonly: bool,
}
