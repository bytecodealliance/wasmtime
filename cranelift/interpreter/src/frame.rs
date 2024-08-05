//! Implements a call frame (activation record) for the Cranelift interpreter.

use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::ir::{types, Function, Value as ValueRef};
use cranelift_entity::EntityRef;
use log::trace;

/// The type used for ensuring [Frame](crate::frame::Frame) entries conform to the expected memory layout.
pub(crate) type Entries = Vec<Option<DataValue>>;

/// Holds the mutable elements of an interpreted function call.
#[derive(Debug)]
pub struct Frame<'a> {
    /// The currently executing function.
    function: &'a Function,
    /// The current mapping of SSA value-references to their actual values. For efficiency, each SSA value is used as an
    /// index into the Vec, meaning some slots may be unused.
    registers: Entries,
}

impl<'a> Frame<'a> {
    /// Construct a new [Frame] for a function. This allocates a slot in the hash map for each SSA `Value` (renamed to
    /// `ValueRef` here) which should mean that no additional allocations are needed while interpreting the frame.
    pub fn new(function: &'a Function) -> Self {
        let num_slots = function.dfg.num_values();
        trace!("Create new frame for function: {}", function.signature);
        Self {
            function,
            registers: vec![None; num_slots],
        }
    }

    /// Retrieve the actual value associated with an SSA reference.
    #[inline]
    pub fn get(&self, name: ValueRef) -> &DataValue {
        assert!(name.index() < self.registers.len());
        trace!("Get {}", name);
        &self
            .registers
            .get(name.index())
            .unwrap_or_else(|| panic!("unknown value: {name}"))
            .as_ref()
            .or_else(|| {
                // We couldn't find the `name` value directly in `registers`, but it is still
                // possible that it is aliased to another value.

                // If we are looking up an undefined value it will have an invalid type, return
                // before trying to resolve it.
                if self.function.dfg.value_type(name) == types::INVALID {
                    return None;
                }

                let alias = self.function.dfg.resolve_aliases(name);
                self.registers
                    .get(alias.index())
                    .unwrap_or_else(|| panic!("unknown value: {alias}"))
                    .as_ref()
            })
            .unwrap_or_else(|| panic!("empty slot: {name}"))
    }

    /// Retrieve multiple SSA references; see `get`.
    pub fn get_all(&self, names: &[ValueRef]) -> Vec<DataValue> {
        names.iter().map(|r| self.get(*r)).cloned().collect()
    }

    /// Assign `value` to the SSA reference `name`.
    #[inline]
    pub fn set(&mut self, name: ValueRef, value: DataValue) -> Option<DataValue> {
        assert!(name.index() < self.registers.len());
        trace!("Set {} -> {}", name, value);
        std::mem::replace(&mut self.registers[name.index()], Some(value))
    }

    /// Assign to multiple SSA references; see `set`.
    pub fn set_all(&mut self, names: &[ValueRef], values: Vec<DataValue>) {
        assert_eq!(names.len(), values.len());
        for (n, v) in names.iter().zip(values) {
            self.set(*n, v);
        }
    }

    /// Rename all of the SSA references in `old_names` to those in `new_names`. This will remove
    /// any old references that are not in `old_names`. TODO This performs an extra allocation that
    /// could be removed if we copied the values in the right order (i.e. when modifying in place,
    /// we need to avoid changing a value before it is referenced).
    pub fn rename(&mut self, old_names: &[ValueRef], new_names: &[ValueRef]) {
        trace!("Renaming {:?} -> {:?}", old_names, new_names);
        assert_eq!(old_names.len(), new_names.len());
        let new_registers = vec![None; self.registers.len()];
        let mut old_registers = std::mem::replace(&mut self.registers, new_registers);
        self.registers = vec![None; self.registers.len()];
        for (&on, &nn) in old_names.iter().zip(new_names) {
            let value = std::mem::replace(&mut old_registers[on.index()], None);
            self.registers[nn.index()] = value;
        }
    }

    /// Accessor for the current entries in the frame.
    pub fn entries_mut(&mut self) -> &mut [Option<DataValue>] {
        &mut self.registers
    }

    /// Accessor for the [`Function`] of this frame.
    pub fn function(&self) -> &'a Function {
        self.function
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::immediates::{Ieee32, Ieee64};
    use cranelift_codegen::ir::InstBuilder;
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
    use cranelift_reader::parse_functions;

    /// Helper to create a function from CLIF IR.
    fn function(code: &str) -> Function {
        parse_functions(code).unwrap().into_iter().next().unwrap()
    }

    /// Build an empty function with a single return.
    fn empty_function() -> Function {
        let mut func = Function::new();
        let mut context = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut func, &mut context);
        let block = builder.create_block();
        builder.switch_to_block(block);
        builder.ins().return_(&[]);
        func
    }

    #[test]
    fn construction() {
        let func = empty_function();
        // Construction should not fail.
        Frame::new(&func);
    }

    #[test]
    fn assignment_and_retrieval() {
        let func = function("function %test(i32) -> i32 { block0(v0:i32): return v0 }");
        let mut frame = Frame::new(&func);
        let ssa_value_ref = ValueRef::from_u32(0);
        let fortytwo = DataValue::I32(42);

        // Verify that setting a valid SSA ref will make the value retrievable.
        frame.set(ssa_value_ref, fortytwo.clone());
        assert_eq!(frame.get(ssa_value_ref), &fortytwo);
    }

    #[test]
    fn assignment_to_extra_slots() {
        let func = function("function %test(i32) -> i32 { block0(v10:i32): return v10 }");
        let mut frame = Frame::new(&func);
        let ssa_value_ref = ValueRef::from_u32(5);
        let fortytwo = DataValue::I32(42);

        // Due to how Cranelift organizes its SSA values, the use of v10 defines 11 slots for values
        // to fit in--the following should work.
        frame.set(ssa_value_ref, fortytwo.clone());
        assert_eq!(frame.get(ssa_value_ref), &fortytwo);
    }

    #[test]
    #[should_panic(expected = "assertion failed: name.index() < self.registers.len()")]
    fn invalid_assignment() {
        let func = function("function %test(i32) -> i32 { block0(v10:i32): return v10 }");
        let mut frame = Frame::new(&func);
        let fortytwo = DataValue::I32(42);

        // Since the SSA value ref points to 42 and the function only has 11 slots, this should
        // fail. TODO currently this is a panic under the assumption we will not set indexes outside
        // of the valid SSA value range but it might be better as a result.
        frame.set(ValueRef::from_u32(11), fortytwo.clone());
    }

    #[test]
    #[should_panic(expected = "assertion failed: name.index() < self.registers.len()")]
    fn retrieve_nonexistent_value() {
        let func = empty_function();
        let frame = Frame::new(&func);
        let ssa_value_ref = ValueRef::from_u32(1);

        // Retrieving a non-existent value should return an error.
        frame.get(ssa_value_ref);
    }

    #[test]
    #[should_panic(expected = "empty slot: v5")]
    fn retrieve_and_assign_multiple_values() {
        let func = function("function %test(i32) -> i32 { block0(v10:i32): return v10 }");
        let mut frame = Frame::new(&func);
        let ssa_value_refs = [
            ValueRef::from_u32(2),
            ValueRef::from_u32(4),
            ValueRef::from_u32(6),
        ];
        let values = vec![
            DataValue::I8(1),
            DataValue::I8(42),
            DataValue::F32(Ieee32::from(0.42)),
        ];

        // We can assign and retrieve multiple (cloned) values.
        frame.set_all(&ssa_value_refs, values.clone());
        let retrieved_values = frame.get_all(&ssa_value_refs);
        assert_eq!(values, retrieved_values);

        // But if we attempt to retrieve an invalid value we should get an error:
        frame.get_all(&[ValueRef::from_u32(2), ValueRef::from_u32(5)]);
    }

    #[test]
    #[should_panic(expected = "empty slot: v10")]
    fn rename() {
        let func = function("function %test(i32) -> i32 { block0(v10:i32): return v10 }");
        let mut frame = Frame::new(&func);
        let old_ssa_value_refs = [ValueRef::from_u32(9), ValueRef::from_u32(10)];
        let values = vec![DataValue::I8(1), DataValue::F64(Ieee64::from(0.0))];
        frame.set_all(&old_ssa_value_refs, values.clone());

        // Rename the old SSA values to the new values.
        let new_ssa_value_refs = [ValueRef::from_u32(4), ValueRef::from_u32(2)];
        frame.rename(&old_ssa_value_refs, &new_ssa_value_refs);

        // Now we should be able to retrieve new values and the old ones should fail.
        assert_eq!(frame.get_all(&new_ssa_value_refs), values);
        frame.get(ValueRef::from_u32(10));
    }

    #[test]
    #[should_panic(expected = "empty slot: v2")]
    fn rename_duplicates_causes_inconsistency() {
        let func = function("function %test(i32) -> i32 { block0(v10:i32): return v10 }");
        let mut frame = Frame::new(&func);
        let old_ssa_value_refs = [ValueRef::from_u32(1), ValueRef::from_u32(9)];
        let values = vec![DataValue::I8(1), DataValue::F64(Ieee64::from(f64::NAN))];
        frame.set_all(&old_ssa_value_refs, values.clone());

        // Rename the old SSA values to the new values.
        let old_duplicated_ssa_value_refs = [ValueRef::from_u32(1), ValueRef::from_u32(1)];
        let new_ssa_value_refs = [ValueRef::from_u32(4), ValueRef::from_u32(2)];
        frame.rename(&old_duplicated_ssa_value_refs, &new_ssa_value_refs);

        // If we use duplicates then subsequent renamings (v1 -> v2) will be empty.
        frame.get(ValueRef::from_u32(2));
    }
}
