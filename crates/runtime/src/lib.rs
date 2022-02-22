//! Runtime library support for Wasmtime.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(
    feature = "cargo-clippy",
    allow(clippy::new_without_default, clippy::new_without_default)
)]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        clippy::float_arithmetic,
        clippy::mut_mut,
        clippy::nonminimal_bool,
        clippy::map_unwrap_or,
        clippy::clippy::print_stdout,
        clippy::unicode_not_nfc,
        clippy::use_self
    )
)]
#![cfg_attr(not(memfd), allow(unused_variables, unreachable_code))]

use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use anyhow::Error;
use wasmtime_environ::DefinedFuncIndex;
use wasmtime_environ::DefinedMemoryIndex;
use wasmtime_environ::FunctionInfo;
use wasmtime_environ::SignatureIndex;

mod export;
mod externref;
mod imports;
mod instance;
mod memory;
mod mmap;
mod mmap_vec;
mod table;
mod traphandlers;
mod vmcontext;

pub mod debug_builtins;
pub mod libcalls;

pub use wasmtime_jit_debug::gdb_jit_int::GdbJitImageRegistration;

pub use crate::export::*;
pub use crate::externref::*;
pub use crate::imports::Imports;
pub use crate::instance::{
    InstanceAllocationRequest, InstanceAllocator, InstanceHandle, InstantiationError, LinkError,
    OnDemandInstanceAllocator, StorePtr,
};
#[cfg(feature = "pooling-allocator")]
pub use crate::instance::{
    InstanceLimits, ModuleLimits, PoolingAllocationStrategy, PoolingInstanceAllocator,
};
pub use crate::memory::{DefaultMemoryCreator, Memory, RuntimeLinearMemory, RuntimeMemoryCreator};
pub use crate::mmap::Mmap;
pub use crate::mmap_vec::MmapVec;
pub use crate::table::{Table, TableElement};
pub use crate::traphandlers::{
    catch_traps, init_traps, raise_lib_trap, raise_user_trap, resume_panic, tls_eager_initialize,
    SignalHandler, TlsRestore, Trap,
};
pub use crate::vmcontext::{
    VMCallerCheckedAnyfunc, VMContext, VMFunctionBody, VMFunctionImport, VMGlobalDefinition,
    VMGlobalImport, VMInterrupts, VMInvokeArgument, VMMemoryDefinition, VMMemoryImport,
    VMSharedSignatureIndex, VMTableDefinition, VMTableImport, VMTrampoline, ValRaw,
};

mod module_id;
pub use module_id::{CompiledModuleId, CompiledModuleIdAllocator};

#[cfg(memfd)]
mod memfd;
#[cfg(memfd)]
pub use crate::memfd::{MemFdSlot, MemoryMemFd, ModuleMemFds};

#[cfg(not(memfd))]
mod memfd_disabled;
#[cfg(not(memfd))]
pub use crate::memfd_disabled::{MemFdSlot, MemoryMemFd, ModuleMemFds};

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Dynamic runtime functionality needed by this crate throughout the execution
/// of a wasm instance.
///
/// This trait is used to store a raw pointer trait object within each
/// `VMContext`. This raw pointer trait object points back to the
/// `wasmtime::Store` internally but is type-erased so this `wasmtime_runtime`
/// crate doesn't need the entire `wasmtime` crate to build.
///
/// Note that this is an extra-unsafe trait because no heed is paid to the
/// lifetime of this store or the Send/Sync-ness of this store. All of that must
/// be respected by embedders (e.g. the `wasmtime::Store` structure). The theory
/// is that `wasmtime::Store` handles all this correctly.
pub unsafe trait Store {
    /// Returns the raw pointer in memory where this store's shared
    /// `VMInterrupts` structure is located.
    ///
    /// Used to configure `VMContext` initialization and store the right pointer
    /// in the `VMContext`.
    fn vminterrupts(&self) -> *mut VMInterrupts;

    /// Returns a pointer to the global epoch counter.
    ///
    /// Used to configure the `VMContext` on initialization.
    fn epoch_ptr(&self) -> *const AtomicU64;

    /// Returns the externref management structures necessary for this store.
    ///
    /// The first element returned is the table in which externrefs are stored
    /// throughout wasm execution, and the second element is how to look up
    /// module information for gc requests.
    fn externref_activations_table(
        &mut self,
    ) -> (&mut VMExternRefActivationsTable, &dyn ModuleInfoLookup);

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
    fn memory_grow_failed(&mut self, error: &Error);
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
    fn table_grow_failed(&mut self, error: &Error);
    /// Callback invoked whenever fuel runs out by a wasm instance. If an error
    /// is returned that's raised as a trap. Otherwise wasm execution will
    /// continue as normal.
    fn out_of_gas(&mut self) -> Result<(), Error>;
    /// Callback invoked whenever an instance observes a new epoch
    /// number. Cannot fail; cooperative epoch-based yielding is
    /// completely semantically transparent. Returns the new deadline.
    fn new_epoch(&mut self) -> Result<u64, Error>;
}

/// Functionality required by this crate for a particular module. This
/// is chiefly needed for lazy initialization of various bits of
/// instance state.
///
/// When an instance is created, it holds an Arc<dyn ModuleRuntimeInfo>
/// so that it can get to signatures, metadata on functions, memfd and
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

    /// The signatures.
    fn signature(&self, index: SignatureIndex) -> VMSharedSignatureIndex;

    /// The base address of where JIT functions are located.
    fn image_base(&self) -> usize;

    /// Descriptors about each compiled function, such as the offset from
    /// `image_base`.
    fn function_info(&self, func_index: DefinedFuncIndex) -> &FunctionInfo;

    /// memfd images, if any, for this module.
    fn memfd_image(&self, memory: DefinedMemoryIndex) -> anyhow::Result<Option<&Arc<MemoryMemFd>>>;

    /// A unique ID for this particular module. This can be used to
    /// allow for fastpaths to optimize a "re-instantiate the same
    /// module again" case.
    fn unique_id(&self) -> Option<CompiledModuleId>;

    /// A slice pointing to all data that is referenced by this instance.
    fn wasm_data(&self) -> &[u8];

    /// Returns an array, indexed by `SignatureIndex` of all
    /// `VMSharedSignatureIndex` entries corresponding to the `SignatureIndex`.
    fn signature_ids(&self) -> &[VMSharedSignatureIndex];
}
