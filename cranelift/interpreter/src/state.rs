//! Cranelift instructions modify the state of the machine; the [State] trait describes these
//! ways this can happen.
use crate::address::{Address, AddressSize};
use crate::frame::Frame;
use crate::interpreter::LibCallHandler;
use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::ir::{
    types, ExternalName, FuncRef, Function, GlobalValue, LibCall, MemFlags, Signature, StackSlot,
    Type, Value,
};
use cranelift_codegen::isa::CallConv;
use smallvec::SmallVec;
use thiserror::Error;

/// This trait manages the state necessary to interpret a single Cranelift instruction--it describes
/// all of the ways a Cranelift interpreter can interact with its virtual state. This makes it
/// possible to use the [Interpreter](crate::interpreter::Interpreter) in a range of situations:
/// - when interpretation needs to happen in a way isolated from the host a state which keeps a
///   stack and bound checks memory accesses can be used, like
///   [InterpreterState](crate::interpreter::InterpreterState).
/// - when interpretation needs to have access to the host a state which allows direct access to the
///   host memory and native functions can be used.
pub trait State<'a> {
    /// Retrieve a reference to a [Function].
    fn get_function(&self, func_ref: FuncRef) -> Option<&'a Function>;
    /// Retrieve a reference to the currently executing [Function].
    fn get_current_function(&self) -> &'a Function;
    /// Retrieve the handler callback for a [LibCall]
    fn get_libcall_handler(&self) -> LibCallHandler;

    /// Record that an interpreter has called into a new [Function].
    fn push_frame(&mut self, function: &'a Function);
    /// Record that an interpreter has returned from a called [Function].
    fn pop_frame(&mut self);

    fn current_frame_mut(&mut self) -> &mut Frame<'a>;
    fn current_frame(&self) -> &Frame<'a>;

    /// Collect a list of values `V` by their [value references](cranelift_codegen::ir::Value).
    fn collect_values(&self, names: &[Value]) -> SmallVec<[DataValue; 1]> {
        let frame = self.current_frame();
        names.into_iter().map(|n| frame.get(*n).clone()).collect()
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
    ) -> Result<DataValue, MemoryError>;
    /// Store a value `V` into memory at the given `address`, checking if it belongs either to the
    /// stack or to one of the heaps; the number of bytes stored corresponds to the specified [Type].
    fn checked_store(
        &mut self,
        address: Address,
        v: DataValue,
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
    fn resolve_global_value(&self, gv: GlobalValue) -> Result<DataValue, MemoryError>;

    /// Retrieves the current pinned reg value
    fn get_pinned_reg(&self) -> DataValue;
    /// Sets a value for the pinned reg
    fn set_pinned_reg(&mut self, v: DataValue);
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
            // FIXME handle non-64bit systems
            InterpreterFunctionRef::LibCall(lc) => lc.signature(CallConv::SystemV, types::I64),
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
    OutOfBoundsLoad {
        addr: Address,
        load_size: usize,
        mem_flags: MemFlags,
    },
    #[error("Store of {store_size} bytes is larger than available size at address {addr:?}")]
    OutOfBoundsStore {
        addr: Address,
        store_size: usize,
        mem_flags: MemFlags,
    },
    #[error("Load of {load_size} bytes is misaligned at address {addr:?}")]
    MisalignedLoad { addr: Address, load_size: usize },
    #[error("Store of {store_size} bytes is misaligned at address {addr:?}")]
    MisalignedStore { addr: Address, store_size: usize },
}
