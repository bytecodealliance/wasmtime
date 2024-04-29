//! Runtime library support for Wasmtime.

#![deny(missing_docs)]
#![warn(clippy::cast_sign_loss)]

use anyhow::{Error, Result};
use std::fmt;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use wasmtime_environ::{
    DefinedFuncIndex, DefinedMemoryIndex, HostPtr, ModuleInternedTypeIndex, VMOffsets,
    VMSharedTypeIndex,
};

mod arch;
mod async_yield;
#[cfg(feature = "component-model")]
pub mod component;
mod const_expr;
mod export;
mod gc;
mod imports;
mod instance;
mod memory;
mod mmap;
mod mmap_vec;
mod send_sync_ptr;
mod store_box;
mod sys;
mod table;
mod traphandlers;
mod vmcontext;

mod threads;
pub use self::threads::*;

#[cfg(feature = "debug-builtins")]
pub mod debug_builtins;
pub mod libcalls;
pub mod mpk;

#[cfg(feature = "debug-builtins")]
pub use wasmtime_jit_debug::gdb_jit_int::GdbJitImageRegistration;

pub use crate::arch::{get_stack_pointer, V128Abi};
pub use crate::async_yield::*;
pub use crate::export::*;
pub use crate::gc::*;
pub use crate::imports::Imports;
pub use crate::instance::{
    GcHeapAllocationIndex, Instance, InstanceAllocationRequest, InstanceAllocator,
    InstanceAllocatorImpl, InstanceHandle, MemoryAllocationIndex, OnDemandInstanceAllocator,
    StorePtr, TableAllocationIndex,
};
#[cfg(feature = "pooling-allocator")]
pub use crate::instance::{
    InstanceLimits, PoolingInstanceAllocator, PoolingInstanceAllocatorConfig,
};
pub use crate::memory::{DefaultMemoryCreator, Memory, RuntimeLinearMemory, RuntimeMemoryCreator};
pub use crate::mmap::Mmap;
pub use crate::mmap_vec::MmapVec;
pub use crate::mpk::MpkEnabled;
pub use crate::store_box::*;
pub use crate::sys::unwind::UnwindRegistration;
pub use crate::table::{Table, TableElement};
pub use crate::traphandlers::*;
pub use crate::vmcontext::{
    VMArrayCallFunction, VMArrayCallHostFuncContext, VMContext, VMFuncRef, VMFunctionBody,
    VMFunctionImport, VMGlobalDefinition, VMGlobalImport, VMInvokeArgument, VMMemoryDefinition,
    VMMemoryImport, VMNativeCallFunction, VMNativeCallHostFuncContext, VMOpaqueContext,
    VMRuntimeLimits, VMTableDefinition, VMTableImport, VMWasmCallFunction, ValRaw,
};
pub use send_sync_ptr::SendSyncPtr;

mod module_id;
pub use module_id::{CompiledModuleId, CompiledModuleIdAllocator};

mod cow;
pub use crate::cow::{MemoryImage, MemoryImageSlot, ModuleMemoryImages};

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Dynamic runtime functionality needed by this crate throughout the execution
/// of a wasm instance.
///
/// This trait is used to store a raw pointer trait object within each
/// `VMContext`. This raw pointer trait object points back to the
/// `wasmtime::Store` internally but is type-erased so this `wasmtime-runtime`
/// crate doesn't need the entire `wasmtime` crate to build.
///
/// Note that this is an extra-unsafe trait because no heed is paid to the
/// lifetime of this store or the Send/Sync-ness of this store. All of that must
/// be respected by embedders (e.g. the `wasmtime::Store` structure). The theory
/// is that `wasmtime::Store` handles all this correctly.
pub unsafe trait Store {
    /// Returns the raw pointer in memory where this store's shared
    /// `VMRuntimeLimits` structure is located.
    ///
    /// Used to configure `VMContext` initialization and store the right pointer
    /// in the `VMContext`.
    fn vmruntime_limits(&self) -> *mut VMRuntimeLimits;

    /// Returns a pointer to the global epoch counter.
    ///
    /// Used to configure the `VMContext` on initialization.
    fn epoch_ptr(&self) -> *const AtomicU64;

    /// Get this store's GC heap.
    fn gc_store(&mut self) -> &mut GcStore {
        self.maybe_gc_store()
            .expect("attempt to access the GC store before it has been allocated")
    }

    /// Get this store's GC heap, if it has been allocated.
    fn maybe_gc_store(&mut self) -> Option<&mut GcStore>;

    /// Callback invoked to allow the store's resource limiter to reject a
    /// memory grow operation.
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> Result<bool, Error>;

    /// Callback invoked to notify the store's resource limiter that a memory
    /// grow operation has failed.
    ///
    /// Note that this is not invoked if `memory_growing` returns an error.
    fn memory_grow_failed(&mut self, error: Error) -> Result<()>;

    /// Callback invoked to allow the store's resource limiter to reject a
    /// table grow operation.
    fn table_growing(
        &mut self,
        current: u32,
        desired: u32,
        maximum: Option<u32>,
    ) -> Result<bool, Error>;

    /// Callback invoked to notify the store's resource limiter that a table
    /// grow operation has failed.
    ///
    /// Note that this is not invoked if `table_growing` returns an error.
    fn table_grow_failed(&mut self, error: Error) -> Result<()>;

    /// Callback invoked whenever fuel runs out by a wasm instance. If an error
    /// is returned that's raised as a trap. Otherwise wasm execution will
    /// continue as normal.
    fn out_of_gas(&mut self) -> Result<(), Error>;

    /// Callback invoked whenever an instance observes a new epoch
    /// number. Cannot fail; cooperative epoch-based yielding is
    /// completely semantically transparent. Returns the new deadline.
    fn new_epoch(&mut self) -> Result<u64, Error>;

    /// Callback invoked whenever an instance needs to trigger a GC.
    ///
    /// Optionally given a GC reference that is rooted for the collection, and
    /// then whose updated GC reference is returned.
    ///
    /// Cooperative, async-yielding (if configured) is completely transparent.
    ///
    /// If the async GC was cancelled, returns an error. This should be raised
    /// as a trap to clean up Wasm execution.
    fn gc(&mut self, root: Option<VMGcRef>) -> Result<Option<VMGcRef>>;

    /// Metadata required for resources for the component model.
    #[cfg(feature = "component-model")]
    fn component_calls(&mut self) -> &mut component::CallContexts;
}

/// Functionality required by this crate for a particular module. This
/// is chiefly needed for lazy initialization of various bits of
/// instance state.
///
/// When an instance is created, it holds an `Arc<dyn ModuleRuntimeInfo>`
/// so that it can get to signatures, metadata on functions, memory and
/// funcref-table images, etc. All of these things are ordinarily known
/// by the higher-level layers of Wasmtime. Specifically, the main
/// implementation of this trait is provided by
/// `wasmtime::module::ModuleInner`.  Since the runtime crate sits at
/// the bottom of the dependence DAG though, we don't know or care about
/// that; we just need some implementor of this trait for each
/// allocation request.
pub trait ModuleRuntimeInfo: Send + Sync + 'static {
    /// The underlying Module.
    fn module(&self) -> &Arc<wasmtime_environ::Module>;

    /// Translate a module-level interned type index into an engine-level
    /// interned type index.
    fn engine_type_index(&self, module_index: ModuleInternedTypeIndex) -> VMSharedTypeIndex;

    /// Returns the address, in memory, that the function `index` resides at.
    fn function(&self, index: DefinedFuncIndex) -> NonNull<VMWasmCallFunction>;

    /// Returns the address, in memory, of the trampoline that allows the given
    /// defined Wasm function to be called by the native calling convention.
    ///
    /// Returns `None` for Wasm functions which do not escape, and therefore are
    /// not callable from outside the Wasm module itself.
    fn native_to_wasm_trampoline(
        &self,
        index: DefinedFuncIndex,
    ) -> Option<NonNull<VMNativeCallFunction>>;

    /// Returns the address, in memory, of the trampoline that allows the given
    /// defined Wasm function to be called by the array calling convention.
    ///
    /// Returns `None` for Wasm functions which do not escape, and therefore are
    /// not callable from outside the Wasm module itself.
    fn array_to_wasm_trampoline(&self, index: DefinedFuncIndex) -> Option<VMArrayCallFunction>;

    /// Return the address, in memory, of the trampoline that allows Wasm to
    /// call a native function of the given signature.
    fn wasm_to_native_trampoline(
        &self,
        signature: VMSharedTypeIndex,
    ) -> Option<NonNull<VMWasmCallFunction>>;

    /// Returns the `MemoryImage` structure used for copy-on-write
    /// initialization of the memory, if it's applicable.
    fn memory_image(&self, memory: DefinedMemoryIndex)
        -> anyhow::Result<Option<&Arc<MemoryImage>>>;

    /// A unique ID for this particular module. This can be used to
    /// allow for fastpaths to optimize a "re-instantiate the same
    /// module again" case.
    fn unique_id(&self) -> Option<CompiledModuleId>;

    /// A slice pointing to all data that is referenced by this instance.
    fn wasm_data(&self) -> &[u8];

    /// Returns an array, indexed by `ModuleInternedTypeIndex` of all
    /// `VMSharedSignatureIndex` entries corresponding to the `SignatureIndex`.
    fn type_ids(&self) -> &[VMSharedTypeIndex];

    /// Offset information for the current host.
    fn offsets(&self) -> &VMOffsets<HostPtr>;
}

/// Returns the host OS page size, in bytes.
pub fn page_size() -> usize {
    static PAGE_SIZE: AtomicUsize = AtomicUsize::new(0);

    return match PAGE_SIZE.load(Ordering::Relaxed) {
        0 => {
            let size = sys::vm::get_page_size();
            assert!(size != 0);
            PAGE_SIZE.store(size, Ordering::Relaxed);
            size
        }
        n => n,
    };
}

/// Result of [`Memory::atomic_wait32`] and [`Memory::atomic_wait64`]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum WaitResult {
    /// Indicates that a `wait` completed by being awoken by a different thread.
    /// This means the thread went to sleep and didn't time out.
    Ok = 0,
    /// Indicates that `wait` did not complete and instead returned due to the
    /// value in memory not matching the expected value.
    Mismatch = 1,
    /// Indicates that `wait` completed with a timeout, meaning that the
    /// original value matched as expected but nothing ever called `notify`.
    TimedOut = 2,
}

/// Description about a fault that occurred in WebAssembly.
#[derive(Debug)]
pub struct WasmFault {
    /// The size of memory, in bytes, at the time of the fault.
    pub memory_size: usize,
    /// The WebAssembly address at which the fault occurred.
    pub wasm_address: u64,
}

impl fmt::Display for WasmFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "memory fault at wasm address 0x{:x} in linear memory of size 0x{:x}",
            self.wasm_address, self.memory_size,
        )
    }
}
