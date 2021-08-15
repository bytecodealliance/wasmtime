//! Cranelift instructions modify the state of the machine; the [State] trait describes these
//! ways this can happen.
use crate::address::{Address, AddressSize};
use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{FuncRef, Function, StackSlot, Type, Value};
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

    /// Computes the stack address for this stack slot, including an offset.
    fn stack_address(
        &self,
        size: AddressSize,
        slot: StackSlot,
        offset: u64,
    ) -> Result<Address, MemoryError>;
    /// Computes a heap address
    fn heap_address(&self, size: AddressSize, offset: u64) -> Result<Address, MemoryError>;
    /// Retrieve a value `V` from memory at the given `address`, checking if it belongs either to the
    /// stack or to one of the heaps; the number of bytes loaded corresponds to the specified [Type].
    fn checked_load(&self, address: Address, ty: Type) -> Result<V, MemoryError>;
    /// Store a value `V` into memory at the given `address`, checking if it belongs either to the
    /// stack or to one of the heaps; the number of bytes stored corresponds to the specified [Type].
    fn checked_store(&mut self, address: Address, v: V) -> Result<(), MemoryError>;
}

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Invalid DataValue passed as an address: {0}")]
    InvalidAddress(DataValue),
    #[error("Invalid type for address: {0}")]
    InvalidAddressType(Type),
    #[error("Requested an the entry {entry} but only {max} entries are allowed")]
    InvalidEntry { entry: u64, max: u64 },
    #[error("Requested an offset of {offset} but max was {max}")]
    InvalidOffset { offset: u64, max: u64 },
    #[error("Load of {load_size} bytes is larger than available size at address {addr:?}")]
    OutOfBoundsLoad { addr: Address, load_size: usize },
    #[error("Store of {store_size} bytes is larger than available size at address {addr:?}")]
    OutOfBoundsStore { addr: Address, store_size: usize },
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

    fn stack_address(
        &self,
        _size: AddressSize,
        _slot: StackSlot,
        _offset: u64,
    ) -> Result<Address, MemoryError> {
        unimplemented!()
    }

    fn heap_address(&self, _size: AddressSize, _offset: u64) -> Result<Address, MemoryError> {
        unimplemented!()
    }

    fn checked_load(&self, _addr: Address, _ty: Type) -> Result<V, MemoryError> {
        unimplemented!()
    }

    fn checked_store(&mut self, _addr: Address, _v: V) -> Result<(), MemoryError> {
        unimplemented!()
    }
}
