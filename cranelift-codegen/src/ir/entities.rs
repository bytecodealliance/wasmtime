//! Cranelift IR entity references.
//!
//! Instructions in Cranelift IR need to reference other entities in the function. This can be other
//! parts of the function like extended basic blocks or stack slots, or it can be external entities
//! that are declared in the function preamble in the text format.
//!
//! These entity references in instruction operands are not implemented as Rust references both
//! because Rust's ownership and mutability rules make it difficult, and because 64-bit pointers
//! take up a lot of space, and we want a compact in-memory representation. Instead, entity
//! references are structs wrapping a `u32` index into a table in the `Function` main data
//! structure. There is a separate index type for each entity type, so we don't lose type safety.
//!
//! The `entities` module defines public types for the entity references along with constants
//! representing an invalid reference. We prefer to use `Option<EntityRef>` whenever possible, but
//! unfortunately that type is twice as large as the 32-bit index type on its own. Thus, compact
//! data structures use the `PackedOption<EntityRef>` representation, while function arguments and
//! return values prefer the more Rust-like `Option<EntityRef>` variant.
//!
//! The entity references all implement the `Display` trait in a way that matches the textual IR
//! format.

use crate::entity::entity_impl;
use core::fmt;
use core::u32;
#[cfg(feature = "enable-serde")]
use serde::{Deserialize, Serialize};

/// An opaque reference to an extended basic block in a function.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Ebb(u32);
entity_impl!(Ebb, "ebb");

impl Ebb {
    /// Create a new EBB reference from its number. This corresponds to the `ebbNN` representation.
    ///
    /// This method is for use by the parser.
    pub fn with_number(n: u32) -> Option<Self> {
        if n < u32::MAX {
            Some(Ebb(n))
        } else {
            None
        }
    }
}

/// An opaque reference to an SSA value.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Value(u32);
entity_impl!(Value, "v");

impl Value {
    /// Create a value from its number representation.
    /// This is the number in the `vNN` notation.
    ///
    /// This method is for use by the parser.
    pub fn with_number(n: u32) -> Option<Self> {
        if n < u32::MAX / 2 {
            Some(Value(n))
        } else {
            None
        }
    }
}

/// An opaque reference to an instruction in a function.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Inst(u32);
entity_impl!(Inst, "inst");

/// An opaque reference to a stack slot.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct StackSlot(u32);
entity_impl!(StackSlot, "ss");

impl StackSlot {
    /// Create a new stack slot reference from its number.
    ///
    /// This method is for use by the parser.
    pub fn with_number(n: u32) -> Option<Self> {
        if n < u32::MAX {
            Some(StackSlot(n))
        } else {
            None
        }
    }
}

/// An opaque reference to a global value.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct GlobalValue(u32);
entity_impl!(GlobalValue, "gv");

impl GlobalValue {
    /// Create a new global value reference from its number.
    ///
    /// This method is for use by the parser.
    pub fn with_number(n: u32) -> Option<Self> {
        if n < u32::MAX {
            Some(GlobalValue(n))
        } else {
            None
        }
    }
}

/// An opaque reference to a jump table.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct JumpTable(u32);
entity_impl!(JumpTable, "jt");

impl JumpTable {
    /// Create a new jump table reference from its number.
    ///
    /// This method is for use by the parser.
    pub fn with_number(n: u32) -> Option<Self> {
        if n < u32::MAX {
            Some(JumpTable(n))
        } else {
            None
        }
    }
}

/// A reference to an external function.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct FuncRef(u32);
entity_impl!(FuncRef, "fn");

impl FuncRef {
    /// Create a new external function reference from its number.
    ///
    /// This method is for use by the parser.
    pub fn with_number(n: u32) -> Option<Self> {
        if n < u32::MAX {
            Some(FuncRef(n))
        } else {
            None
        }
    }
}

/// A reference to a function signature.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct SigRef(u32);
entity_impl!(SigRef, "sig");

impl SigRef {
    /// Create a new function signature reference from its number.
    ///
    /// This method is for use by the parser.
    pub fn with_number(n: u32) -> Option<Self> {
        if n < u32::MAX {
            Some(SigRef(n))
        } else {
            None
        }
    }
}

/// A reference to a heap.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Heap(u32);
entity_impl!(Heap, "heap");

impl Heap {
    /// Create a new heap reference from its number.
    ///
    /// This method is for use by the parser.
    pub fn with_number(n: u32) -> Option<Self> {
        if n < u32::MAX {
            Some(Heap(n))
        } else {
            None
        }
    }
}

/// A reference to a table.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Table(u32);
entity_impl!(Table, "table");

impl Table {
    /// Create a new table reference from its number.
    ///
    /// This method is for use by the parser.
    pub fn with_number(n: u32) -> Option<Self> {
        if n < u32::MAX {
            Some(Table(n))
        } else {
            None
        }
    }
}

/// A reference to any of the entities defined in this module.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum AnyEntity {
    /// The whole function.
    Function,
    /// An extended basic block.
    Ebb(Ebb),
    /// An instruction.
    Inst(Inst),
    /// An SSA value.
    Value(Value),
    /// A stack slot.
    StackSlot(StackSlot),
    /// A Global value.
    GlobalValue(GlobalValue),
    /// A jump table.
    JumpTable(JumpTable),
    /// An external function.
    FuncRef(FuncRef),
    /// A function call signature.
    SigRef(SigRef),
    /// A heap.
    Heap(Heap),
    /// A table.
    Table(Table),
}

impl fmt::Display for AnyEntity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AnyEntity::Function => write!(f, "function"),
            AnyEntity::Ebb(r) => r.fmt(f),
            AnyEntity::Inst(r) => r.fmt(f),
            AnyEntity::Value(r) => r.fmt(f),
            AnyEntity::StackSlot(r) => r.fmt(f),
            AnyEntity::GlobalValue(r) => r.fmt(f),
            AnyEntity::JumpTable(r) => r.fmt(f),
            AnyEntity::FuncRef(r) => r.fmt(f),
            AnyEntity::SigRef(r) => r.fmt(f),
            AnyEntity::Heap(r) => r.fmt(f),
            AnyEntity::Table(r) => r.fmt(f),
        }
    }
}

impl fmt::Debug for AnyEntity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        (self as &dyn fmt::Display).fmt(f)
    }
}

impl From<Ebb> for AnyEntity {
    fn from(r: Ebb) -> Self {
        AnyEntity::Ebb(r)
    }
}

impl From<Inst> for AnyEntity {
    fn from(r: Inst) -> Self {
        AnyEntity::Inst(r)
    }
}

impl From<Value> for AnyEntity {
    fn from(r: Value) -> Self {
        AnyEntity::Value(r)
    }
}

impl From<StackSlot> for AnyEntity {
    fn from(r: StackSlot) -> Self {
        AnyEntity::StackSlot(r)
    }
}

impl From<GlobalValue> for AnyEntity {
    fn from(r: GlobalValue) -> Self {
        AnyEntity::GlobalValue(r)
    }
}

impl From<JumpTable> for AnyEntity {
    fn from(r: JumpTable) -> Self {
        AnyEntity::JumpTable(r)
    }
}

impl From<FuncRef> for AnyEntity {
    fn from(r: FuncRef) -> Self {
        AnyEntity::FuncRef(r)
    }
}

impl From<SigRef> for AnyEntity {
    fn from(r: SigRef) -> Self {
        AnyEntity::SigRef(r)
    }
}

impl From<Heap> for AnyEntity {
    fn from(r: Heap) -> Self {
        AnyEntity::Heap(r)
    }
}

impl From<Table> for AnyEntity {
    fn from(r: Table) -> Self {
        AnyEntity::Table(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::u32;
    use std::string::ToString;

    #[test]
    fn value_with_number() {
        assert_eq!(Value::with_number(0).unwrap().to_string(), "v0");
        assert_eq!(Value::with_number(1).unwrap().to_string(), "v1");

        assert_eq!(Value::with_number(u32::MAX / 2), None);
        assert!(Value::with_number(u32::MAX / 2 - 1).is_some());
    }

    #[test]
    fn memory() {
        use crate::packed_option::PackedOption;
        use core::mem;
        // This is the whole point of `PackedOption`.
        assert_eq!(
            mem::size_of::<Value>(),
            mem::size_of::<PackedOption<Value>>()
        );
    }
}
