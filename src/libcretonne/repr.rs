//! Representation of Cretonne IL functions.

use types::{Type, FunctionName, Signature};
use entities::{Ebb, NO_EBB, Inst, NO_INST, Value, NO_VALUE, ExpandedValue, StackSlot};
use instructions::*;
use std::fmt::{self, Display, Formatter};
use std::ops::{Index, IndexMut};

/// A function.
///
/// The `Function` struct owns all of its instructions and extended basic blocks, and it works as a
/// container for those objects by implementing both `Index<Inst>` and `Index<Ebb>`.
///
#[derive(Debug)]
pub struct Function {
    /// Name of this function. Mostly used by `.cton` files.
    pub name: FunctionName,

    /// Signature of this function.
    signature: Signature,

    /// The entry block.
    pub entry_block: Ebb,

    /// Stack slots allocated in this function.
    stack_slots: Vec<StackSlotData>,

    /// Data about all of the instructions in the function. The instructions in this vector is not
    /// necessarily in program order. The `Inst` reference indexes into this vector.
    instructions: Vec<InstructionData>,

    /// Extended basic blocks in the function, not necessarily in program order. The `Ebb`
    /// reference indexes into this vector.
    extended_basic_blocks: Vec<EbbData>,

    /// Extended value table. Most `Value` references refer directly to their defining instruction.
    /// Others index into this table.
    extended_values: Vec<ValueData>,

    // Linked list nodes for the layout order of instructions. Forms a double linked list per EBB,
    // terminated in both ends by NO_INST.
    inst_order: Vec<InstNode>,
}

impl Function {
    /// Create a function with the given name and signature.
    pub fn with_name_signature(name: FunctionName, sig: Signature) -> Function {
        Function {
            name: name,
            signature: sig,
            entry_block: NO_EBB,
            stack_slots: Vec::new(),
            instructions: Vec::new(),
            extended_basic_blocks: Vec::new(),
            extended_values: Vec::new(),
            inst_order: Vec::new(),
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

    // Instructions.

    /// Create a new instruction.
    ///
    /// The instruction is allowed to produce at most one result as indicated by `data.ty`. Use
    /// `make_multi_inst()` to create instructions with multiple results.
    pub fn make_inst(&mut self, data: InstructionData) -> Inst {
        let inst = Inst::new(self.instructions.len());
        self.instructions.push(data);
        self.inst_order.push(InstNode {
            prev: NO_INST,
            next: NO_INST,
        });
        debug_assert_eq!(self.instructions.len(), self.inst_order.len());
        inst
    }

    /// Make an instruction that may produce multiple results.
    ///
    /// The type of the first result is `data.ty`. If the instruction generates more than one
    /// result, additional result types are in `extra_result_types`.
    ///
    /// Not all instruction formats can represent multiple result values. This function will panic
    /// if the format of `data` is insufficient.
    pub fn make_multi_inst(&mut self, data: InstructionData, extra_result_types: &[Type]) -> Inst {
        let inst = self.make_inst(data);

        if !extra_result_types.is_empty() {
            // Additional values form a linked list starting from the second result value. Generate
            // the list backwards so we don't have to modify value table entries in place. (This
            // causes additional result values to be numbered backwards which is not the aestetic
            // choice, but since it is only visible in extremely rare instructions with 3+ results,
            // we don't care).
            let mut head = NO_VALUE;
            for ty in extra_result_types.into_iter().rev() {
                head = self.make_value(ValueData::Def {
                    ty: *ty,
                    def: inst,
                    next: head,
                });
            }

            // Update the second_result pointer in `inst`.
            if let Some(second_result_ref) = self.instructions[inst.index()].second_result_mut() {
                *second_result_ref = head;
            } else {
                panic!("Instruction format doesn't allow multiple results.");
            }
        }

        inst
    }

    /// Get the first result of an instruction.
    ///
    /// If `Inst` doesn't produce any results, this returns a `Value` with a `VOID` type.
    pub fn first_result(&self, inst: Inst) -> Value {
        Value::new_direct(inst)
    }

    /// Iterate through all the results of an instruction.
    pub fn inst_results<'a>(&'a self, inst: Inst) -> Values<'a> {
        Values {
            func: self,
            cur: Value::new_direct(inst),
        }
    }

    // Basic blocks

    /// Create a new basic block.
    pub fn make_ebb(&mut self) -> Ebb {
        let ebb = Ebb::new(self.extended_basic_blocks.len());
        self.extended_basic_blocks.push(EbbData::new());
        ebb
    }

    /// Reference the representation of an EBB.
    fn ebb(&self, ebb: Ebb) -> &EbbData {
        &self.extended_basic_blocks[ebb.index()]
    }

    /// Mutably reference the representation of an EBB.
    fn ebb_mut(&mut self, ebb: Ebb) -> &mut EbbData {
        &mut self.extended_basic_blocks[ebb.index()]
    }

    /// Iterate over all the EBBs in order of creation.
    pub fn ebbs_numerically(&self) -> NumericalEbbs {
        NumericalEbbs {
            cur: 0,
            limit: self.extended_basic_blocks.len(),
        }
    }

    /// Append an argument with type `ty` to `ebb`.
    pub fn append_ebb_arg(&mut self, ebb: Ebb, ty: Type) -> Value {
        let val = self.make_value(ValueData::Argument {
            ty: ty,
            ebb: ebb,
            next: NO_VALUE,
        });

        let last_arg = self.ebb(ebb).last_arg;
        match last_arg.expand() {
            // If last_arg = NO_VALUE, we're adding the first EBB argument.
            ExpandedValue::None => self.ebb_mut(ebb).first_arg = val,
            ExpandedValue::Table(index) => {
                // Append to linked list of arguments.
                if let ValueData::Argument { ref mut next, .. } = self.extended_values[index] {
                    *next = val;
                } else {
                    panic!("wrong type of extended value referenced by Ebb::last_arg");
                }
            }
            ExpandedValue::Direct(_) => panic!("Direct value cannot appear as EBB argument"),
        }
        self.ebb_mut(ebb).last_arg = val;

        val
    }

    /// Iterate through the arguments to an EBB.
    pub fn ebb_args<'a>(&'a self, ebb: Ebb) -> Values<'a> {
        Values {
            func: self,
            cur: self.ebb(ebb).first_arg,
        }
    }

    /// Append an instruction to a basic block.
    pub fn append_inst(&mut self, ebb: Ebb, inst: Inst) {
        let old_last = self[ebb].last_inst;

        self.inst_order[inst.index()] = InstNode {
            prev: old_last,
            next: NO_INST,
        };

        if old_last == NO_INST {
            assert!(self[ebb].first_inst == NO_INST);
            self[ebb].first_inst = inst;
        } else {
            self.inst_order[old_last.index()].next = inst;
        }
        self[ebb].last_inst = inst;
    }

    /// Iterate through the instructions in `ebb`.
    pub fn ebb_insts<'a>(&'a self, ebb: Ebb) -> EbbInsts<'a> {
        EbbInsts {
            func: self,
            cur: self[ebb].first_inst,
        }
    }

    // Values.

    /// Allocate an extended value entry.
    fn make_value(&mut self, data: ValueData) -> Value {
        let vref = Value::new_table(self.extended_values.len());
        self.extended_values.push(data);
        vref
    }

    /// Get the type of a value.
    pub fn value_type(&self, v: Value) -> Type {
        use entities::ExpandedValue::*;
        use self::ValueData::*;
        match v.expand() {
            Direct(i) => self[i].first_type(),
            Table(i) => {
                match self.extended_values[i] {
                    Def { ty, .. } => ty,
                    Argument { ty, .. } => ty,
                }
            }
            None => panic!("NO_VALUE has no type"),
        }
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

// ====--------------------------------------------------------------------------------------====//
//
// Extended basic block implementation.
//
// ====--------------------------------------------------------------------------------------====//

/// Contents of an extended basic block.
///
/// Arguments for an extended basic block are values that dominate everything in the EBB. All
/// branches to this EBB must provide matching arguments, and the arguments to the entry EBB must
/// match the function arguments.
#[derive(Debug)]
pub struct EbbData {
    /// First argument to this EBB, or `NO_VALUE` if the block has no arguments.
    ///
    /// The arguments are all ValueData::Argument entries that form a linked list from `first_arg`
    /// to `last_arg`.
    first_arg: Value,

    /// Last argument to this EBB, or `NO_VALUE` if the block has no arguments.
    last_arg: Value,

    /// First instruction in this block, or `NO_INST`.
    first_inst: Inst,

    /// Last instruction in this block, or `NO_INST`.
    last_inst: Inst,
}

impl EbbData {
    fn new() -> EbbData {
        EbbData {
            first_arg: NO_VALUE,
            last_arg: NO_VALUE,
            first_inst: NO_INST,
            last_inst: NO_INST,
        }
    }
}

impl Index<Ebb> for Function {
    type Output = EbbData;

    fn index<'a>(&'a self, ebb: Ebb) -> &'a EbbData {
        &self.extended_basic_blocks[ebb.index()]
    }
}

impl IndexMut<Ebb> for Function {
    fn index_mut<'a>(&'a mut self, ebb: Ebb) -> &'a mut EbbData {
        &mut self.extended_basic_blocks[ebb.index()]
    }
}

pub struct EbbInsts<'a> {
    func: &'a Function,
    cur: Inst,
}

impl<'a> Iterator for EbbInsts<'a> {
    type Item = Inst;

    fn next(&mut self) -> Option<Self::Item> {
        let prev = self.cur;
        if prev == NO_INST {
            None
        } else {
            // Advance self.cur to the next inst.
            self.cur = self.func.inst_order[prev.index()].next;
            Some(prev)
        }
    }
}

/// Iterate through all EBBs in a function in numerical order.
/// This order is stable, but has little significance to the semantics of the function.
pub struct NumericalEbbs {
    cur: usize,
    limit: usize,
}

impl Iterator for NumericalEbbs {
    type Item = Ebb;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur < self.limit {
            let prev = Ebb::new(self.cur);
            self.cur += 1;
            Some(prev)
        } else {
            None
        }
    }
}

// ====--------------------------------------------------------------------------------------====//
//
// Instruction implementation.
//
// The InstructionData layout is defined in the `instructions` module.
//
// ====--------------------------------------------------------------------------------------====//

/// Allow immutable access to instructions via function indexing.
impl Index<Inst> for Function {
    type Output = InstructionData;

    fn index<'a>(&'a self, inst: Inst) -> &'a InstructionData {
        &self.instructions[inst.index()]
    }
}

/// A node in a double linked list of instructions is a basic block.
#[derive(Debug)]
struct InstNode {
    prev: Inst,
    next: Inst,
}

// ====--------------------------------------------------------------------------------------====//
//
// Value implementation.
//
// ====--------------------------------------------------------------------------------------====//

// Most values are simply the first value produced by an instruction.
// Other values have an entry in the value table.
#[derive(Debug)]
enum ValueData {
    // Value is defined by an instruction, but it is not the first result.
    Def {
        ty: Type,
        def: Inst,
        next: Value, // Next result defined by `def`.
    },

    // Value is an EBB argument.
    Argument {
        ty: Type,
        ebb: Ebb,
        next: Value, // Next argument to `ebb`.
    },
}

/// Iterate through a list of related value references, such as:
///
/// - All results defined by an instruction.
/// - All arguments to an EBB
///
/// A value iterator borrows a Function reference.
pub struct Values<'a> {
    func: &'a Function,
    cur: Value,
}

impl<'a> Iterator for Values<'a> {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        let prev = self.cur;

        // Advance self.cur to the next value, or NO_VALUE.
        self.cur = match prev.expand() {
            ExpandedValue::Direct(inst) => self.func[inst].second_result().unwrap_or_default(),
            ExpandedValue::Table(index) => {
                match self.func.extended_values[index] {
                    ValueData::Def { next, .. } => next,
                    ValueData::Argument { next, .. } => next,
                }
            }
            ExpandedValue::None => return None,
        };

        Some(prev)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use types;
    use instructions::*;

    #[test]
    fn make_inst() {
        let mut func = Function::new();

        let idata = InstructionData::Nullary {
            opcode: Opcode::Iconst,
            ty: types::I32,
        };
        let inst = func.make_inst(idata);
        assert_eq!(inst.to_string(), "inst0");

        // Immutable reference resolution.
        let ins = &func[inst];
        assert_eq!(ins.opcode(), Opcode::Iconst);
        assert_eq!(ins.first_type(), types::I32);
    }

    #[test]
    fn multiple_results() {
        use types::*;
        let mut func = Function::new();

        let idata = InstructionData::call(Opcode::Vconst, I64);
        let inst = func.make_multi_inst(idata, &[I8, F64]);
        assert_eq!(inst.to_string(), "inst0");
        let results: Vec<Value> = func.inst_results(inst).collect();
        assert_eq!(results.len(), 3);
        assert_eq!(func.value_type(results[0]), I64);
        assert_eq!(func.value_type(results[1]), I8);
        assert_eq!(func.value_type(results[2]), F64);
    }

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

    #[test]
    fn ebb() {
        let mut func = Function::new();

        assert_eq!(func.ebbs_numerically().next(), None);

        let ebb = func.make_ebb();
        assert_eq!(ebb.to_string(), "ebb0");
        assert_eq!(func.ebb_args(ebb).next(), None);

        let arg1 = func.append_ebb_arg(ebb, types::F32);
        assert_eq!(arg1.to_string(), "vx0");
        {
            let mut args1 = func.ebb_args(ebb);
            assert_eq!(args1.next(), Some(arg1));
            assert_eq!(args1.next(), None);
        }
        let arg2 = func.append_ebb_arg(ebb, types::I16);
        assert_eq!(arg2.to_string(), "vx1");
        {
            let mut args2 = func.ebb_args(ebb);
            assert_eq!(args2.next(), Some(arg1));
            assert_eq!(args2.next(), Some(arg2));
            assert_eq!(args2.next(), None);
        }

        // The numerical ebb iterator doesn't capture the function.
        let mut ebbs = func.ebbs_numerically();
        assert_eq!(ebbs.next(), Some(ebb));
        assert_eq!(ebbs.next(), None);

        assert_eq!(func.ebb_insts(ebb).next(), None);

        let inst = func.make_inst(InstructionData::Nullary {
            opcode: Opcode::Iconst,
            ty: types::I32,
        });
        func.append_inst(ebb, inst);
        {
            let mut ii = func.ebb_insts(ebb);
            assert_eq!(ii.next(), Some(inst));
            assert_eq!(ii.next(), None);
        }
        assert_eq!(func[ebb].first_inst, inst);
        assert_eq!(func[ebb].last_inst, inst);

        let inst2 = func.make_inst(InstructionData::Nullary {
            opcode: Opcode::Iconst,
            ty: types::I32,
        });
        func.append_inst(ebb, inst2);
        {
            let mut ii = func.ebb_insts(ebb);
            assert_eq!(ii.next(), Some(inst));
            assert_eq!(ii.next(), Some(inst2));
            assert_eq!(ii.next(), None);
        }
        assert_eq!(func[ebb].first_inst, inst);
        assert_eq!(func[ebb].last_inst, inst2);
    }
}
