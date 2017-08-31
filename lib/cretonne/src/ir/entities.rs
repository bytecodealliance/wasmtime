//! IL entity references.
//!
//! Instructions in Cretonne IL need to reference other entities in the function. This can be other
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
//! The entity references all implement the `Display` trait in a way that matches the textual IL
//! format.

use std::fmt;
use std::u32;

/// An opaque reference to an extended basic block in a function.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct Ebb(u32);
entity_impl!(Ebb, "ebb");

impl Ebb {
    /// Create a new EBB reference from its number. This corresponds to the `ebbNN` representation.
    ///
    /// This method is for use by the parser.
    pub fn with_number(n: u32) -> Option<Ebb> {
        if n < u32::MAX { Some(Ebb(n)) } else { None }
    }
}

/// An opaque reference to an SSA value.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct Value(u32);
entity_impl!(Value, "v");

impl Value {
    /// Create a value from its number representation.
    /// This is the number in the `vNN` notation.
    ///
    /// This method is for use by the parser.
    pub fn with_number(n: u32) -> Option<Value> {
        if n < u32::MAX / 2 {
            Some(Value(n))
        } else {
            None
        }
    }
}

/// An opaque reference to an instruction in a function.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct Inst(u32);
entity_impl!(Inst, "inst");

/// An opaque reference to a stack slot.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct StackSlot(u32);
entity_impl!(StackSlot, "ss");

/// An opaque reference to a global variable.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct GlobalVar(u32);
entity_impl!(GlobalVar, "gv");

impl GlobalVar {
    /// Create a new global variable reference from its number.
    ///
    /// This method is for use by the parser.
    pub fn with_number(n: u32) -> Option<GlobalVar> {
        if n < u32::MAX {
            Some(GlobalVar(n))
        } else {
            None
        }
    }
}

/// An opaque reference to a jump table.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct JumpTable(u32);
entity_impl!(JumpTable, "jt");

/// A reference to an external function.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct FuncRef(u32);
entity_impl!(FuncRef, "fn");

/// A reference to a function signature.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct SigRef(u32);
entity_impl!(SigRef, "sig");

/// A reference to a heap.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Heap(u32);
entity_impl!(Heap, "heap");

/// A reference to any of the entities defined in this module.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
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
    /// A Global variable.
    GlobalVar(GlobalVar),
    /// A jump table.
    JumpTable(JumpTable),
    /// An external function.
    FuncRef(FuncRef),
    /// A function call signature.
    SigRef(SigRef),
    /// A heap.
    Heap(Heap),
}

impl fmt::Display for AnyEntity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AnyEntity::Function => write!(f, "function"),
            AnyEntity::Ebb(r) => r.fmt(f),
            AnyEntity::Inst(r) => r.fmt(f),
            AnyEntity::Value(r) => r.fmt(f),
            AnyEntity::StackSlot(r) => r.fmt(f),
            AnyEntity::GlobalVar(r) => r.fmt(f),
            AnyEntity::JumpTable(r) => r.fmt(f),
            AnyEntity::FuncRef(r) => r.fmt(f),
            AnyEntity::SigRef(r) => r.fmt(f),
            AnyEntity::Heap(r) => r.fmt(f),
        }
    }
}

impl From<Ebb> for AnyEntity {
    fn from(r: Ebb) -> AnyEntity {
        AnyEntity::Ebb(r)
    }
}

impl From<Inst> for AnyEntity {
    fn from(r: Inst) -> AnyEntity {
        AnyEntity::Inst(r)
    }
}

impl From<Value> for AnyEntity {
    fn from(r: Value) -> AnyEntity {
        AnyEntity::Value(r)
    }
}

impl From<StackSlot> for AnyEntity {
    fn from(r: StackSlot) -> AnyEntity {
        AnyEntity::StackSlot(r)
    }
}

impl From<GlobalVar> for AnyEntity {
    fn from(r: GlobalVar) -> AnyEntity {
        AnyEntity::GlobalVar(r)
    }
}

impl From<JumpTable> for AnyEntity {
    fn from(r: JumpTable) -> AnyEntity {
        AnyEntity::JumpTable(r)
    }
}

impl From<FuncRef> for AnyEntity {
    fn from(r: FuncRef) -> AnyEntity {
        AnyEntity::FuncRef(r)
    }
}

impl From<SigRef> for AnyEntity {
    fn from(r: SigRef) -> AnyEntity {
        AnyEntity::SigRef(r)
    }
}

impl From<Heap> for AnyEntity {
    fn from(r: Heap) -> AnyEntity {
        AnyEntity::Heap(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::u32;

    #[test]
    fn value_with_number() {
        assert_eq!(Value::with_number(0).unwrap().to_string(), "v0");
        assert_eq!(Value::with_number(1).unwrap().to_string(), "v1");

        assert_eq!(Value::with_number(u32::MAX / 2), None);
        assert!(Value::with_number(u32::MAX / 2 - 1).is_some());
    }

    #[test]
    fn memory() {
        use std::mem;
        use packed_option::PackedOption;
        // This is the whole point of `PackedOption`.
        assert_eq!(
            mem::size_of::<Value>(),
            mem::size_of::<PackedOption<Value>>()
        );
    }
}
