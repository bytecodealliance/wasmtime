//! Data flow graph tracking Instructions, Values, and EBBs.

use ir::{Ebb, Inst, Value, Type};
use ir::entities::{NO_VALUE, ExpandedValue};
use ir::instructions::InstructionData;
use entity_map::{EntityMap, PrimaryEntityData};

use std::ops::{Index, IndexMut};
use std::u16;

/// A data flow graph defines all instuctions and extended basic blocks in a function as well as
/// the data flow dependencies between them. The DFG also tracks values which can be either
/// instruction results or EBB arguments.
///
/// The layout of EBBs in the function and of instructions in each EBB is recorded by the
/// `FunctionLayout` data structure which form the other half of the function representation.
///
#[derive(Clone)]
pub struct DataFlowGraph {
    /// Data about all of the instructions in the function, including opcodes and operands.
    /// The instructions in this map are not in program order. That is tracked by `Layout`, along
    /// with the EBB containing each instruction.
    insts: EntityMap<Inst, InstructionData>,

    /// Extended basic blocks in the function and their arguments.
    /// This map is not in program order. That is handled by `Layout`, and so is the sequence of
    /// instructions contained in each EBB.
    ebbs: EntityMap<Ebb, EbbData>,

    /// Extended value table. Most `Value` references refer directly to their defining instruction.
    /// Others index into this table.
    ///
    /// This is implemented directly with a `Vec` rather than an `EntityMap<Value, ...>` because
    /// the Value entity references can refer to two things -- an instruction or an extended value.
    extended_values: Vec<ValueData>,
}

impl PrimaryEntityData for InstructionData {}
impl PrimaryEntityData for EbbData {}

impl DataFlowGraph {
    /// Create a new empty `DataFlowGraph`.
    pub fn new() -> DataFlowGraph {
        DataFlowGraph {
            insts: EntityMap::new(),
            ebbs: EntityMap::new(),
            extended_values: Vec::new(),
        }
    }

    /// Get the total number of instructions created in this function, whether they are currently
    /// inserted in the layout or not.
    ///
    /// This is intended for use with `EntityMap::with_capacity`.
    pub fn num_insts(&self) -> usize {
        self.insts.len()
    }

    /// Get the total number of extended basic blocks created in this function, whether they are
    /// currently inserted in the layout or not.
    ///
    /// This is intended for use with `EntityMap::with_capacity`.
    pub fn num_ebbs(&self) -> usize {
        self.ebbs.len()
    }
}

/// Handling values.
///
/// Values are either EBB arguments or instruction results.
impl DataFlowGraph {
    // Allocate an extended value entry.
    fn make_value(&mut self, data: ValueData) -> Value {
        let vref = Value::new_table(self.extended_values.len());
        self.extended_values.push(data);
        vref
    }

    /// Get the type of a value.
    pub fn value_type(&self, v: Value) -> Type {
        use ir::entities::ExpandedValue::*;
        match v.expand() {
            Direct(i) => self.insts[i].first_type(),
            Table(i) => {
                match self.extended_values[i] {
                    ValueData::Inst { ty, .. } => ty,
                    ValueData::Arg { ty, .. } => ty,
                }
            }
            None => panic!("NO_VALUE has no type"),
        }
    }

    /// Get the definition of a value.
    ///
    /// This is either the instruction that defined it or the Ebb that has the value as an
    /// argument.
    pub fn value_def(&self, v: Value) -> ValueDef {
        use ir::entities::ExpandedValue::*;
        match v.expand() {
            Direct(inst) => ValueDef::Res(inst, 0),
            Table(idx) => {
                match self.extended_values[idx] {
                    ValueData::Inst { inst, num, .. } => ValueDef::Res(inst, num as usize),
                    ValueData::Arg { ebb, num, .. } => ValueDef::Arg(ebb, num as usize),
                }
            }
            None => panic!("NO_VALUE has no def"),
        }
    }
}

/// Where did a value come from?
#[derive(Debug, PartialEq, Eq)]
pub enum ValueDef {
    /// Value is the n'th result of an instruction.
    Res(Inst, usize),
    /// Value is the n'th argument to an EBB.
    Arg(Ebb, usize),
}

// Internal table storage for extended values.
#[derive(Clone)]
enum ValueData {
    // Value is defined by an instruction, but it is not the first result.
    Inst {
        ty: Type,
        num: u16, // Result number starting from 0.
        inst: Inst,
        next: Value, // Next result defined by `def`.
    },

    // Value is an EBB argument.
    Arg {
        ty: Type,
        num: u16, // Argument number, starting from 0.
        ebb: Ebb,
        next: Value, // Next argument to `ebb`.
    },
}

/// Iterate through a list of related value references, such as:
///
/// - All results defined by an instruction. See `DataFlowGraph::inst_results`.
/// - All arguments to an EBB. See `DataFlowGraph::ebb_args`.
///
/// A value iterator borrows a `DataFlowGraph` reference.
pub struct Values<'a> {
    dfg: &'a DataFlowGraph,
    cur: Value,
}

impl<'a> Iterator for Values<'a> {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        let prev = self.cur;

        // Advance self.cur to the next value, or NO_VALUE.
        self.cur = match prev.expand() {
            ExpandedValue::Direct(inst) => self.dfg.insts[inst].second_result().unwrap_or_default(),
            ExpandedValue::Table(index) => {
                match self.dfg.extended_values[index] {
                    ValueData::Inst { next, .. } => next,
                    ValueData::Arg { next, .. } => next,
                }
            }
            ExpandedValue::None => return None,
        };

        Some(prev)
    }
}

/// Instructions.
///
impl DataFlowGraph {
    /// Create a new instruction.
    ///
    /// The type of the first result is indicated by `data.ty`. If the instruction produces
    /// multiple results, also call `make_inst_results` to allocate value table entries.
    pub fn make_inst(&mut self, data: InstructionData) -> Inst {
        self.insts.push(data)
    }

    /// Create result values for an instruction that produces multiple results.
    ///
    /// Instructions that produce 0 or 1 result values only need to be created with `make_inst`. If
    /// the instruction may produce more than 1 result, call `make_inst_results` to allocate
    /// value table entries for the additional results.
    ///
    /// The result value types are determined from the instruction's value type constraints and the
    /// provided `ctrl_typevar` type for polymorphic instructions. For non-polymorphic
    /// instructions, `ctrl_typevar` is ignored, and `VOID` can be used.
    ///
    /// The type of the first result value is also set, even if it was already set in the
    /// `InstructionData` passed to `make_inst`. If this function is called with a single-result
    /// instruction, that is the only effect.
    ///
    /// Returns the number of results produced by the instruction.
    pub fn make_inst_results(&mut self, inst: Inst, ctrl_typevar: Type) -> usize {
        let constraints = self.insts[inst].opcode().constraints();
        let fixed_results = constraints.fixed_results();

        // Additional values form a linked list starting from the second result value. Generate
        // the list backwards so we don't have to modify value table entries in place. (This
        // causes additional result values to be numbered backwards which is not the aestetic
        // choice, but since it is only visible in extremely rare instructions with 3+ results,
        // we don't care).
        let mut head = NO_VALUE;
        let mut first_type = Type::default();

        // TBD: Function call return values for direct and indirect function calls.

        if fixed_results > 0 {
            for res_idx in (1..fixed_results).rev() {
                head = self.make_value(ValueData::Inst {
                    ty: constraints.result_type(res_idx, ctrl_typevar),
                    num: res_idx as u16,
                    inst: inst,
                    next: head,
                });
            }
            first_type = constraints.result_type(0, ctrl_typevar);
        }

        // Update the second_result pointer in `inst`.
        if head != NO_VALUE {
            *self.insts[inst]
                .second_result_mut()
                .expect("instruction format doesn't allow multiple results") = head;
        }
        *self.insts[inst].first_type_mut() = first_type;

        fixed_results
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
            dfg: self,
            cur: if self.insts[inst].first_type().is_void() {
                NO_VALUE
            } else {
                Value::new_direct(inst)
            },
        }
    }
}

/// Allow immutable access to instructions via indexing.
impl Index<Inst> for DataFlowGraph {
    type Output = InstructionData;

    fn index<'a>(&'a self, inst: Inst) -> &'a InstructionData {
        &self.insts[inst]
    }
}

/// Allow mutable access to instructions via indexing.
impl IndexMut<Inst> for DataFlowGraph {
    fn index_mut<'a>(&'a mut self, inst: Inst) -> &'a mut InstructionData {
        &mut self.insts[inst]
    }
}

/// Extended basic blocks.
impl DataFlowGraph {
    /// Create a new basic block.
    pub fn make_ebb(&mut self) -> Ebb {
        self.ebbs.push(EbbData::new())
    }

    /// Get the number of arguments on `ebb`.
    pub fn num_ebb_args(&self, ebb: Ebb) -> usize {
        let last_arg = self.ebbs[ebb].last_arg;
        match last_arg.expand() {
            ExpandedValue::None => 0,
            ExpandedValue::Table(idx) => {
                if let ValueData::Arg { num, .. } = self.extended_values[idx] {
                    num as usize + 1
                } else {
                    panic!("inconsistent value table entry for EBB arg");
                }
            }
            ExpandedValue::Direct(_) => panic!("inconsistent value table entry for EBB arg"),
        }
    }

    /// Append an argument with type `ty` to `ebb`.
    pub fn append_ebb_arg(&mut self, ebb: Ebb, ty: Type) -> Value {
        let num_args = self.num_ebb_args(ebb);
        assert!(num_args <= u16::MAX as usize, "Too many arguments to EBB");
        let val = self.make_value(ValueData::Arg {
            ty: ty,
            ebb: ebb,
            num: num_args as u16,
            next: NO_VALUE,
        });
        let last_arg = self.ebbs[ebb].last_arg;
        match last_arg.expand() {
            // If last_arg is NO_VALUE, we're adding the first EBB argument.
            ExpandedValue::None => {
                self.ebbs[ebb].first_arg = val;
            }
            // Append to linked list of arguments.
            ExpandedValue::Table(idx) => {
                if let ValueData::Arg { ref mut next, .. } = self.extended_values[idx] {
                    *next = val;
                } else {
                    panic!("inconsistent value table entry for EBB arg");
                }
            }
            ExpandedValue::Direct(_) => panic!("inconsistent value table entry for EBB arg"),
        };
        self.ebbs[ebb].last_arg = val;
        val
    }

    /// Iterate through the arguments to an EBB.
    pub fn ebb_args<'a>(&'a self, ebb: Ebb) -> Values<'a> {
        Values {
            dfg: self,
            cur: self.ebbs[ebb].first_arg,
        }
    }
}

// Contents of an extended basic block.
//
// Arguments for an extended basic block are values that dominate everything in the EBB. All
// branches to this EBB must provide matching arguments, and the arguments to the entry EBB must
// match the function arguments.
#[derive(Clone)]
struct EbbData {
    // First argument to this EBB, or `NO_VALUE` if the block has no arguments.
    //
    // The arguments are all ValueData::Argument entries that form a linked list from `first_arg`
    // to `last_arg`.
    first_arg: Value,

    // Last argument to this EBB, or `NO_VALUE` if the block has no arguments.
    last_arg: Value,
}

impl EbbData {
    fn new() -> EbbData {
        EbbData {
            first_arg: NO_VALUE,
            last_arg: NO_VALUE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ir::types;
    use ir::{Opcode, InstructionData};

    #[test]
    fn make_inst() {
        let mut dfg = DataFlowGraph::new();

        let idata = InstructionData::Nullary {
            opcode: Opcode::Iconst,
            ty: types::I32,
        };
        let inst = dfg.make_inst(idata);
        assert_eq!(inst.to_string(), "inst0");

        // Immutable reference resolution.
        {
            let immdfg = &dfg;
            let ins = &immdfg[inst];
            assert_eq!(ins.opcode(), Opcode::Iconst);
            assert_eq!(ins.first_type(), types::I32);
        }

        // Result iterator.
        let mut res = dfg.inst_results(inst);
        let val = res.next().unwrap();
        assert!(res.next().is_none());

        assert_eq!(dfg.value_def(val), ValueDef::Res(inst, 0));
        assert_eq!(dfg.value_type(val), types::I32);
    }

    #[test]
    fn no_results() {
        let mut dfg = DataFlowGraph::new();

        let idata = InstructionData::Nullary {
            opcode: Opcode::Trap,
            ty: types::VOID,
        };
        let inst = dfg.make_inst(idata);

        // Result iterator should be empty.
        let mut res = dfg.inst_results(inst);
        assert_eq!(res.next(), None);
    }

    #[test]
    fn ebb() {
        let mut dfg = DataFlowGraph::new();

        let ebb = dfg.make_ebb();
        assert_eq!(ebb.to_string(), "ebb0");
        assert_eq!(dfg.num_ebb_args(ebb), 0);
        assert_eq!(dfg.ebb_args(ebb).next(), None);

        let arg1 = dfg.append_ebb_arg(ebb, types::F32);
        assert_eq!(arg1.to_string(), "vx0");
        assert_eq!(dfg.num_ebb_args(ebb), 1);
        {
            let mut args1 = dfg.ebb_args(ebb);
            assert_eq!(args1.next(), Some(arg1));
            assert_eq!(args1.next(), None);
        }
        let arg2 = dfg.append_ebb_arg(ebb, types::I16);
        assert_eq!(arg2.to_string(), "vx1");
        assert_eq!(dfg.num_ebb_args(ebb), 2);
        {
            let mut args2 = dfg.ebb_args(ebb);
            assert_eq!(args2.next(), Some(arg1));
            assert_eq!(args2.next(), Some(arg2));
            assert_eq!(args2.next(), None);
        }

        assert_eq!(dfg.value_def(arg1), ValueDef::Arg(ebb, 0));
        assert_eq!(dfg.value_def(arg2), ValueDef::Arg(ebb, 1));
        assert_eq!(dfg.value_type(arg1), types::F32);
        assert_eq!(dfg.value_type(arg2), types::I16);
    }
}
