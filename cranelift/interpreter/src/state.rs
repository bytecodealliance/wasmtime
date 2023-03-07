//! Cranelift instructions modify the state of the machine; the [State] trait describes these
//! ways this can happen.
use crate::address::{Address, AddressSize};
use crate::interpreter::LibCallHandler;
use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::ir::{
    ExternalName, FuncRef, Function, GlobalValue, LibCall, MemFlags, Signature, StackSlot, Type,
    Value,
};
use cranelift_codegen::isa::CallConv;
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
    /// Retrieve the handler callback for a [LibCall](cranelift_codegen::ir::LibCall)
    fn get_libcall_handler(&self) -> LibCallHandler<V>;
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

    /// Computes the stack address for this stack slot, including an offset.
    fn stack_address(
        &self,
        size: AddressSize,
        slot: StackSlot,
        offset: u64,
    ) -> Result<Address, MemoryError>;
    /// Retrieve a value `V` from memory at the given `address`, checking if it belongs either to the
    /// stack or to one of the heaps; the number of bytes loaded corresponds to the specified [Type].
    fn checked_load(
        &self,
        address: Address,
        ty: Type,
        mem_flags: MemFlags,
    ) -> Result<V, MemoryError>;
    /// Store a value `V` into memory at the given `address`, checking if it belongs either to the
    /// stack or to one of the heaps; the number of bytes stored corresponds to the specified [Type].
    fn checked_store(
        &mut self,
        address: Address,
        v: V,
        mem_flags: MemFlags,
    ) -> Result<(), MemoryError>;

    /// Compute the address of a function given its name.
    fn function_address(
        &self,
        size: AddressSize,
        name: &ExternalName,
    ) -> Result<Address, MemoryError>;

    /// Retrieve a reference to a [Function] given its address.
    fn get_function_from_address(&self, address: Address) -> Option<InterpreterFunctionRef<'a>>;

    /// Given a global value, compute the final value for that global value, applying all operations
    /// in intermediate global values.
    fn resolve_global_value(&self, gv: GlobalValue) -> Result<V, MemoryError>;

    /// Checks if an address is valid and within a known region of memory
    fn validate_address(&self, address: &Address) -> Result<(), MemoryError>;

    /// Retrieves the current pinned reg value
    fn get_pinned_reg(&self) -> V;
    /// Sets a value for the pinned reg
    fn set_pinned_reg(&mut self, v: V);
}

pub enum InterpreterFunctionRef<'a> {
    Function(&'a Function),
    LibCall(LibCall),
}

impl<'a> InterpreterFunctionRef<'a> {
    pub fn signature(&self) -> Signature {
        match self {
            InterpreterFunctionRef::Function(f) => f.stencil.signature.clone(),
            // CallConv here is sort of irrelevant, since we don't use it for anything
            InterpreterFunctionRef::LibCall(lc) => lc.signature(CallConv::SystemV),
        }
    }
}

impl<'a> From<&'a Function> for InterpreterFunctionRef<'a> {
    fn from(f: &'a Function) -> Self {
        InterpreterFunctionRef::Function(f)
    }
}

impl From<LibCall> for InterpreterFunctionRef<'_> {
    fn from(lc: LibCall) -> Self {
        InterpreterFunctionRef::LibCall(lc)
    }
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
    #[error("Load of {load_size} bytes is misaligned at address {addr:?}")]
    MisalignedLoad { addr: Address, load_size: usize },
    #[error("Store of {store_size} bytes is misaligned at address {addr:?}")]
    MisalignedStore { addr: Address, store_size: usize },
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

    fn get_libcall_handler(&self) -> LibCallHandler<V> {
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

    fn stack_address(
        &self,
        _size: AddressSize,
        _slot: StackSlot,
        _offset: u64,
    ) -> Result<Address, MemoryError> {
        unimplemented!()
    }

    fn checked_load(
        &self,
        _addr: Address,
        _ty: Type,
        _mem_flags: MemFlags,
    ) -> Result<V, MemoryError> {
        unimplemented!()
    }

    fn checked_store(
        &mut self,
        _addr: Address,
        _v: V,
        _mem_flags: MemFlags,
    ) -> Result<(), MemoryError> {
        unimplemented!()
    }

    fn function_address(
        &self,
        _size: AddressSize,
        _name: &ExternalName,
    ) -> Result<Address, MemoryError> {
        unimplemented!()
    }

    fn get_function_from_address(&self, _address: Address) -> Option<InterpreterFunctionRef<'a>> {
        unimplemented!()
    }

    fn resolve_global_value(&self, _gv: GlobalValue) -> Result<V, MemoryError> {
        unimplemented!()
    }

    fn validate_address(&self, _addr: &Address) -> Result<(), MemoryError> {
        unimplemented!()
    }

    fn get_pinned_reg(&self) -> V {
        unimplemented!()
    }

    fn set_pinned_reg(&mut self, _v: V) {
        unimplemented!()
    }
}
