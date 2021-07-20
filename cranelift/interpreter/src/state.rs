//! Cranelift instructions modify the state of the machine; the [State] trait describes these
//! ways this can happen.
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{FuncRef, Function, Type, Value};
use cranelift_entity::PrimaryMap;
use smallvec::SmallVec;
use thiserror::Error;

/// This trait manages the state necessary to interpret a single Cranelift instruction--it describes
/// all of the ways a Cranelift interpreter can interact with its virtual state. This makes it
/// possible to use the [Interpreter](crate::interpreter::Interpreter) in a range of situations:
/// - when interpretation requires understanding all of the ways state can change (e.g. loading and
/// storing from the heap) we will use a full-fledged state, like
/// [InterpreterState](crate::interpreter::InterpreterState).
/// - when interpretation can ignore some state changes (e.g. abstract interpretation of arithmetic
/// instructions--no heap knowledge required), we can partially implement this trait. See
/// [ImmutableRegisterState] for an example of this: it only exposes the values referenced by the
/// SSA references in the current frame and not much else.
pub trait State<'a, V> {
    /// Retrieve a reference to a [Function].
    fn get_function(&self, func_ref: FuncRef) -> Option<&'a Function>;
    /// Retrieve a reference to the currently executing [Function].
    fn get_current_function(&self) -> &'a Function;
    /// Record that an interpreter has called into a new [Function].
    fn push_frame(&mut self, function: &'a Function);
    /// Record that an interpreter has returned from a called [Function].
    fn pop_frame(&mut self);

    /// Retrieve a value `V` by its [value reference](cranelift_codegen::ir::Value) from the
    /// virtual register file.
    fn get_value(&self, name: Value) -> Option<V>;
    /// Assign a value `V` to its [value reference](cranelift_codegen::ir::Value) in the
    /// virtual register file.
    fn set_value(&mut self, name: Value, value: V) -> Option<V>;
    /// Collect a list of values `V` by their  [value references](cranelift_codegen::ir::Value);
    /// this is a convenience method for `get_value`. If no value is found for a value reference,
    /// return an `Err` containing the offending reference.
    fn collect_values(&self, names: &[Value]) -> Result<SmallVec<[V; 1]>, Value> {
        let mut values = SmallVec::with_capacity(names.len());
        for &n in names {
            match self.get_value(n) {
                None => return Err(n),
                Some(v) => values.push(v),
            }
        }
        Ok(values)
    }

    /// Check if an [IntCC] flag has been set.
    fn has_iflag(&self, flag: IntCC) -> bool;
    /// Set an [IntCC] flag.
    fn set_iflag(&mut self, flag: IntCC);
    /// Check if a [FloatCC] flag has been set.
    fn has_fflag(&self, flag: FloatCC) -> bool;
    /// Set a [FloatCC] flag.
    fn set_fflag(&mut self, flag: FloatCC);
    /// Clear all [IntCC] and [FloatCC] flags.
    fn clear_flags(&mut self);

    /// Retrieve a value `V` from the heap at the given `offset`; the number of bytes loaded
    /// corresponds to the specified [Type].
    fn load_heap(&self, offset: usize, ty: Type) -> Result<V, MemoryError>;
    /// Store a value `V` into the heap at the given `offset`. The [Type] of `V` will determine
    /// the number of bytes stored.
    fn store_heap(&mut self, offset: usize, v: V) -> Result<(), MemoryError>;

    /// Retrieve a value `V` from the stack at the given `offset`; the number of bytes loaded
    /// corresponds to the specified [Type].
    fn load_stack(&self, offset: usize, ty: Type) -> Result<V, MemoryError>;
    /// Store a value `V` on the stack at the given `offset`. The [Type] of `V` will determine
    /// the number of bytes stored.
    fn store_stack(&mut self, offset: usize, v: V) -> Result<(), MemoryError>;
}

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("insufficient memory: asked for address {0} in memory of size {1}")]
    InsufficientMemory(usize, usize),
}

/// This dummy state allows interpretation over an immutable mapping of values in a single frame.
pub struct ImmutableRegisterState<'a, V>(&'a PrimaryMap<Value, V>);
impl<'a, V> ImmutableRegisterState<'a, V> {
    pub fn new(values: &'a PrimaryMap<Value, V>) -> Self {
        Self(values)
    }
}

impl<'a, V> State<'a, V> for ImmutableRegisterState<'a, V>
where
    V: Clone,
{
    fn get_function(&self, _func_ref: FuncRef) -> Option<&'a Function> {
        None
    }

    fn get_current_function(&self) -> &'a Function {
        unimplemented!()
    }

    fn push_frame(&mut self, _function: &'a Function) {
        unimplemented!()
    }

    fn pop_frame(&mut self) {
        unimplemented!()
    }

    fn get_value(&self, name: Value) -> Option<V> {
        self.0.get(name).cloned()
    }

    fn set_value(&mut self, _name: Value, _value: V) -> Option<V> {
        None
    }

    fn has_iflag(&self, _flag: IntCC) -> bool {
        false
    }

    fn has_fflag(&self, _flag: FloatCC) -> bool {
        false
    }

    fn set_iflag(&mut self, _flag: IntCC) {}

    fn set_fflag(&mut self, _flag: FloatCC) {}

    fn clear_flags(&mut self) {}

    fn load_heap(&self, _offset: usize, _ty: Type) -> Result<V, MemoryError> {
        unimplemented!()
    }

    fn store_heap(&mut self, _offset: usize, _v: V) -> Result<(), MemoryError> {
        unimplemented!()
    }

    fn load_stack(&self, _offset: usize, _ty: Type) -> Result<V, MemoryError> {
        unimplemented!()
    }

    fn store_stack(&mut self, _offset: usize, _v: V) -> Result<(), MemoryError> {
        unimplemented!()
    }
}
