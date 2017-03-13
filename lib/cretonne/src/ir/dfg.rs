//! Data flow graph tracking Instructions, Values, and EBBs.

use ir::{Ebb, Inst, Value, Type, SigRef, Signature, FuncRef, ValueListPool};
use ir::entities::ExpandedValue;
use ir::instructions::{Opcode, InstructionData, CallInfo};
use ir::extfunc::ExtFuncData;
use entity_map::{EntityMap, PrimaryEntityData};
use ir::builder::{InsertBuilder, ReplaceBuilder};
use ir::layout::Cursor;
use packed_option::PackedOption;

use std::ops::{Index, IndexMut};
use std::u16;

/// A data flow graph defines all instructions and extended basic blocks in a function as well as
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
    pub insts: EntityMap<Inst, InstructionData>,

    /// Memory pool of value lists referenced by instructions in `insts`.
    pub value_lists: ValueListPool,

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

    /// Function signature table. These signatures are referenced by indirect call instructions as
    /// well as the external function references.
    pub signatures: EntityMap<SigRef, Signature>,

    /// External function references. These are functions that can be called directly.
    pub ext_funcs: EntityMap<FuncRef, ExtFuncData>,
}

impl PrimaryEntityData for InstructionData {}
impl PrimaryEntityData for EbbData {}
impl PrimaryEntityData for Signature {}
impl PrimaryEntityData for ExtFuncData {}

impl DataFlowGraph {
    /// Create a new empty `DataFlowGraph`.
    pub fn new() -> DataFlowGraph {
        DataFlowGraph {
            insts: EntityMap::new(),
            value_lists: ValueListPool::new(),
            ebbs: EntityMap::new(),
            extended_values: Vec::new(),
            signatures: EntityMap::new(),
            ext_funcs: EntityMap::new(),
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

    /// Returns `true` if the given ebb reference is valid.
    pub fn ebb_is_valid(&self, ebb: Ebb) -> bool {
        self.ebbs.is_valid(ebb)
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

    /// Check if a value reference is valid.
    pub fn value_is_valid(&self, v: Value) -> bool {
        match v.expand() {
            ExpandedValue::Direct(inst) => self.insts.is_valid(inst),
            ExpandedValue::Table(index) => index < self.extended_values.len(),
        }
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
                    ValueData::Alias { ty, .. } => ty,
                }
            }
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
                    ValueData::Alias { original, .. } => {
                        // Make sure we only recurse one level. `resolve_aliases` has safeguards to
                        // detect alias loops without overrunning the stack.
                        self.value_def(self.resolve_aliases(original))
                    }
                }
            }
        }
    }

    /// Resolve value aliases.
    ///
    /// Find the original SSA value that `value` aliases.
    pub fn resolve_aliases(&self, value: Value) -> Value {
        use ir::entities::ExpandedValue::Table;
        let mut v = value;

        // Note that extended_values may be empty here.
        for _ in 0..1 + self.extended_values.len() {
            v = match v.expand() {
                Table(idx) => {
                    match self.extended_values[idx] {
                        ValueData::Alias { original, .. } => {
                            // Follow alias values.
                            original
                        }
                        _ => return v,
                    }
                }
                _ => return v,
            };
        }
        panic!("Value alias loop detected for {}", value);
    }

    /// Resolve value copies.
    ///
    /// Find the original definition of a value, looking through value aliases as well as
    /// copy/spill/fill instructions.
    pub fn resolve_copies(&self, value: Value) -> Value {
        use ir::entities::ExpandedValue::Direct;
        let mut v = self.resolve_aliases(value);

        for _ in 0..self.insts.len() {
            v = self.resolve_aliases(match v.expand() {
                Direct(inst) => {
                    match self[inst] {
                        InstructionData::Unary { opcode, arg, .. } => {
                            match opcode {
                                Opcode::Copy | Opcode::Spill | Opcode::Fill => arg,
                                _ => return v,
                            }
                        }
                        _ => return v,
                    }
                }
                _ => return v,
            });
        }
        panic!("Copy loop detected for {}", value);
    }

    /// Turn a value into an alias of another.
    ///
    /// Change the `dest` value to behave as an alias of `src`. This means that all uses of `dest`
    /// will behave as if they used that value `src`.
    ///
    /// The `dest` value cannot be a direct value defined as the first result of an instruction. To
    /// replace a direct value with `src`, its defining instruction should be replaced with a
    /// `copy src` instruction. See `replace()`.
    pub fn change_to_alias(&mut self, dest: Value, src: Value) {
        use ir::entities::ExpandedValue::Table;

        // Try to create short alias chains by finding the original source value.
        // This also avoids the creation of loops.
        let original = self.resolve_aliases(src);
        assert!(dest != original,
                "Aliasing {} to {} would create a loop",
                dest,
                src);
        let ty = self.value_type(original);
        assert_eq!(self.value_type(dest),
                   ty,
                   "Aliasing {} to {} would change its type {} to {}",
                   dest,
                   src,
                   self.value_type(dest),
                   ty);

        if let Table(idx) = dest.expand() {
            self.extended_values[idx] = ValueData::Alias {
                ty: ty,
                original: original,
            };
        } else {
            panic!("Cannot change direct value {} into an alias", dest);
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
        next: PackedOption<Value>, // Next result defined by `def`.
    },

    // Value is an EBB argument.
    Arg {
        ty: Type,
        num: u16, // Argument number, starting from 0.
        ebb: Ebb,
        next: PackedOption<Value>, // Next argument to `ebb`.
    },

    // Value is an alias of another value.
    // An alias value can't be linked as an instruction result or EBB argument. It is used as a
    // placeholder when the original instruction or EBB has been rewritten or modified.
    Alias { ty: Type, original: Value },
}

/// Iterate through a list of related value references, such as:
///
/// - All results defined by an instruction. See `DataFlowGraph::inst_results`.
/// - All arguments to an EBB. See `DataFlowGraph::ebb_args`.
///
/// A value iterator borrows a `DataFlowGraph` reference.
pub struct Values<'a> {
    dfg: &'a DataFlowGraph,
    cur: Option<Value>,
}

impl<'a> Iterator for Values<'a> {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        let rval = self.cur;
        if let Some(prev) = rval {
            // Advance self.cur to the next value, or `None`.
            self.cur = match prev.expand() {
                ExpandedValue::Direct(inst) => self.dfg.insts[inst].second_result(),
                ExpandedValue::Table(index) => {
                    match self.dfg.extended_values[index] {
                        ValueData::Inst { next, .. } => next.into(),
                        ValueData::Arg { next, .. } => next.into(),
                        ValueData::Alias { .. } => {
                            panic!("Alias value {} appeared in value list", prev)
                        }
                    }
                }
            };
        }
        rval
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

    /// Get the instruction reference that will be assigned to the next instruction created by
    /// `make_inst`.
    ///
    /// This is only really useful to the parser.
    pub fn next_inst(&self) -> Inst {
        self.insts.next_key()
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
        let mut total_results = fixed_results;

        // Additional values form a linked list starting from the second result value. Generate
        // the list backwards so we don't have to modify value table entries in place. (This
        // causes additional result values to be numbered backwards which is not the aesthetic
        // choice, but since it is only visible in extremely rare instructions with 3+ results,
        // we don't care).
        let mut head = None;
        let mut first_type = None;
        let mut rev_num = 1;

        // Get the call signature if this is a function call.
        if let Some(sig) = self.call_signature(inst) {
            // Create result values corresponding to the call return types.
            let var_results = self.signatures[sig].return_types.len();
            total_results += var_results;

            for res_idx in (0..var_results).rev() {
                if let Some(ty) = first_type {
                    head = Some(self.make_value(ValueData::Inst {
                        ty: ty,
                        num: (total_results - rev_num) as u16,
                        inst: inst,
                        next: head.into(),
                    }));
                    rev_num += 1;
                }
                first_type = Some(self.signatures[sig].return_types[res_idx].value_type);
            }
        }

        // Then the fixed results which will appear at the front of the list.
        for res_idx in (0..fixed_results).rev() {
            if let Some(ty) = first_type {
                head = Some(self.make_value(ValueData::Inst {
                    ty: ty,
                    num: (total_results - rev_num) as u16,
                    inst: inst,
                    next: head.into(),
                }));
                rev_num += 1;
            }
            first_type = Some(constraints.result_type(res_idx, ctrl_typevar));
        }

        // Update the second_result pointer in `inst`.
        if head.is_some() {
            *self.insts[inst]
                .second_result_mut()
                .expect("instruction format doesn't allow multiple results") = head.into();
        }
        *self.insts[inst].first_type_mut() = first_type.unwrap_or_default();

        total_results
    }

    /// Create an `InsertBuilder` that will insert an instruction at the cursor's current position.
    pub fn ins<'c, 'fc: 'c, 'fd>(&'fd mut self,
                                 at: &'c mut Cursor<'fc>)
                                 -> InsertBuilder<'c, 'fc, 'fd> {
        InsertBuilder::new(self, at)
    }

    /// Create a `ReplaceBuilder` that will replace `inst` with a new instruction in place.
    pub fn replace(&mut self, inst: Inst) -> ReplaceBuilder {
        ReplaceBuilder::new(self, inst)
    }

    /// Detach secondary instruction results, and return them as an iterator.
    ///
    /// If `inst` produces two or more results, detach these secondary result values from `inst`,
    /// and return an iterator that will enumerate them. The first result value cannot be detached.
    ///
    /// Use this method to detach secondary values before using `replace(inst)` to provide an
    /// alternate instruction for computing the primary result value.
    pub fn detach_secondary_results(&mut self, inst: Inst) -> Values {
        let second_result = self[inst].second_result_mut().and_then(|r| r.take());
        Values {
            dfg: self,
            cur: second_result,
        }
    }

    /// Get the first result of an instruction.
    ///
    /// If `Inst` doesn't produce any results, this returns a `Value` with a `VOID` type.
    pub fn first_result(&self, inst: Inst) -> Value {
        Value::new_direct(inst)
    }

    /// Iterate through all the results of an instruction.
    pub fn inst_results(&self, inst: Inst) -> Values {
        Values {
            dfg: self,
            cur: if self.insts[inst].first_type().is_void() {
                None
            } else {
                Some(Value::new_direct(inst))
            },
        }
    }

    /// Get the call signature of a direct or indirect call instruction.
    /// Returns `None` if `inst` is not a call instruction.
    pub fn call_signature(&self, inst: Inst) -> Option<SigRef> {
        match self.insts[inst].analyze_call(&self.value_lists) {
            CallInfo::NotACall => None,
            CallInfo::Direct(f, _) => Some(self.ext_funcs[f].signature),
            CallInfo::Indirect(s, _) => Some(s),
        }
    }

    /// Compute the type of an instruction result from opcode constraints and call signatures.
    ///
    /// This computes the same sequence of result types that `make_inst_results()` above would
    /// assign to the created result values, but it does not depend on `make_inst_results()` being
    /// called first.
    ///
    /// Returns `None` if asked about a result index that is too large.
    pub fn compute_result_type(&self,
                               inst: Inst,
                               result_idx: usize,
                               ctrl_typevar: Type)
                               -> Option<Type> {
        let constraints = self.insts[inst].opcode().constraints();
        let fixed_results = constraints.fixed_results();

        if result_idx < fixed_results {
            return Some(constraints.result_type(result_idx, ctrl_typevar));
        }

        // Not a fixed result, try to extract a return type from the call signature.
        self.call_signature(inst).and_then(|sigref| {
            self.signatures[sigref]
                .return_types
                .get(result_idx - fixed_results)
                .map(|&arg| arg.value_type)
        })
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
        match self.ebbs[ebb].last_arg.expand() {
            None => 0,
            Some(last_arg) => {
                if let ExpandedValue::Table(idx) = last_arg.expand() {
                    if let ValueData::Arg { num, .. } = self.extended_values[idx] {
                        return num as usize + 1;
                    }
                }
                panic!("inconsistent value table entry for EBB argument");
            }
        }
    }

    /// Append an argument with type `ty` to `ebb`.
    pub fn append_ebb_arg(&mut self, ebb: Ebb, ty: Type) -> Value {
        let val = self.make_value(ValueData::Arg {
            ty: ty,
            ebb: ebb,
            num: 0,
            next: None.into(),
        });
        self.put_ebb_arg(ebb, val);
        val
    }

    /// Iterate through the arguments to an EBB.
    pub fn ebb_args(&self, ebb: Ebb) -> Values {
        Values {
            dfg: self,
            cur: self.ebbs[ebb].first_arg.into(),
        }
    }

    /// Given a value that is known to be an EBB argument, return the next EBB argument.
    pub fn next_ebb_arg(&self, arg: Value) -> Option<Value> {
        if let ExpandedValue::Table(index) = arg.expand() {
            if let ValueData::Arg { next, .. } = self.extended_values[index] {
                return next.into();
            }
        }
        panic!("{} is not an EBB argument", arg);
    }

    /// Detach all the arguments from `ebb` and return the first one.
    ///
    /// The whole list of detached arguments can be traversed with `next_ebb_arg`. This method does
    /// not return a `Values` iterator since it is often necessary to mutate the DFG while
    /// processing the list of arguments.
    ///
    /// This is a quite low-level operation. Sensible things to do with the detached EBB arguments
    /// is to put them back on the same EBB with `put_ebb_arg()` or change them into aliases
    /// with `change_to_alias()`.
    pub fn take_ebb_args(&mut self, ebb: Ebb) -> Option<Value> {
        let first = self.ebbs[ebb].first_arg.into();
        self.ebbs[ebb].first_arg = None.into();
        self.ebbs[ebb].last_arg = None.into();
        first
    }

    /// Append an existing argument value to `ebb`.
    ///
    /// The appended value should already be an EBB argument belonging to `ebb`, but it can't be
    /// attached. In practice, this means that it should be one of the values returned from
    /// `take_ebb_args()`.
    ///
    /// In almost all cases, you should be using `append_ebb_arg()` instead of this method.
    pub fn put_ebb_arg(&mut self, ebb: Ebb, arg: Value) {
        let arg_num = match self.ebbs[ebb].last_arg.map(|v| v.expand()) {
            // If last_argument is `None`, we're adding the first EBB argument.
            None => {
                self.ebbs[ebb].first_arg = arg.into();
                0
            }
            // Append to the linked list of arguments.
            Some(ExpandedValue::Table(idx)) => {
                if let ValueData::Arg { ref mut next, num, .. } = self.extended_values[idx] {
                    *next = arg.into();
                    assert!(num < u16::MAX, "Too many arguments to EBB");
                    num + 1
                } else {
                    panic!("inconsistent value table entry for EBB argument");
                }
            }
            _ => panic!("inconsistent value table entry for EBB argument"),
        };
        self.ebbs[ebb].last_arg = arg.into();

        // Now update `arg` itself.
        let arg_ebb = ebb;
        if let ExpandedValue::Table(idx) = arg.expand() {
            if let ValueData::Arg { ref mut num, ebb, ref mut next, .. } =
                self.extended_values[idx] {
                *num = arg_num;
                *next = None.into();
                assert_eq!(arg_ebb, ebb, "{} should already belong to EBB", arg);
                return;
            }
        }
        panic!("{} must be an EBB argument value", arg);
    }
}

// Contents of an extended basic block.
//
// Arguments for an extended basic block are values that dominate everything in the EBB. All
// branches to this EBB must provide matching arguments, and the arguments to the entry EBB must
// match the function arguments.
#[derive(Clone)]
struct EbbData {
    // First argument to this EBB, or `None` if the block has no arguments.
    //
    // The arguments are all `ValueData::Argument` entries that form a linked list from `first_arg`
    // to `last_arg`.
    first_arg: PackedOption<Value>,

    // Last argument to this EBB, or `None` if the block has no arguments.
    last_arg: PackedOption<Value>,
}

impl EbbData {
    fn new() -> EbbData {
        EbbData {
            first_arg: None.into(),
            last_arg: None.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ir::types;
    use ir::{Function, Cursor, Opcode, InstructionData};

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
        assert_eq!(dfg.take_ebb_args(ebb), None);
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

        // Swap the two EBB arguments.
        let take1 = dfg.take_ebb_args(ebb).unwrap();
        assert_eq!(dfg.num_ebb_args(ebb), 0);
        assert_eq!(dfg.ebb_args(ebb).next(), None);
        let take2 = dfg.next_ebb_arg(take1).unwrap();
        assert_eq!(take1, arg1);
        assert_eq!(take2, arg2);
        assert_eq!(dfg.next_ebb_arg(take2), None);
        dfg.put_ebb_arg(ebb, take2);
        let arg3 = dfg.append_ebb_arg(ebb, types::I32);
        dfg.put_ebb_arg(ebb, take1);
        assert_eq!(dfg.ebb_args(ebb).collect::<Vec<_>>(), [take2, arg3, take1]);
    }

    #[test]
    fn aliases() {
        use ir::InstBuilder;
        use ir::entities::ExpandedValue::Direct;
        use ir::condcodes::IntCC;

        let mut func = Function::new();
        let dfg = &mut func.dfg;
        let ebb0 = dfg.make_ebb();
        let pos = &mut Cursor::new(&mut func.layout);
        pos.insert_ebb(ebb0);

        // Build a little test program.
        let v1 = dfg.ins(pos).iconst(types::I32, 42);

        // Make sure we can resolve value aliases even when extended_values is empty.
        assert_eq!(dfg.resolve_aliases(v1), v1);

        let arg0 = dfg.append_ebb_arg(ebb0, types::I32);
        let (s, c) = dfg.ins(pos).iadd_cout(v1, arg0);
        let iadd = match s.expand() {
            Direct(i) => i,
            _ => panic!(),
        };

        // Detach the 'c' value from `iadd`.
        {
            let mut vals = dfg.detach_secondary_results(iadd);
            assert_eq!(vals.next(), Some(c));
            assert_eq!(vals.next(), None);
        }

        // Replace `iadd_cout` with a normal `iadd` and an `icmp`.
        dfg.replace(iadd).iadd(v1, arg0);
        let c2 = dfg.ins(pos).icmp(IntCC::UnsignedLessThan, s, v1);
        dfg.change_to_alias(c, c2);

        assert_eq!(dfg.resolve_aliases(c2), c2);
        assert_eq!(dfg.resolve_aliases(c), c2);

        // Make a copy of the alias.
        let c3 = dfg.ins(pos).copy(c);
        // This does not see through copies.
        assert_eq!(dfg.resolve_aliases(c3), c3);
        // But this goes through both copies and aliases.
        assert_eq!(dfg.resolve_copies(c3), c2);
    }
}
