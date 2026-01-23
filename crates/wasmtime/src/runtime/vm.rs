//! Runtime library support for Wasmtime.

#![deny(missing_docs)]
// See documentation in crates/wasmtime/src/runtime.rs for why this is
// selectively enabled here.
#![warn(clippy::cast_sign_loss)]

// Polyfill `std::simd::i8x16` etc. until they're stable.
#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
#[expect(non_camel_case_types, reason = "matching wasm conventions")]
pub(crate) type i8x16 = core::arch::x86_64::__m128i;
#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
#[expect(non_camel_case_types, reason = "matching wasm conventions")]
pub(crate) type f32x4 = core::arch::x86_64::__m128;
#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
#[expect(non_camel_case_types, reason = "matching wasm conventions")]
pub(crate) type f64x2 = core::arch::x86_64::__m128d;

// On platforms other than x86_64, define i8x16 to a non-constructible type;
// we need a type because we have a lot of macros for defining builtin
// functions that are awkward to make conditional on the target, but it
// doesn't need to actually be constructible unless we're on x86_64.
#[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
#[expect(non_camel_case_types, reason = "matching wasm conventions")]
#[derive(Copy, Clone)]
pub(crate) struct i8x16(core::convert::Infallible);
#[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
#[expect(non_camel_case_types, reason = "matching wasm conventions")]
#[derive(Copy, Clone)]
pub(crate) struct f32x4(core::convert::Infallible);
#[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
#[expect(non_camel_case_types, reason = "matching wasm conventions")]
#[derive(Copy, Clone)]
pub(crate) struct f64x2(core::convert::Infallible);

use crate::StoreContextMut;
use crate::prelude::*;
use crate::store::{StoreInner, StoreOpaque, StoreResourceLimiter};
use crate::type_registry::RegisteredType;
use alloc::sync::Arc;
use core::fmt;
use core::ops::{Deref, DerefMut};
use core::pin::pin;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{Context, Poll, Waker};
use wasmtime_environ::error::OutOfMemory;
use wasmtime_environ::{DefinedMemoryIndex, HostPtr, VMOffsets, VMSharedTypeIndex};

#[cfg(feature = "gc")]
use wasmtime_environ::ModuleInternedTypeIndex;

mod always_mut;
#[cfg(feature = "component-model")]
pub mod component;
mod const_expr;
mod export;
mod gc;
mod imports;
mod instance;
mod memory;
mod mmap_vec;
#[cfg(has_virtual_memory)]
mod pagemap_disabled;
mod provenance;
mod send_sync_ptr;
mod stack_switching;
mod store_box;
mod sys;
mod table;
#[cfg(feature = "gc")]
mod throw;
mod traphandlers;
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

pub use crate::runtime::vm::always_mut::*;
pub use crate::runtime::vm::export::*;
pub use crate::runtime::vm::gc::*;
pub use crate::runtime::vm::imports::Imports;
pub use crate::runtime::vm::instance::{
    GcHeapAllocationIndex, Instance, InstanceAllocationRequest, InstanceAllocator, InstanceHandle,
    MemoryAllocationIndex, OnDemandInstanceAllocator, TableAllocationIndex, initialize_instance,
};
#[cfg(feature = "pooling-allocator")]
pub use crate::runtime::vm::instance::{
    InstanceLimits, PoolConcurrencyLimitError, PoolingAllocatorMetrics, PoolingInstanceAllocator,
    PoolingInstanceAllocatorConfig,
};
pub use crate::runtime::vm::interpreter::*;
pub use crate::runtime::vm::memory::{
    Memory, MemoryBase, RuntimeLinearMemory, RuntimeMemoryCreator, SharedMemory,
};
pub use crate::runtime::vm::mmap_vec::MmapVec;
pub use crate::runtime::vm::provenance::*;
pub use crate::runtime::vm::stack_switching::*;
pub use crate::runtime::vm::store_box::*;
#[cfg(feature = "std")]
pub use crate::runtime::vm::sys::mmap::open_file_for_mmap;
#[cfg(has_host_compiler_backend)]
pub use crate::runtime::vm::sys::unwind::UnwindRegistration;
pub use crate::runtime::vm::table::{Table, TableElementType};
#[cfg(feature = "gc")]
pub use crate::runtime::vm::throw::*;
pub use crate::runtime::vm::traphandlers::*;
#[cfg(feature = "component-model")]
pub use crate::runtime::vm::vmcontext::VMArrayCallFunction;
pub use crate::runtime::vm::vmcontext::{
    VMArrayCallHostFuncContext, VMContext, VMFuncRef, VMFunctionImport, VMGlobalDefinition,
    VMGlobalImport, VMGlobalKind, VMMemoryDefinition, VMMemoryImport, VMOpaqueContext,
    VMStoreContext, VMTableImport, VMTagImport, VMWasmCallFunction, ValRaw,
};
#[cfg(has_custom_sync)]
pub(crate) use sys::capi;

pub use send_sync_ptr::SendSyncPtr;
pub use wasmtime_unwinder::Unwind;

#[cfg(has_host_compiler_backend)]
pub use wasmtime_unwinder::{UnwindHost, get_stack_pointer};

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

#[cfg(any(feature = "async", feature = "gc"))]
mod async_yield;
#[cfg(any(feature = "async", feature = "gc"))]
pub use crate::runtime::vm::async_yield::*;

#[cfg(feature = "gc-null")]
mod send_sync_unsafe_cell;
#[cfg(feature = "gc-null")]
pub use send_sync_unsafe_cell::SendSyncUnsafeCell;

cfg_if::cfg_if! {
    if #[cfg(has_virtual_memory)] {
        pub use crate::runtime::vm::byte_count::*;
        pub use crate::runtime::vm::mmap::{Mmap, MmapOffset};
        pub use self::cow::{MemoryImage, MemoryImageSlot, ModuleMemoryImages};
    } else {
        pub use self::cow_disabled::{MemoryImage, MemoryImageSlot, ModuleMemoryImages};
    }
}

/// Source of data used for [`MemoryImage`]
pub trait ModuleMemoryImageSource: Send + Sync + 'static {
    /// Returns this image's slice of all wasm data for a module which is then
    /// further sub-sliced for a particular initialization segment.
    fn wasm_data(&self) -> &[u8];

    /// Optionally returns the backing mmap. Used for using the backing mmap's
    /// file to perform other mmaps, for example.
    fn mmap(&self) -> Option<&MmapVec>;
}

/// Dynamic runtime functionality needed by this crate throughout the execution
/// of a wasm instance.
///
/// This trait is used to store a raw pointer trait object within each
/// `VMContext`. This raw pointer trait object points back to the
/// `wasmtime::Store` internally but is type-erased to avoid needing to
/// monomorphize the entire runtime on the `T` in `Store<T>`
///
/// # Safety
///
/// This trait should be implemented by nothing other than `StoreInner<T>` in
/// this crate. It's not sound to implement it for anything else due to
/// `unchecked_context_mut` below.
///
/// It's also worth nothing that there are various locations where a `*mut dyn
/// VMStore` is asserted to be both `Send` and `Sync` which disregards the `T`
/// that's actually stored in the store itself. It's assume that the high-level
/// APIs using `Store<T>` are correctly inferring send/sync on the returned
/// values (e.g. futures) and that internally in the runtime we aren't doing
/// anything "weird" with threads for example.
pub unsafe trait VMStore: 'static {
    /// Get a shared borrow of this store's `StoreOpaque`.
    fn store_opaque(&self) -> &StoreOpaque;

    /// Get an exclusive borrow of this store's `StoreOpaque`.
    fn store_opaque_mut(&mut self) -> &mut StoreOpaque;

    /// Returns a split borrow to the limiter plus `StoreOpaque` at the same
    /// time.
    fn resource_limiter_and_store_opaque(
        &mut self,
    ) -> (Option<StoreResourceLimiter<'_>>, &mut StoreOpaque);

    /// Callback invoked whenever an instance observes a new epoch
    /// number. Cannot fail; cooperative epoch-based yielding is
    /// completely semantically transparent. Returns the new deadline.
    #[cfg(target_has_atomic = "64")]
    fn new_epoch_updated_deadline(&mut self) -> Result<crate::UpdateDeadline>;

    /// Metadata required for resources for the component model.
    #[cfg(feature = "component-model")]
    fn component_calls(&mut self) -> &mut component::CallContexts;

    #[cfg(feature = "component-model-async")]
    fn component_async_store(
        &mut self,
    ) -> &mut dyn crate::runtime::component::VMComponentAsyncStore;

    /// Invoke a debug handler, if present, at a debug event.
    #[cfg(feature = "debug")]
    fn block_on_debug_handler(&mut self, event: crate::DebugEvent) -> crate::Result<()>;
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

impl dyn VMStore + '_ {
    /// Asserts that this `VMStore` was originally paired with `StoreInner<T>`
    /// and then casts to the `StoreContextMut` type.
    ///
    /// # Unsafety
    ///
    /// This method is not safe as there's no static guarantee that `T` is
    /// correct for this store.
    pub(crate) unsafe fn unchecked_context_mut<T>(&mut self) -> StoreContextMut<'_, T> {
        unsafe { StoreContextMut(&mut *(self as *mut dyn VMStore as *mut StoreInner<T>)) }
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
struct VMStoreRawPtr(pub NonNull<dyn VMStore>);

// SAFETY: this is the purpose of `VMStoreRawPtr`, see docs above about safe
// usage.
unsafe impl Send for VMStoreRawPtr {}
unsafe impl Sync for VMStoreRawPtr {}

/// Functionality required by this crate for a particular module. This is
/// chiefly needed for lazy initialization of various bits of instance state.
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
    offsets: VMOffsets<HostPtr>,
    _registered_type: Option<RegisteredType>,
}

impl ModuleRuntimeInfo {
    pub(crate) fn bare(module: Arc<wasmtime_environ::Module>) -> Result<Self, OutOfMemory> {
        ModuleRuntimeInfo::bare_with_registered_type(module, None)
    }

    pub(crate) fn bare_with_registered_type(
        module: Arc<wasmtime_environ::Module>,
        registered_type: Option<RegisteredType>,
    ) -> Result<Self, OutOfMemory> {
        let info = try_new(BareModuleInfo {
            offsets: VMOffsets::new(HostPtr, &module),
            module,
            _registered_type: registered_type,
        })?;
        Ok(ModuleRuntimeInfo::Bare(info))
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
    #[cfg(feature = "gc")]
    fn engine_type_index(&self, module_index: ModuleInternedTypeIndex) -> VMSharedTypeIndex {
        match self {
            ModuleRuntimeInfo::Module(m) => m
                .engine_code()
                .signatures()
                .shared_type(module_index)
                .expect("bad module-level interned type index"),
            ModuleRuntimeInfo::Bare(_) => unreachable!(),
        }
    }

    /// Returns the `MemoryImage` structure used for copy-on-write
    /// initialization of the memory, if it's applicable.
    fn memory_image(&self, memory: DefinedMemoryIndex) -> crate::Result<Option<&Arc<MemoryImage>>> {
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
    #[cfg(feature = "pooling-allocator")]
    fn unique_id(&self) -> Option<CompiledModuleId> {
        match self {
            ModuleRuntimeInfo::Module(m) => Some(m.id()),
            ModuleRuntimeInfo::Bare(_) => None,
        }
    }

    /// A slice pointing to all data that is referenced by this instance.
    fn wasm_data(&self) -> &[u8] {
        match self {
            ModuleRuntimeInfo::Module(m) => m.engine_code().wasm_data(),
            ModuleRuntimeInfo::Bare(_) => &[],
        }
    }

    /// Returns an array, indexed by `ModuleInternedTypeIndex` of all
    /// `VMSharedSignatureIndex` entries corresponding to the `SignatureIndex`.
    fn type_ids(&self) -> &[VMSharedTypeIndex] {
        match self {
            ModuleRuntimeInfo::Module(m) => m
                .engine_code()
                .signatures()
                .as_module_map()
                .values()
                .as_slice(),
            ModuleRuntimeInfo::Bare(_) => &[],
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
    // NB: this function is duplicated in `crates/fiber/src/unix.rs` so if this
    // changes that should probably get updated as well.
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

/// Asserts that the future `f` is ready and returns its output.
///
/// This function is intended to be used with `Store::validate_sync_call`.
/// Internals of Wasmtime are generally `async` when they optionally can be,
/// meaning that synchronous entrypoints will invoke this function after
/// invoking the asynchronous internals. The `validate_sync_call` method
/// ensures that during this `async` function call there won't actually be any
/// yield points. If a yield point could possibly happen, then
/// `validate_sync_call` will fail.
///
/// If `validate_sync_call` passes, then this function is an extra assert that
/// yes, indeed, we coded everything correctly in Wasmtime and there shouldn't
/// be any yield points in the future provided, so its result should be ready
/// immediately.
///
/// # Panics
///
/// Panics if `f` is not yet ready.
pub fn assert_ready<F: Future>(f: F) -> F::Output {
    one_poll(f).unwrap()
}

/// Attempts one poll of `f` to see if its output is available.
///
/// This function is intended for a few minor entrypoints into the Wasmtime API
/// where a synchronous function is documented to work even when `async_support`
/// is enabled. For example growing a `Memory` can be done with a synchronous
/// function, but it's documented to panic with an async resource limiter.
///
/// This function provides the opportunity to poll `f` once to see if its output
/// is available. If it isn't then `None` is returned and an appropriate panic
/// message should be generated recommending to use an async function (e.g.
/// `grow_async` instead of `grow`).
fn one_poll<F: Future>(f: F) -> Option<F::Output> {
    let mut context = Context::from_waker(&Waker::noop());
    match pin!(f).poll(&mut context) {
        Poll::Ready(output) => Some(output),
        Poll::Pending => None,
    }
}
