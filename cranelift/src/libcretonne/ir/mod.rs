//! Representation of Cretonne IL functions.

pub mod entities;
pub mod types;
pub mod condcodes;
pub mod immediates;
pub mod instructions;
pub mod jumptable;
pub mod dfg;
pub mod layout;

use ir::types::{FunctionName, Signature};
use entity_map::{EntityRef, EntityMap};
use ir::entities::{StackSlot, JumpTable};
use ir::jumptable::JumpTableData;
use ir::dfg::DataFlowGraph;
use ir::layout::Layout;
use std::fmt::{self, Debug, Display, Formatter};
use std::ops::Index;

/// A function.
pub struct Function {
    /// Name of this function. Mostly used by `.cton` files.
    pub name: FunctionName,

    /// Signature of this function.
    signature: Signature,

    /// Stack slots allocated in this function.
    stack_slots: Vec<StackSlotData>,

    /// Jump tables used in this function.
    pub jump_tables: EntityMap<JumpTable, JumpTableData>,

    /// Data flow graph containing the primary definition of all instructions, EBBs and values.
    pub dfg: DataFlowGraph,

    /// Layout of EBBs and instructions in the function body.
    pub layout: Layout,
}

impl Function {
    /// Create a function with the given name and signature.
    pub fn with_name_signature(name: FunctionName, sig: Signature) -> Function {
        Function {
            name: name,
            signature: sig,
            stack_slots: Vec::new(),
            jump_tables: EntityMap::new(),
            dfg: DataFlowGraph::new(),
            layout: Layout::new(),
        }
    }

    /// Create a new empty, anomymous function.
    pub fn new() -> Function {
        Self::with_name_signature(FunctionName::new(), Signature::new())
    }

    /// Get the signature of this function.
    pub fn own_signature(&self) -> &Signature {
        &self.signature
    }

    // Stack slots.

    /// Allocate a new stack slot.
    pub fn make_stack_slot(&mut self, data: StackSlotData) -> StackSlot {
        let ss = StackSlot::new(self.stack_slots.len());
        self.stack_slots.push(data);
        ss
    }

    /// Iterate over all stack slots in function.
    pub fn stack_slot_iter(&self) -> StackSlotIter {
        StackSlotIter {
            cur: 0,
            end: self.stack_slots.len(),
        }
    }
}

impl Debug for Function {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        use write::function_to_string;
        fmt.write_str(&function_to_string(self))
    }
}

// ====--------------------------------------------------------------------------------------====//
//
// Stack slot implementation.
//
// ====--------------------------------------------------------------------------------------====//

/// Contents of a stack slot.
#[derive(Debug)]
pub struct StackSlotData {
    /// Size of stack slot in bytes.
    pub size: u32,
}

impl StackSlotData {
    /// Create a stack slot with the specified byte size.
    pub fn new(size: u32) -> StackSlotData {
        StackSlotData { size: size }
    }
}

impl Display for StackSlotData {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "stack_slot {}", self.size)
    }
}

/// Allow immutable access to stack slots via function indexing.
impl Index<StackSlot> for Function {
    type Output = StackSlotData;

    fn index<'a>(&'a self, ss: StackSlot) -> &'a StackSlotData {
        &self.stack_slots[ss.index()]
    }
}

/// Stack slot iterator visits all stack slots in a function, returning `StackSlot` references.
pub struct StackSlotIter {
    cur: usize,
    end: usize,
}

impl Iterator for StackSlotIter {
    type Item = StackSlot;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur < self.end {
            let ss = StackSlot::new(self.cur);
            self.cur += 1;
            Some(ss)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_slot() {
        let mut func = Function::new();

        let ss0 = func.make_stack_slot(StackSlotData::new(4));
        let ss1 = func.make_stack_slot(StackSlotData::new(8));
        assert_eq!(ss0.to_string(), "ss0");
        assert_eq!(ss1.to_string(), "ss1");

        assert_eq!(func[ss0].size, 4);
        assert_eq!(func[ss1].size, 8);
    }
}
