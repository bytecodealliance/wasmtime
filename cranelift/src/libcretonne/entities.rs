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
//! data structures use the sentinen constant, while function arguments and return values prefer
//! the more Rust-like `Option<EntityRef>` variant.
//!
//! The entity references all implement the `Display` trait in a way that matches the textual IL
//! format.

use std::default::Default;
use std::fmt::{self, Display, Formatter, Write};
use std::u32;

/// An opaque reference to an extended basic block in a function.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Ebb(u32);

impl Ebb {
    pub fn new(index: usize) -> Ebb {
        assert!(index < (u32::MAX as usize));
        Ebb(index as u32)
    }

    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

/// Display an `Ebb` reference as "ebb12".
impl Display for Ebb {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "ebb{}", self.0)
    }
}

/// A guaranteed invalid EBB reference.
pub const NO_EBB: Ebb = Ebb(u32::MAX);

impl Default for Ebb {
    fn default() -> Ebb {
        NO_EBB
    }
}


/// An opaque reference to an instruction in a function.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Inst(u32);

impl Inst {
    pub fn new(index: usize) -> Inst {
        assert!(index < (u32::MAX as usize));
        Inst(index as u32)
    }

    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

/// Display an `Inst` reference as "inst7".
impl Display for Inst {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "inst{}", self.0)
    }
}

/// A guaranteed invalid instruction reference.
pub const NO_INST: Inst = Inst(u32::MAX);

impl Default for Inst {
    fn default() -> Inst {
        NO_INST
    }
}


/// An opaque reference to an SSA value.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Value(u32);

// Value references can either reference an instruction directly, or they can refer to the extended
// value table.
pub enum ExpandedValue {
    // This is the first value produced by the referenced instruction.
    Direct(Inst),

    // This value is described in the extended value table.
    Table(usize),

    // This is NO_VALUE.
    None,
}

impl Value {
    pub fn direct_from_number(n: u32) -> Value {
        let encoding = n * 2;
        assert!(encoding < u32::MAX);
        Value(encoding)
    }

    pub fn new_direct(i: Inst) -> Value {
        let encoding = i.index() * 2;
        assert!(encoding < u32::MAX as usize);
        Value(encoding as u32)
    }

    pub fn new_table(index: usize) -> Value {
        let encoding = index * 2 + 1;
        assert!(encoding < u32::MAX as usize);
        Value(encoding as u32)
    }

    // Expand the internal representation into something useful.
    pub fn expand(&self) -> ExpandedValue {
        use self::ExpandedValue::*;
        if *self == NO_VALUE {
            return None;
        }
        let index = (self.0 / 2) as usize;
        if self.0 % 2 == 0 {
            Direct(Inst::new(index))
        } else {
            Table(index)
        }
    }
}

/// Display a `Value` reference as "v7" or "v2x".
impl Display for Value {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        use self::ExpandedValue::*;
        match self.expand() {
            Direct(i) => write!(fmt, "v{}", i.0),
            Table(i) => write!(fmt, "vx{}", i),
            None => write!(fmt, "NO_VALUE"),
        }
    }
}

/// A guaranteed invalid value reference.
pub const NO_VALUE: Value = Value(u32::MAX);

impl Default for Value {
    fn default() -> Value {
        NO_VALUE
    }
}

/// An opaque reference to a stack slot.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct StackSlot(u32);

impl StackSlot {
    pub fn new(index: usize) -> StackSlot {
        assert!(index < (u32::MAX as usize));
        StackSlot(index as u32)
    }

    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

/// Display a `StackSlot` reference as "ss12".
impl Display for StackSlot {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "ss{}", self.0)
    }
}

/// A guaranteed invalid stack slot reference.
pub const NO_STACK_SLOT: StackSlot = StackSlot(u32::MAX);

impl Default for StackSlot {
    fn default() -> StackSlot {
        NO_STACK_SLOT
    }
}
