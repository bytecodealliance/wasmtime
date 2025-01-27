//! Runtime library support for Wasmtime.

#![deny(missing_docs)]
// See documentation in crates/wasmtime/src/runtime.rs for why this is
// selectively enabled here.
#![warn(clippy::cast_sign_loss)]

use crate::prelude::*;
use crate::store::StoreOpaque;
use alloc::sync::Arc;
use core::fmt;
use core::ops::Deref;
use core::ops::DerefMut;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};
use wasmtime_environ::{
    DefinedFuncIndex, DefinedMemoryIndex, HostPtr, ModuleInternedTypeIndex, VMOffsets,
    VMSharedTypeIndex,
};

#[cfg(has_host_compiler_backend)]
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
mod mmap_vec;
mod provenance;
mod send_sync_ptr;
mod send_sync_unsafe_cell;
mod store_box;
mod sys;
mod table;
mod traphandlers;
mod unwind;
mod vmcontext;

#[cfg(feature = "threads")]
mod parking_spot;

// Note that `debug_builtins` here is disabled with a feature or a lack of a
// native compilation backend because it's only here to assist in debugging
// natively compiled code.
#[cfg(all(has_host_compiler_backend, feature = "debug-builtins"))]
pub mod debug_builtins;
pub mod libcalls;
pub mod mpk;

#[cfg(feature = "pulley")]
pub(crate) mod interpreter;
#[cfg(not(feature = "pulley"))]
pub(crate) mod interpreter_disabled;
#[cfg(not(feature = "pulley"))]
pub(crate) use interpreter_disabled as interpreter;

#[cfg(feature = "debug-builtins")]
pub use wasmtime_jit_debug::gdb_jit_int::GdbJitImageRegistration;

#[cfg(has_host_compiler_backend)]
pub use crate::runtime::vm::arch::get_stack_pointer;
pub use crate::runtime::vm::async_yield::*;
pub use crate::runtime::vm::export::*;
pub use crate::runtime::vm::gc::*;
pub use crate::runtime::vm::imports::Imports;
pub use crate::runtime::vm::instance::{
    GcHeapAllocationIndex, Instance, InstanceAllocationRequest, InstanceAllocator,
    InstanceAllocatorImpl, InstanceAndStore, InstanceHandle, MemoryAllocationIndex,
    OnDemandInstanceAllocator, StorePtr, TableAllocationIndex,
};
#[cfg(feature = "pooling-allocator")]
pub use crate::runtime::vm::instance::{
    InstanceLimits, PoolConcurrencyLimitError, PoolingInstanceAllocator,
    PoolingInstanceAllocatorConfig,
};
pub use crate::runtime::vm::interpreter::*;
pub use crate::runtime::vm::memory::{
    Memory, MemoryBase, RuntimeLinearMemory, RuntimeMemoryCreator, SharedMemory,
};
pub use crate::runtime::vm::mmap_vec::MmapVec;
pub use crate::runtime::vm::mpk::MpkEnabled;
pub use crate::runtime::vm::provenance::*;
pub use crate::runtime::vm::store_box::*;
#[cfg(feature = "std")]
pub use crate::runtime::vm::sys::mmap::open_file_for_mmap;
#[cfg(has_host_compiler_backend)]
pub use crate::runtime::vm::sys::unwind::UnwindRegistration;
pub use crate::runtime::vm::table::{Table, TableElement};
pub use crate::runtime::vm::traphandlers::*;
pub use crate::runtime::vm::unwind::*;
pub use crate::runtime::vm::vmcontext::{
    VMArrayCallFunction, VMArrayCallHostFuncContext, VMContext, VMFuncRef, VMFunctionBody,
    VMFunctionImport, VMGlobalDefinition, VMGlobalImport, VMMemoryDefinition, VMMemoryImport,
    VMOpaqueContext, VMRuntimeLimits, VMTableImport, VMWasmCallFunction, ValRaw,
};
pub use send_sync_ptr::SendSyncPtr;
pub use send_sync_unsafe_cell::SendSyncUnsafeCell;

mod module_id;
pub use module_id::CompiledModuleId;

#[cfg(has_virtual_memory)]
mod byte_count;
#[cfg(has_virtual_memory)]
mod cow;
#[cfg(not(has_virtual_memory))]
mod cow_disabled;
#[cfg(has_virtual_memory)]
mod mmap;

cfg_if::cfg_if! {
    if #[cfg(has_virtual_memory)] {
        pub use crate::runtime::vm::byte_count::*;
        pub use crate::runtime::vm::mmap::{Mmap, MmapOffset};
        pub use self::cow::{MemoryImage, MemoryImageSlot, ModuleMemoryImages};
    } else {
        pub use self::cow_disabled::{MemoryImage, MemoryImageSlot, ModuleMemoryImages};
    }
}

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
pub unsafe trait VMStore {
    /// Get a shared borrow of this store's `StoreOpaque`.
    fn store_opaque(&self) -> &StoreOpaque;

    /// Get an exclusive borrow of this store's `StoreOpaque`.
    fn store_opaque_mut(&mut self) -> &mut StoreOpaque;

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
        current: usize,
        desired: usize,
        maximum: Option<usize>,
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
    fn maybe_async_gc(&mut self, root: Option<VMGcRef>) -> Result<Option<VMGcRef>>;

    /// Metadata required for resources for the component model.
    #[cfg(feature = "component-model")]
    fn component_calls(&mut self) -> &mut component::CallContexts;
}

impl Deref for dyn VMStore + '_ {
    type Target = StoreOpaque;

    fn deref(&self) -> &Self::Target {
        self.store_opaque()
    }
}

impl DerefMut for dyn VMStore + '_ {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.store_opaque_mut()
    }
}

/// A newtype wrapper around `NonNull<dyn VMStore>` intended to be a
/// self-pointer back to the `Store<T>` within raw data structures like
/// `VMContext`.
///
/// This type exists to manually, and unsafely, implement `Send` and `Sync`.
/// The `VMStore` trait doesn't require `Send` or `Sync` which means this isn't
/// naturally either trait (e.g. with `SendSyncPtr` instead). Note that this
/// means that `Instance` is, for example, mistakenly considered
/// unconditionally `Send` and `Sync`. This is hopefully ok for now though
/// because from a user perspective the only type that matters is `Store<T>`.
/// That type is `Send + Sync` if `T: Send + Sync` already so the internal
/// storage of `Instance` shouldn't matter as the final result is the same.
/// Note though that this means we need to be extra vigilant about cross-thread
/// usage of `Instance` and `ComponentInstance` for example.
#[derive(Copy, Clone)]
#[repr(transparent)]
struct VMStoreRawPtr(NonNull<dyn VMStore>);

// SAFETY: this is the purpose of `VMStoreRawPtr`, see docs above about safe
// usage.
unsafe impl Send for VMStoreRawPtr {}
unsafe impl Sync for VMStoreRawPtr {}

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
#[derive(Clone)]
pub enum ModuleRuntimeInfo {
    Module(crate::Module),
    Bare(Box<BareModuleInfo>),
}

/// A barebones implementation of ModuleRuntimeInfo that is useful for
/// cases where a purpose-built environ::Module is used and a full
/// CompiledModule does not exist (for example, for tests or for the
/// default-callee instance).
#[derive(Clone)]
pub struct BareModuleInfo {
    module: Arc<wasmtime_environ::Module>,
    one_signature: Option<VMSharedTypeIndex>,
    offsets: VMOffsets<HostPtr>,
}

impl ModuleRuntimeInfo {
    pub(crate) fn bare(module: Arc<wasmtime_environ::Module>) -> Self {
        ModuleRuntimeInfo::bare_maybe_imported_func(module, None)
    }

    pub(crate) fn bare_maybe_imported_func(
        module: Arc<wasmtime_environ::Module>,
        one_signature: Option<VMSharedTypeIndex>,
    ) -> Self {
        ModuleRuntimeInfo::Bare(Box::new(BareModuleInfo {
            offsets: VMOffsets::new(HostPtr, &module),
            module,
            one_signature,
        }))
    }

    /// The underlying Module.
    pub(crate) fn env_module(&self) -> &Arc<wasmtime_environ::Module> {
        match self {
            ModuleRuntimeInfo::Module(m) => m.env_module(),
            ModuleRuntimeInfo::Bare(b) => &b.module,
        }
    }

    /// Translate a module-level interned type index into an engine-level
    /// interned type index.
    fn engine_type_index(&self, module_index: ModuleInternedTypeIndex) -> VMSharedTypeIndex {
        match self {
            ModuleRuntimeInfo::Module(m) => m
                .code_object()
                .signatures()
                .shared_type(module_index)
                .expect("bad module-level interned type index"),
            ModuleRuntimeInfo::Bare(_) => unreachable!(),
        }
    }

    /// Returns the address, in memory, that the function `index` resides at.
    fn function(&self, index: DefinedFuncIndex) -> NonNull<VMWasmCallFunction> {
        let module = match self {
            ModuleRuntimeInfo::Module(m) => m,
            ModuleRuntimeInfo::Bare(_) => unreachable!(),
        };
        let ptr = module
            .compiled_module()
            .finished_function(index)
            .as_ptr()
            .cast::<VMWasmCallFunction>()
            .cast_mut();
        NonNull::new(ptr).unwrap()
    }

    /// Returns the address, in memory, of the trampoline that allows the given
    /// defined Wasm function to be called by the array calling convention.
    ///
    /// Returns `None` for Wasm functions which do not escape, and therefore are
    /// not callable from outside the Wasm module itself.
    fn array_to_wasm_trampoline(
        &self,
        index: DefinedFuncIndex,
    ) -> Option<NonNull<VMArrayCallFunction>> {
        let m = match self {
            ModuleRuntimeInfo::Module(m) => m,
            ModuleRuntimeInfo::Bare(_) => unreachable!(),
        };
        let ptr = NonNull::from(m.compiled_module().array_to_wasm_trampoline(index)?);
        Some(ptr.cast())
    }

    /// Returns the `MemoryImage` structure used for copy-on-write
    /// initialization of the memory, if it's applicable.
    fn memory_image(
        &self,
        memory: DefinedMemoryIndex,
    ) -> anyhow::Result<Option<&Arc<MemoryImage>>> {
        match self {
            ModuleRuntimeInfo::Module(m) => {
                let images = m.memory_images()?;
                Ok(images.and_then(|images| images.get_memory_image(memory)))
            }
            ModuleRuntimeInfo::Bare(_) => Ok(None),
        }
    }

    /// A unique ID for this particular module. This can be used to
    /// allow for fastpaths to optimize a "re-instantiate the same
    /// module again" case.
    fn unique_id(&self) -> Option<CompiledModuleId> {
        match self {
            ModuleRuntimeInfo::Module(m) => Some(m.id()),
            ModuleRuntimeInfo::Bare(_) => None,
        }
    }

    /// A slice pointing to all data that is referenced by this instance.
    fn wasm_data(&self) -> &[u8] {
        match self {
            ModuleRuntimeInfo::Module(m) => m.compiled_module().code_memory().wasm_data(),
            ModuleRuntimeInfo::Bare(_) => &[],
        }
    }

    /// Returns an array, indexed by `ModuleInternedTypeIndex` of all
    /// `VMSharedSignatureIndex` entries corresponding to the `SignatureIndex`.
    fn type_ids(&self) -> &[VMSharedTypeIndex] {
        match self {
            ModuleRuntimeInfo::Module(m) => m
                .code_object()
                .signatures()
                .as_module_map()
                .values()
                .as_slice(),
            ModuleRuntimeInfo::Bare(b) => match &b.one_signature {
                Some(s) => core::slice::from_ref(s),
                None => &[],
            },
        }
    }

    /// Offset information for the current host.
    pub(crate) fn offsets(&self) -> &VMOffsets<HostPtr> {
        match self {
            ModuleRuntimeInfo::Module(m) => m.offsets(),
            ModuleRuntimeInfo::Bare(b) => &b.offsets,
        }
    }
}

/// Returns the host OS page size, in bytes.
#[cfg(has_virtual_memory)]
pub fn host_page_size() -> usize {
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

/// Result of `Memory::atomic_wait32` and `Memory::atomic_wait64`
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
