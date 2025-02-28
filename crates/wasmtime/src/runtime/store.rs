//! Wasmtime's "store" type
//!
//! This module, and its submodules, contain the `Store` type and various types
//! used to interact with it. At first glance this is a pretty confusing module
//! where you need to know the difference between:
//!
//! * `Store<T>`
//! * `StoreContext<T>`
//! * `StoreContextMut<T>`
//! * `AsContext`
//! * `AsContextMut`
//! * `StoreInner<T>`
//! * `StoreOpaque`
//! * `StoreData`
//!
//! There's... quite a lot going on here, and it's easy to be confused. This
//! comment is ideally going to serve the purpose of clarifying what all these
//! types are for and why they're motivated.
//!
//! First it's important to know what's "internal" and what's "external". Almost
//! everything above is defined as `pub`, but only some of the items are
//! reexported to the outside world to be usable from this crate. Otherwise all
//! items are `pub` within this `store` module, and the `store` module is
//! private to the `wasmtime` crate. Notably `Store<T>`, `StoreContext<T>`,
//! `StoreContextMut<T>`, `AsContext`, and `AsContextMut` are all public
//! interfaces to the `wasmtime` crate. You can think of these as:
//!
//! * `Store<T>` - an owned reference to a store, the "root of everything"
//! * `StoreContext<T>` - basically `&StoreInner<T>`
//! * `StoreContextMut<T>` - more-or-less `&mut StoreInner<T>` with caveats.
//!   Explained later.
//! * `AsContext` - similar to `AsRef`, but produces `StoreContext<T>`
//! * `AsContextMut` - similar to `AsMut`, but produces `StoreContextMut<T>`
//!
//! Next comes the internal structure of the `Store<T>` itself. This looks like:
//!
//! * `Store<T>` - this type is just a pointer large. It's primarily just
//!   intended to be consumed by the outside world. Note that the "just a
//!   pointer large" is a load-bearing implementation detail in Wasmtime. This
//!   enables it to store a pointer to its own trait object which doesn't need
//!   to change over time.
//!
//! * `StoreInner<T>` - the first layer of the contents of a `Store<T>`, what's
//!   stored inside the `Box`. This is the general Rust pattern when one struct
//!   is a layer over another. The surprising part, though, is that this is
//!   further subdivided. This structure only contains things which actually
//!   need `T` itself. The downside of this structure is that it's always
//!   generic and means that code is monomorphized into consumer crates. We
//!   strive to have things be as monomorphic as possible in `wasmtime` so this
//!   type is not heavily used.
//!
//! * `StoreOpaque` - this is the primary contents of the `StoreInner<T>` type.
//!   Stored inline in the outer type the "opaque" here means that it's a
//!   "store" but it doesn't have access to the `T`. This is the primary
//!   "internal" reference that Wasmtime uses since `T` is rarely needed by the
//!   internals of Wasmtime.
//!
//! * `StoreData` - this is a final helper struct stored within `StoreOpaque`.
//!   All references of Wasm items into a `Store` are actually indices into a
//!   table in this structure, and the `StoreData` being separate makes it a bit
//!   easier to manage/define/work with. There's no real fundamental reason this
//!   is split out, although sometimes it's useful to have separate borrows into
//!   these tables than the `StoreOpaque`.
//!
//! A major caveat with these representations is that the internal `&mut
//! StoreInner<T>` is never handed out publicly to consumers of this crate, only
//! through a wrapper of `StoreContextMut<'_, T>`. The reason for this is that
//! we want to provide mutable, but not destructive, access to the contents of a
//! `Store`. For example if a `StoreInner<T>` were replaced with some other
//! `StoreInner<T>` then that would drop live instances, possibly those
//! currently executing beneath the current stack frame. This would not be a
//! safe operation.
//!
//! This means, though, that the `wasmtime` crate, which liberally uses `&mut
//! StoreOpaque` internally, has to be careful to never actually destroy the
//! contents of `StoreOpaque`. This is an invariant that we, as the authors of
//! `wasmtime`, must uphold for the public interface to be safe.

use crate::instance::InstanceData;
use crate::linker::Definition;
use crate::module::RegisteredModuleId;
use crate::prelude::*;
use crate::runtime::vm::mpk::ProtectionKey;
#[cfg(feature = "gc")]
use crate::runtime::vm::GcRootsList;
use crate::runtime::vm::{
    ExportGlobal, GcStore, InstanceAllocationRequest, InstanceAllocator, InstanceHandle,
    Interpreter, InterpreterRef, ModuleRuntimeInfo, OnDemandInstanceAllocator, SignalHandler,
    StoreBox, StorePtr, Unwind, VMContext, VMFuncRef, VMGcRef, VMStoreContext,
};
use crate::trampoline::VMHostGlobalContext;
use crate::RootSet;
use crate::{module::ModuleRegistry, Engine, Module, Trap, Val, ValRaw};
use crate::{Global, Instance, Memory, Table, Uninhabited};
use alloc::sync::Arc;
use core::fmt;
use core::marker;
use core::mem::{self, ManuallyDrop};
use core::num::NonZeroU64;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;
use wasmtime_environ::TripleExt;

mod context;
pub use self::context::*;
mod data;
pub use self::data::*;
mod func_refs;
use func_refs::FuncRefs;
#[cfg(feature = "async")]
mod async_;
#[cfg(all(feature = "async", feature = "call-hook"))]
pub use self::async_::CallHookHandler;
#[cfg(feature = "async")]
use self::async_::*;

/// A [`Store`] is a collection of WebAssembly instances and host-defined state.
///
/// All WebAssembly instances and items will be attached to and refer to a
/// [`Store`]. For example instances, functions, globals, and tables are all
/// attached to a [`Store`]. Instances are created by instantiating a
/// [`Module`](crate::Module) within a [`Store`].
///
/// A [`Store`] is intended to be a short-lived object in a program. No form
/// of GC is implemented at this time so once an instance is created within a
/// [`Store`] it will not be deallocated until the [`Store`] itself is dropped.
/// This makes [`Store`] unsuitable for creating an unbounded number of
/// instances in it because [`Store`] will never release this memory. It's
/// recommended to have a [`Store`] correspond roughly to the lifetime of a
/// "main instance" that an embedding is interested in executing.
///
/// ## Type parameter `T`
///
/// Each [`Store`] has a type parameter `T` associated with it. This `T`
/// represents state defined by the host. This state will be accessible through
/// the [`Caller`](crate::Caller) type that host-defined functions get access
/// to. This `T` is suitable for storing `Store`-specific information which
/// imported functions may want access to.
///
/// The data `T` can be accessed through methods like [`Store::data`] and
/// [`Store::data_mut`].
///
/// ## Stores, contexts, oh my
///
/// Most methods in Wasmtime take something of the form
/// [`AsContext`](crate::AsContext) or [`AsContextMut`](crate::AsContextMut) as
/// the first argument. These two traits allow ergonomically passing in the
/// context you currently have to any method. The primary two sources of
/// contexts are:
///
/// * `Store<T>`
/// * `Caller<'_, T>`
///
/// corresponding to what you create and what you have access to in a host
/// function. You can also explicitly acquire a [`StoreContext`] or
/// [`StoreContextMut`] and pass that around as well.
///
/// Note that all methods on [`Store`] are mirrored onto [`StoreContext`],
/// [`StoreContextMut`], and [`Caller`](crate::Caller). This way no matter what
/// form of context you have you can call various methods, create objects, etc.
///
/// ## Stores and `Default`
///
/// You can create a store with default configuration settings using
/// `Store::default()`. This will create a brand new [`Engine`] with default
/// configuration (see [`Config`](crate::Config) for more information).
///
/// ## Cross-store usage of items
///
/// In `wasmtime` wasm items such as [`Global`] and [`Memory`] "belong" to a
/// [`Store`]. The store they belong to is the one they were created with
/// (passed in as a parameter) or instantiated with. This store is the only
/// store that can be used to interact with wasm items after they're created.
///
/// The `wasmtime` crate will panic if the [`Store`] argument passed in to these
/// operations is incorrect. In other words it's considered a programmer error
/// rather than a recoverable error for the wrong [`Store`] to be used when
/// calling APIs.
pub struct Store<T> {
    // for comments about `ManuallyDrop`, see `Store::into_data`
    inner: ManuallyDrop<Box<StoreInner<T>>>,
}

#[derive(Copy, Clone, Debug)]
/// Passed to the argument of [`Store::call_hook`] to indicate a state transition in
/// the WebAssembly VM.
pub enum CallHook {
    /// Indicates the VM is calling a WebAssembly function, from the host.
    CallingWasm,
    /// Indicates the VM is returning from a WebAssembly function, to the host.
    ReturningFromWasm,
    /// Indicates the VM is calling a host function, from WebAssembly.
    CallingHost,
    /// Indicates the VM is returning from a host function, to WebAssembly.
    ReturningFromHost,
}

impl CallHook {
    /// Indicates the VM is entering host code (exiting WebAssembly code)
    pub fn entering_host(&self) -> bool {
        match self {
            CallHook::ReturningFromWasm | CallHook::CallingHost => true,
            _ => false,
        }
    }
    /// Indicates the VM is exiting host code (entering WebAssembly code)
    pub fn exiting_host(&self) -> bool {
        match self {
            CallHook::ReturningFromHost | CallHook::CallingWasm => true,
            _ => false,
        }
    }
}

/// Internal contents of a `Store<T>` that live on the heap.
///
/// The members of this struct are those that need to be generic over `T`, the
/// store's internal type storage. Otherwise all things that don't rely on `T`
/// should go into `StoreOpaque`.
pub struct StoreInner<T> {
    /// Generic metadata about the store that doesn't need access to `T`.
    inner: StoreOpaque,

    limiter: Option<ResourceLimiterInner<T>>,
    call_hook: Option<CallHookInner<T>>,
    #[cfg(target_has_atomic = "64")]
    epoch_deadline_behavior:
        Option<Box<dyn FnMut(StoreContextMut<T>) -> Result<UpdateDeadline> + Send + Sync>>,
    // for comments about `ManuallyDrop`, see `Store::into_data`
    data: ManuallyDrop<T>,
}

enum ResourceLimiterInner<T> {
    Sync(Box<dyn FnMut(&mut T) -> &mut (dyn crate::ResourceLimiter) + Send + Sync>),
    #[cfg(feature = "async")]
    Async(Box<dyn FnMut(&mut T) -> &mut (dyn crate::ResourceLimiterAsync) + Send + Sync>),
}

enum CallHookInner<T> {
    #[cfg(feature = "call-hook")]
    Sync(Box<dyn FnMut(StoreContextMut<'_, T>, CallHook) -> Result<()> + Send + Sync>),
    #[cfg(all(feature = "async", feature = "call-hook"))]
    Async(Box<dyn CallHookHandler<T> + Send + Sync>),
    #[allow(dead_code)]
    ForceTypeParameterToBeUsed {
        uninhabited: Uninhabited,
        _marker: marker::PhantomData<T>,
    },
}

/// What to do after returning from a callback when the engine epoch reaches
/// the deadline for a Store during execution of a function using that store.
pub enum UpdateDeadline {
    /// Extend the deadline by the specified number of ticks.
    Continue(u64),
    /// Extend the deadline by the specified number of ticks after yielding to
    /// the async executor loop. This can only be used with an async [`Store`]
    /// configured via [`Config::async_support`](crate::Config::async_support).
    #[cfg(feature = "async")]
    Yield(u64),
}

// Forward methods on `StoreOpaque` to also being on `StoreInner<T>`
impl<T> Deref for StoreInner<T> {
    type Target = StoreOpaque;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for StoreInner<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Monomorphic storage for a `Store<T>`.
///
/// This structure contains the bulk of the metadata about a `Store`. This is
/// used internally in Wasmtime when dependence on the `T` of `Store<T>` isn't
/// necessary, allowing code to be monomorphic and compiled into the `wasmtime`
/// crate itself.
pub struct StoreOpaque {
    // This `StoreOpaque` structure has references to itself. These aren't
    // immediately evident, however, so we need to tell the compiler that it
    // contains self-references. This notably suppresses `noalias` annotations
    // when this shows up in compiled code because types of this structure do
    // indeed alias itself. An example of this is `default_callee` holds a
    // `*mut dyn Store` to the address of this `StoreOpaque` itself, indeed
    // aliasing!
    //
    // It's somewhat unclear to me at this time if this is 100% sufficient to
    // get all the right codegen in all the right places. For example does
    // `Store` need to internally contain a `Pin<Box<StoreInner<T>>>`? Do the
    // contexts need to contain `Pin<&mut StoreInner<T>>`? I'm not familiar
    // enough with `Pin` to understand if it's appropriate here (we do, for
    // example want to allow movement in and out of `data: T`, just not movement
    // of most of the other members). It's also not clear if using `Pin` in a
    // few places buys us much other than a bunch of `unsafe` that we already
    // sort of hand-wave away.
    //
    // In any case this seems like a good mid-ground for now where we're at
    // least telling the compiler something about all the aliasing happening
    // within a `Store`.
    _marker: marker::PhantomPinned,

    engine: Engine,
    vm_store_context: VMStoreContext,
    instances: Vec<StoreInstance>,
    #[cfg(feature = "component-model")]
    num_component_instances: usize,
    signal_handler: Option<SignalHandler>,
    modules: ModuleRegistry,
    func_refs: FuncRefs,
    host_globals: Vec<StoreBox<VMHostGlobalContext>>,

    // GC-related fields.
    gc_store: Option<GcStore>,
    gc_roots: RootSet,
    #[cfg(feature = "gc")]
    gc_roots_list: GcRootsList,
    // Types for which the embedder has created an allocator for.
    #[cfg(feature = "gc")]
    gc_host_alloc_types: crate::hash_set::HashSet<crate::type_registry::RegisteredType>,

    // Numbers of resources instantiated in this store, and their limits
    instance_count: usize,
    instance_limit: usize,
    memory_count: usize,
    memory_limit: usize,
    table_count: usize,
    table_limit: usize,
    #[cfg(feature = "async")]
    async_state: AsyncState,

    // If fuel_yield_interval is enabled, then we store the remaining fuel (that isn't in
    // runtime_limits) here. The total amount of fuel is the runtime limits and reserve added
    // together. Then when we run out of gas, we inject the yield amount from the reserve
    // until the reserve is empty.
    fuel_reserve: u64,
    fuel_yield_interval: Option<NonZeroU64>,
    /// Indexed data within this `Store`, used to store information about
    /// globals, functions, memories, etc.
    ///
    /// Note that this is `ManuallyDrop` because it needs to be dropped before
    /// `rooted_host_funcs` below. This structure contains pointers which are
    /// otherwise kept alive by the `Arc` references in `rooted_host_funcs`.
    store_data: ManuallyDrop<StoreData>,
    default_caller: InstanceHandle,

    /// Used to optimized wasm->host calls when the host function is defined with
    /// `Func::new` to avoid allocating a new vector each time a function is
    /// called.
    hostcall_val_storage: Vec<Val>,
    /// Same as `hostcall_val_storage`, but for the direction of the host
    /// calling wasm.
    wasm_val_raw_storage: Vec<ValRaw>,

    /// A list of lists of definitions which have been used to instantiate
    /// within this `Store`.
    ///
    /// Note that not all instantiations end up pushing to this list. At the
    /// time of this writing only the `InstancePre<T>` type will push to this
    /// list. Pushes to this list are typically accompanied with
    /// `HostFunc::to_func_store_rooted` to clone an `Arc` here once which
    /// preserves a strong reference to the `Arc` for each `HostFunc` stored
    /// within the list of `Definition`s.
    ///
    /// Note that this is `ManuallyDrop` as it must be dropped after
    /// `store_data` above, where the function pointers are stored.
    rooted_host_funcs: ManuallyDrop<Vec<Arc<[Definition]>>>,

    /// Keep track of what protection key is being used during allocation so
    /// that the right memory pages can be enabled when entering WebAssembly
    /// guest code.
    pkey: Option<ProtectionKey>,

    /// Runtime state for components used in the handling of resources, borrow,
    /// and calls. These also interact with the `ResourceAny` type and its
    /// internal representation.
    #[cfg(feature = "component-model")]
    component_host_table: crate::runtime::vm::component::ResourceTable,
    #[cfg(feature = "component-model")]
    component_calls: crate::runtime::vm::component::CallContexts,
    #[cfg(feature = "component-model")]
    host_resource_data: crate::component::HostResourceData,

    /// State related to the executor of wasm code.
    ///
    /// For example if Pulley is enabled and configured then this will store a
    /// Pulley interpreter.
    executor: Executor,
}

/// Executor state within `StoreOpaque`.
///
/// Effectively stores Pulley interpreter state and handles conditional support
/// for Cranelift at compile time.
enum Executor {
    Interpreter(Interpreter),
    #[cfg(has_host_compiler_backend)]
    Native,
}

/// A borrowed reference to `Executor` above.
pub(crate) enum ExecutorRef<'a> {
    Interpreter(InterpreterRef<'a>),
    #[cfg(has_host_compiler_backend)]
    Native,
}

/// An RAII type to automatically mark a region of code as unsafe for GC.
#[doc(hidden)]
pub struct AutoAssertNoGc<'a> {
    store: &'a mut StoreOpaque,
    entered: bool,
}

impl<'a> AutoAssertNoGc<'a> {
    #[inline]
    pub fn new(store: &'a mut StoreOpaque) -> Self {
        let entered = if !cfg!(feature = "gc") {
            false
        } else if let Some(gc_store) = store.gc_store.as_mut() {
            gc_store.gc_heap.enter_no_gc_scope();
            true
        } else {
            false
        };

        AutoAssertNoGc { store, entered }
    }

    /// Creates an `AutoAssertNoGc` value which is forcibly "not entered" and
    /// disables checks for no GC happening for the duration of this value.
    ///
    /// This is used when it is statically otherwise known that a GC doesn't
    /// happen for the various types involved.
    ///
    /// # Unsafety
    ///
    /// This method is `unsafe` as it does not provide the same safety
    /// guarantees as `AutoAssertNoGc::new`. It must be guaranteed by the
    /// caller that a GC doesn't happen.
    #[inline]
    pub unsafe fn disabled(store: &'a mut StoreOpaque) -> Self {
        if cfg!(debug_assertions) {
            AutoAssertNoGc::new(store)
        } else {
            AutoAssertNoGc {
                store,
                entered: false,
            }
        }
    }
}

impl core::ops::Deref for AutoAssertNoGc<'_> {
    type Target = StoreOpaque;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.store
    }
}

impl core::ops::DerefMut for AutoAssertNoGc<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.store
    }
}

impl Drop for AutoAssertNoGc<'_> {
    #[inline]
    fn drop(&mut self) {
        if self.entered {
            self.store.unwrap_gc_store_mut().gc_heap.exit_no_gc_scope();
        }
    }
}

/// Used to associate instances with the store.
///
/// This is needed to track if the instance was allocated explicitly with the on-demand
/// instance allocator.
struct StoreInstance {
    handle: InstanceHandle,
    kind: StoreInstanceKind,
}

enum StoreInstanceKind {
    /// An actual, non-dummy instance.
    Real {
        /// The id of this instance's module inside our owning store's
        /// `ModuleRegistry`.
        module_id: RegisteredModuleId,
    },

    /// This is a dummy instance that is just an implementation detail for
    /// something else. For example, host-created memories internally create a
    /// dummy instance.
    ///
    /// Regardless of the configured instance allocator for the engine, dummy
    /// instances always use the on-demand allocator to deallocate the instance.
    Dummy,
}

impl<T> Store<T> {
    /// Creates a new [`Store`] to be associated with the given [`Engine`] and
    /// `data` provided.
    ///
    /// The created [`Store`] will place no additional limits on the size of
    /// linear memories or tables at runtime. Linear memories and tables will
    /// be allowed to grow to any upper limit specified in their definitions.
    /// The store will limit the number of instances, linear memories, and
    /// tables created to 10,000. This can be overridden with the
    /// [`Store::limiter`] configuration method.
    pub fn new(engine: &Engine, data: T) -> Self {
        let pkey = engine.allocator().next_available_pkey();

        let mut inner = Box::new(StoreInner {
            inner: StoreOpaque {
                _marker: marker::PhantomPinned,
                engine: engine.clone(),
                vm_store_context: Default::default(),
                instances: Vec::new(),
                #[cfg(feature = "component-model")]
                num_component_instances: 0,
                signal_handler: None,
                gc_store: None,
                gc_roots: RootSet::default(),
                #[cfg(feature = "gc")]
                gc_roots_list: GcRootsList::default(),
                #[cfg(feature = "gc")]
                gc_host_alloc_types: Default::default(),
                modules: ModuleRegistry::default(),
                func_refs: FuncRefs::default(),
                host_globals: Vec::new(),
                instance_count: 0,
                instance_limit: crate::DEFAULT_INSTANCE_LIMIT,
                memory_count: 0,
                memory_limit: crate::DEFAULT_MEMORY_LIMIT,
                table_count: 0,
                table_limit: crate::DEFAULT_TABLE_LIMIT,
                #[cfg(feature = "async")]
                async_state: AsyncState::default(),
                fuel_reserve: 0,
                fuel_yield_interval: None,
                store_data: ManuallyDrop::new(StoreData::new()),
                default_caller: InstanceHandle::null(),
                hostcall_val_storage: Vec::new(),
                wasm_val_raw_storage: Vec::new(),
                rooted_host_funcs: ManuallyDrop::new(Vec::new()),
                pkey,
                #[cfg(feature = "component-model")]
                component_host_table: Default::default(),
                #[cfg(feature = "component-model")]
                component_calls: Default::default(),
                #[cfg(feature = "component-model")]
                host_resource_data: Default::default(),
                #[cfg(has_host_compiler_backend)]
                executor: if cfg!(feature = "pulley") && engine.target().is_pulley() {
                    Executor::Interpreter(Interpreter::new(engine))
                } else {
                    Executor::Native
                },
                #[cfg(not(has_host_compiler_backend))]
                executor: {
                    debug_assert!(engine.target().is_pulley());
                    Executor::Interpreter(Interpreter::new(engine))
                },
            },
            limiter: None,
            call_hook: None,
            #[cfg(target_has_atomic = "64")]
            epoch_deadline_behavior: None,
            data: ManuallyDrop::new(data),
        });

        // Wasmtime uses the callee argument to host functions to learn about
        // the original pointer to the `Store` itself, allowing it to
        // reconstruct a `StoreContextMut<T>`. When we initially call a `Func`,
        // however, there's no "callee" to provide. To fix this we allocate a
        // single "default callee" for the entire `Store`. This is then used as
        // part of `Func::call` to guarantee that the `callee: *mut VMContext`
        // is never null.
        inner.default_caller = {
            let module = Arc::new(wasmtime_environ::Module::default());
            let shim = ModuleRuntimeInfo::bare(module);
            let allocator = OnDemandInstanceAllocator::default();
            allocator
                .validate_module(shim.env_module(), shim.offsets())
                .unwrap();
            let mut instance = unsafe {
                allocator
                    .allocate_module(InstanceAllocationRequest {
                        host_state: Box::new(()),
                        imports: Default::default(),
                        store: StorePtr::empty(),
                        runtime_info: &shim,
                        wmemcheck: engine.config().wmemcheck,
                        pkey: None,
                        tunables: engine.tunables(),
                    })
                    .expect("failed to allocate default callee")
            };

            // Note the erasure of the lifetime here into `'static`, so in
            // general usage of this trait object must be strictly bounded to
            // the `Store` itself, and this is an invariant that we have to
            // maintain throughout Wasmtime.
            unsafe {
                let traitobj = mem::transmute::<
                    NonNull<dyn crate::runtime::vm::VMStore + '_>,
                    NonNull<dyn crate::runtime::vm::VMStore + 'static>,
                >(NonNull::from(&mut *inner));
                instance.set_store(traitobj);
                instance
            }
        };

        Self {
            inner: ManuallyDrop::new(inner),
        }
    }

    /// Access the underlying data owned by this `Store`.
    #[inline]
    pub fn data(&self) -> &T {
        self.inner.data()
    }

    /// Access the underlying data owned by this `Store`.
    #[inline]
    pub fn data_mut(&mut self) -> &mut T {
        self.inner.data_mut()
    }

    /// Consumes this [`Store`], destroying it, and returns the underlying data.
    pub fn into_data(mut self) -> T {
        self.inner.flush_fiber_stack();

        // This is an unsafe operation because we want to avoid having a runtime
        // check or boolean for whether the data is actually contained within a
        // `Store`. The data itself is stored as `ManuallyDrop` since we're
        // manually managing the memory here, and there's also a `ManuallyDrop`
        // around the `Box<StoreInner<T>>`. The way this works though is a bit
        // tricky, so here's how things get dropped appropriately:
        //
        // * When a `Store<T>` is normally dropped, the custom destructor for
        //   `Store<T>` will drop `T`, then the `self.inner` field. The
        //   rustc-glue destructor runs for `Box<StoreInner<T>>` which drops
        //   `StoreInner<T>`. This cleans up all internal fields and doesn't
        //   touch `T` because it's wrapped in `ManuallyDrop`.
        //
        // * When calling this method we skip the top-level destructor for
        //   `Store<T>` with `mem::forget`. This skips both the destructor for
        //   `T` and the destructor for `StoreInner<T>`. We do, however, run the
        //   destructor for `Box<StoreInner<T>>` which, like above, will skip
        //   the destructor for `T` since it's `ManuallyDrop`.
        //
        // In both cases all the other fields of `StoreInner<T>` should all get
        // dropped, and the manual management of destructors is basically
        // between this method and `Drop for Store<T>`. Note that this also
        // means that `Drop for StoreInner<T>` cannot access `self.data`, so
        // there is a comment indicating this as well.
        unsafe {
            let mut inner = ManuallyDrop::take(&mut self.inner);
            core::mem::forget(self);
            ManuallyDrop::take(&mut inner.data)
        }
    }

    /// Configures the [`ResourceLimiter`] used to limit resource creation
    /// within this [`Store`].
    ///
    /// Whenever resources such as linear memory, tables, or instances are
    /// allocated the `limiter` specified here is invoked with the store's data
    /// `T` and the returned [`ResourceLimiter`] is used to limit the operation
    /// being allocated. The returned [`ResourceLimiter`] is intended to live
    /// within the `T` itself, for example by storing a
    /// [`StoreLimits`](crate::StoreLimits).
    ///
    /// Note that this limiter is only used to limit the creation/growth of
    /// resources in the future, this does not retroactively attempt to apply
    /// limits to the [`Store`].
    ///
    /// # Examples
    ///
    /// ```
    /// use wasmtime::*;
    ///
    /// struct MyApplicationState {
    ///     my_state: u32,
    ///     limits: StoreLimits,
    /// }
    ///
    /// let engine = Engine::default();
    /// let my_state = MyApplicationState {
    ///     my_state: 42,
    ///     limits: StoreLimitsBuilder::new()
    ///         .memory_size(1 << 20 /* 1 MB */)
    ///         .instances(2)
    ///         .build(),
    /// };
    /// let mut store = Store::new(&engine, my_state);
    /// store.limiter(|state| &mut state.limits);
    ///
    /// // Creation of smaller memories is allowed
    /// Memory::new(&mut store, MemoryType::new(1, None)).unwrap();
    ///
    /// // Creation of a larger memory, however, will exceed the 1MB limit we've
    /// // configured
    /// assert!(Memory::new(&mut store, MemoryType::new(1000, None)).is_err());
    ///
    /// // The number of instances in this store is limited to 2, so the third
    /// // instance here should fail.
    /// let module = Module::new(&engine, "(module)").unwrap();
    /// assert!(Instance::new(&mut store, &module, &[]).is_ok());
    /// assert!(Instance::new(&mut store, &module, &[]).is_ok());
    /// assert!(Instance::new(&mut store, &module, &[]).is_err());
    /// ```
    ///
    /// [`ResourceLimiter`]: crate::ResourceLimiter
    pub fn limiter(
        &mut self,
        mut limiter: impl FnMut(&mut T) -> &mut (dyn crate::ResourceLimiter) + Send + Sync + 'static,
    ) {
        // Apply the limits on instances, tables, and memory given by the limiter:
        let inner = &mut self.inner;
        let (instance_limit, table_limit, memory_limit) = {
            let l = limiter(&mut inner.data);
            (l.instances(), l.tables(), l.memories())
        };
        let innermost = &mut inner.inner;
        innermost.instance_limit = instance_limit;
        innermost.table_limit = table_limit;
        innermost.memory_limit = memory_limit;

        // Save the limiter accessor function:
        inner.limiter = Some(ResourceLimiterInner::Sync(Box::new(limiter)));
    }

    /// Configure a function that runs on calls and returns between WebAssembly
    /// and host code.
    ///
    /// The function is passed a [`CallHook`] argument, which indicates which
    /// state transition the VM is making.
    ///
    /// This function may return a [`Trap`]. If a trap is returned when an
    /// import was called, it is immediately raised as-if the host import had
    /// returned the trap. If a trap is returned after wasm returns to the host
    /// then the wasm function's result is ignored and this trap is returned
    /// instead.
    ///
    /// After this function returns a trap, it may be called for subsequent returns
    /// to host or wasm code as the trap propagates to the root call.
    #[cfg(feature = "call-hook")]
    pub fn call_hook(
        &mut self,
        hook: impl FnMut(StoreContextMut<'_, T>, CallHook) -> Result<()> + Send + Sync + 'static,
    ) {
        self.inner.call_hook = Some(CallHookInner::Sync(Box::new(hook)));
    }

    /// Returns the [`Engine`] that this store is associated with.
    pub fn engine(&self) -> &Engine {
        self.inner.engine()
    }

    /// Perform garbage collection.
    ///
    /// Note that it is not required to actively call this function. GC will
    /// automatically happen according to various internal heuristics. This is
    /// provided if fine-grained control over the GC is desired.
    ///
    /// This method is only available when the `gc` Cargo feature is enabled.
    #[cfg(feature = "gc")]
    pub fn gc(&mut self) {
        self.inner.gc()
    }

    /// Returns the amount fuel in this [`Store`]. When fuel is enabled, it must
    /// be configured via [`Store::set_fuel`].
    ///
    /// # Errors
    ///
    /// This function will return an error if fuel consumption is not enabled
    /// via [`Config::consume_fuel`](crate::Config::consume_fuel).
    pub fn get_fuel(&self) -> Result<u64> {
        self.inner.get_fuel()
    }

    /// Set the fuel to this [`Store`] for wasm to consume while executing.
    ///
    /// For this method to work fuel consumption must be enabled via
    /// [`Config::consume_fuel`](crate::Config::consume_fuel). By default a
    /// [`Store`] starts with 0 fuel for wasm to execute with (meaning it will
    /// immediately trap). This function must be called for the store to have
    /// some fuel to allow WebAssembly to execute.
    ///
    /// Most WebAssembly instructions consume 1 unit of fuel. Some
    /// instructions, such as `nop`, `drop`, `block`, and `loop`, consume 0
    /// units, as any execution cost associated with them involves other
    /// instructions which do consume fuel.
    ///
    /// Note that when fuel is entirely consumed it will cause wasm to trap.
    ///
    /// # Errors
    ///
    /// This function will return an error if fuel consumption is not enabled via
    /// [`Config::consume_fuel`](crate::Config::consume_fuel).
    pub fn set_fuel(&mut self, fuel: u64) -> Result<()> {
        self.inner.set_fuel(fuel)
    }

    /// Configures a [`Store`] to yield execution of async WebAssembly code
    /// periodically.
    ///
    /// When a [`Store`] is configured to consume fuel with
    /// [`Config::consume_fuel`](crate::Config::consume_fuel) this method will
    /// configure WebAssembly to be suspended and control will be yielded back to the
    /// caller every `interval` units of fuel consumed. This is only suitable with use of
    /// a store associated with an [async config](crate::Config::async_support) because
    /// only then are futures used and yields are possible.
    ///
    /// The purpose of this behavior is to ensure that futures which represent
    /// execution of WebAssembly do not execute too long inside their
    /// `Future::poll` method. This allows for some form of cooperative
    /// multitasking where WebAssembly will voluntarily yield control
    /// periodically (based on fuel consumption) back to the running thread.
    ///
    /// Note that futures returned by this crate will automatically flag
    /// themselves to get re-polled if a yield happens. This means that
    /// WebAssembly will continue to execute, just after giving the host an
    /// opportunity to do something else.
    ///
    /// The `interval` parameter indicates how much fuel should be
    /// consumed between yields of an async future. When fuel runs out wasm will trap.
    ///
    /// # Error
    ///
    /// This method will error if it is not called on a store associated with an [async
    /// config](crate::Config::async_support).
    pub fn fuel_async_yield_interval(&mut self, interval: Option<u64>) -> Result<()> {
        self.inner.fuel_async_yield_interval(interval)
    }

    /// Sets the epoch deadline to a certain number of ticks in the future.
    ///
    /// When the Wasm guest code is compiled with epoch-interruption
    /// instrumentation
    /// ([`Config::epoch_interruption()`](crate::Config::epoch_interruption)),
    /// and when the `Engine`'s epoch is incremented
    /// ([`Engine::increment_epoch()`](crate::Engine::increment_epoch))
    /// past a deadline, execution can be configured to either trap or
    /// yield and then continue.
    ///
    /// This deadline is always set relative to the current epoch:
    /// `ticks_beyond_current` ticks in the future. The deadline can
    /// be set explicitly via this method, or refilled automatically
    /// on a yield if configured via
    /// [`epoch_deadline_async_yield_and_update()`](Store::epoch_deadline_async_yield_and_update). After
    /// this method is invoked, the deadline is reached when
    /// [`Engine::increment_epoch()`] has been invoked at least
    /// `ticks_beyond_current` times.
    ///
    /// By default a store will trap immediately with an epoch deadline of 0
    /// (which has always "elapsed"). This method is required to be configured
    /// for stores with epochs enabled to some future epoch deadline.
    ///
    /// See documentation on
    /// [`Config::epoch_interruption()`](crate::Config::epoch_interruption)
    /// for an introduction to epoch-based interruption.
    #[cfg(target_has_atomic = "64")]
    pub fn set_epoch_deadline(&mut self, ticks_beyond_current: u64) {
        self.inner.set_epoch_deadline(ticks_beyond_current);
    }

    /// Configures epoch-deadline expiration to trap.
    ///
    /// When epoch-interruption-instrumented code is executed on this
    /// store and the epoch deadline is reached before completion,
    /// with the store configured in this way, execution will
    /// terminate with a trap as soon as an epoch check in the
    /// instrumented code is reached.
    ///
    /// This behavior is the default if the store is not otherwise
    /// configured via
    /// [`epoch_deadline_trap()`](Store::epoch_deadline_trap),
    /// [`epoch_deadline_callback()`](Store::epoch_deadline_callback) or
    /// [`epoch_deadline_async_yield_and_update()`](Store::epoch_deadline_async_yield_and_update).
    ///
    /// This setting is intended to allow for coarse-grained
    /// interruption, but not a deterministic deadline of a fixed,
    /// finite interval. For deterministic interruption, see the
    /// "fuel" mechanism instead.
    ///
    /// Note that when this is used it's required to call
    /// [`Store::set_epoch_deadline`] or otherwise wasm will always immediately
    /// trap.
    ///
    /// See documentation on
    /// [`Config::epoch_interruption()`](crate::Config::epoch_interruption)
    /// for an introduction to epoch-based interruption.
    #[cfg(target_has_atomic = "64")]
    pub fn epoch_deadline_trap(&mut self) {
        self.inner.epoch_deadline_trap();
    }

    /// Configures epoch-deadline expiration to invoke a custom callback
    /// function.
    ///
    /// When epoch-interruption-instrumented code is executed on this
    /// store and the epoch deadline is reached before completion, the
    /// provided callback function is invoked.
    ///
    /// This callback should either return an [`UpdateDeadline`], or
    /// return an error, which will terminate execution with a trap.
    ///
    /// The [`UpdateDeadline`] is a positive number of ticks to
    /// add to the epoch deadline, as well as indicating what
    /// to do after the callback returns. If the [`Store`] is
    /// configured with async support, then the callback may return
    /// [`UpdateDeadline::Yield`] to yield to the async executor before
    /// updating the epoch deadline. Alternatively, the callback may
    /// return [`UpdateDeadline::Continue`] to update the epoch deadline
    /// immediately.
    ///
    /// This setting is intended to allow for coarse-grained
    /// interruption, but not a deterministic deadline of a fixed,
    /// finite interval. For deterministic interruption, see the
    /// "fuel" mechanism instead.
    ///
    /// See documentation on
    /// [`Config::epoch_interruption()`](crate::Config::epoch_interruption)
    /// for an introduction to epoch-based interruption.
    #[cfg(target_has_atomic = "64")]
    pub fn epoch_deadline_callback(
        &mut self,
        callback: impl FnMut(StoreContextMut<T>) -> Result<UpdateDeadline> + Send + Sync + 'static,
    ) {
        self.inner.epoch_deadline_callback(Box::new(callback));
    }
}

impl<'a, T> StoreContext<'a, T> {
    pub(crate) fn async_support(&self) -> bool {
        self.0.async_support()
    }

    /// Returns the underlying [`Engine`] this store is connected to.
    pub fn engine(&self) -> &Engine {
        self.0.engine()
    }

    /// Access the underlying data owned by this `Store`.
    ///
    /// Same as [`Store::data`].
    pub fn data(&self) -> &'a T {
        self.0.data()
    }

    /// Returns the remaining fuel in this store.
    ///
    /// For more information see [`Store::get_fuel`].
    pub fn get_fuel(&self) -> Result<u64> {
        self.0.get_fuel()
    }
}

impl<'a, T> StoreContextMut<'a, T> {
    /// Access the underlying data owned by this `Store`.
    ///
    /// Same as [`Store::data`].
    pub fn data(&self) -> &T {
        self.0.data()
    }

    /// Access the underlying data owned by this `Store`.
    ///
    /// Same as [`Store::data_mut`].
    pub fn data_mut(&mut self) -> &mut T {
        self.0.data_mut()
    }

    /// Returns the underlying [`Engine`] this store is connected to.
    pub fn engine(&self) -> &Engine {
        self.0.engine()
    }

    /// Perform garbage collection of `ExternRef`s.
    ///
    /// Same as [`Store::gc`].
    ///
    /// This method is only available when the `gc` Cargo feature is enabled.
    #[cfg(feature = "gc")]
    pub fn gc(&mut self) {
        self.0.gc()
    }

    /// Returns remaining fuel in this store.
    ///
    /// For more information see [`Store::get_fuel`]
    pub fn get_fuel(&self) -> Result<u64> {
        self.0.get_fuel()
    }

    /// Set the amount of fuel in this store.
    ///
    /// For more information see [`Store::set_fuel`]
    pub fn set_fuel(&mut self, fuel: u64) -> Result<()> {
        self.0.set_fuel(fuel)
    }

    /// Configures this `Store` to periodically yield while executing futures.
    ///
    /// For more information see [`Store::fuel_async_yield_interval`]
    pub fn fuel_async_yield_interval(&mut self, interval: Option<u64>) -> Result<()> {
        self.0.fuel_async_yield_interval(interval)
    }

    /// Sets the epoch deadline to a certain number of ticks in the future.
    ///
    /// For more information see [`Store::set_epoch_deadline`].
    #[cfg(target_has_atomic = "64")]
    pub fn set_epoch_deadline(&mut self, ticks_beyond_current: u64) {
        self.0.set_epoch_deadline(ticks_beyond_current);
    }

    /// Configures epoch-deadline expiration to trap.
    ///
    /// For more information see [`Store::epoch_deadline_trap`].
    #[cfg(target_has_atomic = "64")]
    pub fn epoch_deadline_trap(&mut self) {
        self.0.epoch_deadline_trap();
    }
}

impl<T> StoreInner<T> {
    #[inline]
    fn data(&self) -> &T {
        &self.data
    }

    #[inline]
    fn data_mut(&mut self) -> &mut T {
        &mut self.data
    }

    #[inline]
    pub fn call_hook(&mut self, s: CallHook) -> Result<()> {
        if self.inner.pkey.is_none() && self.call_hook.is_none() {
            Ok(())
        } else {
            self.call_hook_slow_path(s)
        }
    }

    fn call_hook_slow_path(&mut self, s: CallHook) -> Result<()> {
        if let Some(pkey) = &self.inner.pkey {
            let allocator = self.engine().allocator();
            match s {
                CallHook::CallingWasm | CallHook::ReturningFromHost => {
                    allocator.restrict_to_pkey(*pkey)
                }
                CallHook::ReturningFromWasm | CallHook::CallingHost => allocator.allow_all_pkeys(),
            }
        }

        // Temporarily take the configured behavior to avoid mutably borrowing
        // multiple times.
        #[cfg_attr(not(feature = "call-hook"), allow(unreachable_patterns))]
        if let Some(mut call_hook) = self.call_hook.take() {
            let result = self.invoke_call_hook(&mut call_hook, s);
            self.call_hook = Some(call_hook);
            return result;
        }

        Ok(())
    }

    fn invoke_call_hook(&mut self, call_hook: &mut CallHookInner<T>, s: CallHook) -> Result<()> {
        match call_hook {
            #[cfg(feature = "call-hook")]
            CallHookInner::Sync(hook) => hook((&mut *self).as_context_mut(), s),

            #[cfg(all(feature = "async", feature = "call-hook"))]
            CallHookInner::Async(handler) => unsafe {
                self.inner
                    .async_cx()
                    .ok_or_else(|| anyhow!("couldn't grab async_cx for call hook"))?
                    .block_on(
                        handler
                            .handle_call_event((&mut *self).as_context_mut(), s)
                            .as_mut(),
                    )?
            },

            CallHookInner::ForceTypeParameterToBeUsed { uninhabited, .. } => {
                let _ = s;
                match *uninhabited {}
            }
        }
    }

    #[cfg(not(feature = "async"))]
    fn flush_fiber_stack(&mut self) {
        // noop shim so code can assume this always exists.
    }
}

fn get_fuel(injected_fuel: i64, fuel_reserve: u64) -> u64 {
    fuel_reserve.saturating_add_signed(-injected_fuel)
}

// Add remaining fuel from the reserve into the active fuel if there is any left.
fn refuel(
    injected_fuel: &mut i64,
    fuel_reserve: &mut u64,
    yield_interval: Option<NonZeroU64>,
) -> bool {
    let fuel = get_fuel(*injected_fuel, *fuel_reserve);
    if fuel > 0 {
        set_fuel(injected_fuel, fuel_reserve, yield_interval, fuel);
        true
    } else {
        false
    }
}

fn set_fuel(
    injected_fuel: &mut i64,
    fuel_reserve: &mut u64,
    yield_interval: Option<NonZeroU64>,
    new_fuel_amount: u64,
) {
    let interval = yield_interval.unwrap_or(NonZeroU64::MAX).get();
    // If we're yielding periodically we only store the "active" amount of fuel into consumed_ptr
    // for the VM to use.
    let injected = core::cmp::min(interval, new_fuel_amount);
    // Fuel in the VM is stored as an i64, so we have to cap the amount of fuel we inject into the
    // VM at once to be i64 range.
    let injected = core::cmp::min(injected, i64::MAX as u64);
    // Add whatever is left over after injection to the reserve for later use.
    *fuel_reserve = new_fuel_amount - injected;
    // Within the VM we increment to count fuel, so inject a negative amount. The VM will halt when
    // this counter is positive.
    *injected_fuel = -(injected as i64);
}

#[doc(hidden)]
impl StoreOpaque {
    pub fn id(&self) -> StoreId {
        self.store_data.id()
    }

    pub fn bump_resource_counts(&mut self, module: &Module) -> Result<()> {
        fn bump(slot: &mut usize, max: usize, amt: usize, desc: &str) -> Result<()> {
            let new = slot.saturating_add(amt);
            if new > max {
                bail!(
                    "resource limit exceeded: {} count too high at {}",
                    desc,
                    new
                );
            }
            *slot = new;
            Ok(())
        }

        let module = module.env_module();
        let memories = module.num_defined_memories();
        let tables = module.num_defined_tables();

        bump(&mut self.instance_count, self.instance_limit, 1, "instance")?;
        bump(
            &mut self.memory_count,
            self.memory_limit,
            memories,
            "memory",
        )?;
        bump(&mut self.table_count, self.table_limit, tables, "table")?;

        Ok(())
    }

    #[inline]
    pub fn async_support(&self) -> bool {
        cfg!(feature = "async") && self.engine().config().async_support
    }

    #[inline]
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    #[inline]
    pub fn store_data(&self) -> &StoreData {
        &self.store_data
    }

    #[inline]
    pub fn store_data_mut(&mut self) -> &mut StoreData {
        &mut self.store_data
    }

    #[inline]
    pub(crate) fn modules(&self) -> &ModuleRegistry {
        &self.modules
    }

    #[inline]
    pub(crate) fn modules_mut(&mut self) -> &mut ModuleRegistry {
        &mut self.modules
    }

    pub(crate) fn func_refs(&mut self) -> &mut FuncRefs {
        &mut self.func_refs
    }

    pub(crate) fn fill_func_refs(&mut self) {
        self.func_refs.fill(&self.modules);
    }

    pub(crate) fn push_instance_pre_func_refs(&mut self, func_refs: Arc<[VMFuncRef]>) {
        self.func_refs.push_instance_pre_func_refs(func_refs);
    }

    pub(crate) fn host_globals(&mut self) -> &mut Vec<StoreBox<VMHostGlobalContext>> {
        &mut self.host_globals
    }

    pub fn module_for_instance(&self, instance: InstanceId) -> Option<&'_ Module> {
        match self.instances[instance.0].kind {
            StoreInstanceKind::Dummy => None,
            StoreInstanceKind::Real { module_id } => {
                let module = self
                    .modules()
                    .lookup_module_by_id(module_id)
                    .expect("should always have a registered module for real instances");
                Some(module)
            }
        }
    }

    pub unsafe fn add_instance(
        &mut self,
        handle: InstanceHandle,
        module_id: RegisteredModuleId,
    ) -> InstanceId {
        self.instances.push(StoreInstance {
            handle: handle.clone(),
            kind: StoreInstanceKind::Real { module_id },
        });
        InstanceId(self.instances.len() - 1)
    }

    /// Add a dummy instance that to the store.
    ///
    /// These are instances that are just implementation details of something
    /// else (e.g. host-created memories that are not actually defined in any
    /// Wasm module) and therefore shouldn't show up in things like core dumps.
    pub unsafe fn add_dummy_instance(&mut self, handle: InstanceHandle) -> InstanceId {
        self.instances.push(StoreInstance {
            handle: handle.clone(),
            kind: StoreInstanceKind::Dummy,
        });
        InstanceId(self.instances.len() - 1)
    }

    pub fn instance(&self, id: InstanceId) -> &InstanceHandle {
        &self.instances[id.0].handle
    }

    pub fn instance_mut(&mut self, id: InstanceId) -> &mut InstanceHandle {
        &mut self.instances[id.0].handle
    }

    /// Get all instances (ignoring dummy instances) within this store.
    pub fn all_instances<'a>(&'a mut self) -> impl ExactSizeIterator<Item = Instance> + 'a {
        let instances = self
            .instances
            .iter()
            .enumerate()
            .filter_map(|(idx, inst)| {
                let id = InstanceId::from_index(idx);
                if let StoreInstanceKind::Dummy = inst.kind {
                    None
                } else {
                    Some(InstanceData::from_id(id))
                }
            })
            .collect::<Vec<_>>();
        instances
            .into_iter()
            .map(|i| Instance::from_wasmtime(i, self))
    }

    /// Get all memories (host- or Wasm-defined) within this store.
    pub fn all_memories<'a>(&'a mut self) -> impl Iterator<Item = Memory> + 'a {
        // NB: Host-created memories have dummy instances. Therefore, we can get
        // all memories in the store by iterating over all instances (including
        // dummy instances) and getting each of their defined memories.
        let mems = self
            .instances
            .iter_mut()
            .flat_map(|instance| instance.handle.defined_memories())
            .collect::<Vec<_>>();
        mems.into_iter()
            .map(|memory| unsafe { Memory::from_wasmtime_memory(memory, self) })
    }

    /// Iterate over all tables (host- or Wasm-defined) within this store.
    pub fn for_each_table(&mut self, mut f: impl FnMut(&mut Self, Table)) {
        // NB: Host-created tables have dummy instances. Therefore, we can get
        // all memories in the store by iterating over all instances (including
        // dummy instances) and getting each of their defined memories.

        struct TempTakeInstances<'a> {
            instances: Vec<StoreInstance>,
            store: &'a mut StoreOpaque,
        }

        impl<'a> TempTakeInstances<'a> {
            fn new(store: &'a mut StoreOpaque) -> Self {
                let instances = mem::take(&mut store.instances);
                Self { instances, store }
            }
        }

        impl Drop for TempTakeInstances<'_> {
            fn drop(&mut self) {
                assert!(self.store.instances.is_empty());
                self.store.instances = mem::take(&mut self.instances);
            }
        }

        let mut temp = TempTakeInstances::new(self);
        for instance in temp.instances.iter_mut() {
            for table in instance.handle.defined_tables() {
                let table = unsafe { Table::from_wasmtime_table(table, temp.store) };
                f(temp.store, table);
            }
        }
    }

    /// Iterate over all globals (host- or Wasm-defined) within this store.
    pub fn for_each_global(&mut self, mut f: impl FnMut(&mut Self, Global)) {
        struct TempTakeHostGlobalsAndInstances<'a> {
            host_globals: Vec<StoreBox<VMHostGlobalContext>>,
            instances: Vec<StoreInstance>,
            store: &'a mut StoreOpaque,
        }

        impl<'a> TempTakeHostGlobalsAndInstances<'a> {
            fn new(store: &'a mut StoreOpaque) -> Self {
                let host_globals = mem::take(&mut store.host_globals);
                let instances = mem::take(&mut store.instances);
                Self {
                    host_globals,
                    instances,
                    store,
                }
            }
        }

        impl Drop for TempTakeHostGlobalsAndInstances<'_> {
            fn drop(&mut self) {
                assert!(self.store.host_globals.is_empty());
                self.store.host_globals = mem::take(&mut self.host_globals);
                assert!(self.store.instances.is_empty());
                self.store.instances = mem::take(&mut self.instances);
            }
        }

        let mut temp = TempTakeHostGlobalsAndInstances::new(self);
        unsafe {
            // First enumerate all the host-created globals.
            for global in temp.host_globals.iter() {
                let export = ExportGlobal {
                    definition: NonNull::from(&mut global.get().as_mut().global),
                    vmctx: None,
                    global: global.get().as_ref().ty.to_wasm_type(),
                };
                let global = Global::from_wasmtime_global(export, temp.store);
                f(temp.store, global);
            }

            // Then enumerate all instances' defined globals.
            for instance in temp.instances.iter_mut() {
                for (_, export) in instance.handle.defined_globals() {
                    let global = Global::from_wasmtime_global(export, temp.store);
                    f(temp.store, global);
                }
            }
        }
    }

    #[cfg_attr(not(target_os = "linux"), allow(dead_code))] // not used on all platforms
    pub fn set_signal_handler(&mut self, handler: Option<SignalHandler>) {
        self.signal_handler = handler;
    }

    #[inline]
    pub fn vm_store_context(&self) -> &VMStoreContext {
        &self.vm_store_context
    }

    #[inline(never)]
    pub(crate) fn allocate_gc_heap(&mut self) -> Result<()> {
        assert!(self.gc_store.is_none());
        let gc_store = allocate_gc_store(self.engine())?;
        self.gc_store = Some(gc_store);
        return Ok(());

        #[cfg(feature = "gc")]
        fn allocate_gc_store(engine: &Engine) -> Result<GcStore> {
            ensure!(
                engine.features().gc_types(),
                "cannot allocate a GC store when GC is disabled at configuration time"
            );
            let (index, heap) = engine
                .allocator()
                .allocate_gc_heap(&**engine.gc_runtime()?)?;
            Ok(GcStore::new(index, heap))
        }

        #[cfg(not(feature = "gc"))]
        fn allocate_gc_store(_engine: &Engine) -> Result<GcStore> {
            bail!("cannot allocate a GC store: the `gc` feature was disabled at compile time")
        }
    }

    #[inline]
    #[cfg(feature = "gc")]
    pub(crate) fn gc_store(&self) -> Result<&GcStore> {
        match &self.gc_store {
            Some(gc_store) => Ok(gc_store),
            None => bail!("GC heap not initialized yet"),
        }
    }

    #[inline]
    pub(crate) fn gc_store_mut(&mut self) -> Result<&mut GcStore> {
        if self.gc_store.is_none() {
            self.allocate_gc_heap()?;
        }
        Ok(self.unwrap_gc_store_mut())
    }

    /// If this store is configured with a GC heap, return a mutable reference
    /// to it. Otherwise, return `None`.
    #[inline]
    pub(crate) fn optional_gc_store_mut(&mut self) -> Result<Option<&mut GcStore>> {
        if cfg!(not(feature = "gc")) || !self.engine.features().gc_types() {
            Ok(None)
        } else {
            Ok(Some(self.gc_store_mut()?))
        }
    }

    #[inline]
    #[cfg(feature = "gc")]
    pub(crate) fn unwrap_gc_store(&self) -> &GcStore {
        self.gc_store
            .as_ref()
            .expect("attempted to access the store's GC heap before it has been allocated")
    }

    #[inline]
    pub(crate) fn unwrap_gc_store_mut(&mut self) -> &mut GcStore {
        self.gc_store
            .as_mut()
            .expect("attempted to access the store's GC heap before it has been allocated")
    }

    #[inline]
    pub(crate) fn gc_roots(&self) -> &RootSet {
        &self.gc_roots
    }

    #[inline]
    #[cfg(feature = "gc")]
    pub(crate) fn gc_roots_mut(&mut self) -> &mut RootSet {
        &mut self.gc_roots
    }

    #[inline]
    pub(crate) fn exit_gc_lifo_scope(&mut self, scope: usize) {
        self.gc_roots.exit_lifo_scope(self.gc_store.as_mut(), scope);
    }

    #[cfg(feature = "gc")]
    pub fn gc(&mut self) {
        // If the GC heap hasn't been initialized, there is nothing to collect.
        if self.gc_store.is_none() {
            return;
        }

        log::trace!("============ Begin GC ===========");

        // Take the GC roots out of `self` so we can borrow it mutably but still
        // call mutable methods on `self`.
        let mut roots = core::mem::take(&mut self.gc_roots_list);

        self.trace_roots(&mut roots);
        self.unwrap_gc_store_mut().gc(unsafe { roots.iter() });

        // Restore the GC roots for the next GC.
        roots.clear();
        self.gc_roots_list = roots;

        log::trace!("============ End GC ===========");
    }

    #[inline]
    #[cfg(not(feature = "gc"))]
    pub fn gc(&mut self) {
        // Nothing to collect.
        //
        // Note that this is *not* a public method, this is just defined for the
        // crate-internal `StoreOpaque` type. This is a convenience so that we
        // don't have to `cfg` every call site.
    }

    #[cfg(feature = "gc")]
    fn trace_roots(&mut self, gc_roots_list: &mut GcRootsList) {
        log::trace!("Begin trace GC roots");

        // We shouldn't have any leftover, stale GC roots.
        assert!(gc_roots_list.is_empty());

        self.trace_wasm_stack_roots(gc_roots_list);
        self.trace_vmctx_roots(gc_roots_list);
        self.trace_user_roots(gc_roots_list);

        log::trace!("End trace GC roots")
    }

    #[cfg(feature = "gc")]
    fn trace_wasm_stack_roots(&mut self, gc_roots_list: &mut GcRootsList) {
        use crate::runtime::vm::{Backtrace, SendSyncPtr};
        use core::ptr::NonNull;

        log::trace!("Begin trace GC roots :: Wasm stack");

        Backtrace::trace(self, |frame| {
            let pc = frame.pc();
            debug_assert!(pc != 0, "we should always get a valid PC for Wasm frames");

            let fp = frame.fp() as *mut usize;
            debug_assert!(
                !fp.is_null(),
                "we should always get a valid frame pointer for Wasm frames"
            );

            let module_info = self
                .modules()
                .lookup_module_by_pc(pc)
                .expect("should have module info for Wasm frame");

            let stack_map = match module_info.lookup_stack_map(pc) {
                Some(sm) => sm,
                None => {
                    log::trace!("No stack map for this Wasm frame");
                    return core::ops::ControlFlow::Continue(());
                }
            };
            log::trace!(
                "We have a stack map that maps {} bytes in this Wasm frame",
                stack_map.frame_size()
            );

            let sp = unsafe { stack_map.sp(fp) };
            for stack_slot in unsafe { stack_map.live_gc_refs(sp) } {
                let raw: u32 = unsafe { core::ptr::read(stack_slot) };
                log::trace!("Stack slot @ {stack_slot:p} = {raw:#x}");

                let gc_ref = VMGcRef::from_raw_u32(raw);
                if gc_ref.is_some() {
                    unsafe {
                        gc_roots_list.add_wasm_stack_root(SendSyncPtr::new(
                            NonNull::new(stack_slot).unwrap(),
                        ));
                    }
                }
            }

            core::ops::ControlFlow::Continue(())
        });

        log::trace!("End trace GC roots :: Wasm stack");
    }

    #[cfg(feature = "gc")]
    fn trace_vmctx_roots(&mut self, gc_roots_list: &mut GcRootsList) {
        log::trace!("Begin trace GC roots :: vmctx");
        self.for_each_global(|store, global| global.trace_root(store, gc_roots_list));
        self.for_each_table(|store, table| table.trace_roots(store, gc_roots_list));
        log::trace!("End trace GC roots :: vmctx");
    }

    #[cfg(feature = "gc")]
    fn trace_user_roots(&mut self, gc_roots_list: &mut GcRootsList) {
        log::trace!("Begin trace GC roots :: user");
        self.gc_roots.trace_roots(gc_roots_list);
        log::trace!("End trace GC roots :: user");
    }

    /// Insert a host-allocated GC type into this store.
    ///
    /// This makes it suitable for the embedder to allocate instances of this
    /// type in this store, and we don't have to worry about the type being
    /// reclaimed (since it is possible that none of the Wasm modules in this
    /// store are holding it alive).
    #[cfg(feature = "gc")]
    pub(crate) fn insert_gc_host_alloc_type(&mut self, ty: crate::type_registry::RegisteredType) {
        self.gc_host_alloc_types.insert(ty);
    }

    pub fn get_fuel(&self) -> Result<u64> {
        anyhow::ensure!(
            self.engine().tunables().consume_fuel,
            "fuel is not configured in this store"
        );
        let injected_fuel = unsafe { *self.vm_store_context.fuel_consumed.get() };
        Ok(get_fuel(injected_fuel, self.fuel_reserve))
    }

    fn refuel(&mut self) -> bool {
        let injected_fuel = unsafe { &mut *self.vm_store_context.fuel_consumed.get() };
        refuel(
            injected_fuel,
            &mut self.fuel_reserve,
            self.fuel_yield_interval,
        )
    }

    pub fn set_fuel(&mut self, fuel: u64) -> Result<()> {
        anyhow::ensure!(
            self.engine().tunables().consume_fuel,
            "fuel is not configured in this store"
        );
        let injected_fuel = unsafe { &mut *self.vm_store_context.fuel_consumed.get() };
        set_fuel(
            injected_fuel,
            &mut self.fuel_reserve,
            self.fuel_yield_interval,
            fuel,
        );
        Ok(())
    }

    pub fn fuel_async_yield_interval(&mut self, interval: Option<u64>) -> Result<()> {
        anyhow::ensure!(
            self.engine().tunables().consume_fuel,
            "fuel is not configured in this store"
        );
        anyhow::ensure!(
            self.engine().config().async_support,
            "async support is not configured in this store"
        );
        anyhow::ensure!(
            interval != Some(0),
            "fuel_async_yield_interval must not be 0"
        );
        self.fuel_yield_interval = interval.and_then(|i| NonZeroU64::new(i));
        // Reset the fuel active + reserve states by resetting the amount.
        self.set_fuel(self.get_fuel()?)
    }

    #[inline]
    pub fn signal_handler(&self) -> Option<*const SignalHandler> {
        let handler = self.signal_handler.as_ref()?;
        Some(handler)
    }

    #[inline]
    pub fn vm_store_context_ptr(&self) -> NonNull<VMStoreContext> {
        NonNull::from(&self.vm_store_context)
    }

    #[inline]
    pub fn default_caller(&self) -> NonNull<VMContext> {
        self.default_caller.vmctx()
    }

    #[inline]
    pub fn traitobj(&self) -> NonNull<dyn crate::runtime::vm::VMStore> {
        self.default_caller.traitobj(self)
    }

    /// Takes the cached `Vec<Val>` stored internally across hostcalls to get
    /// used as part of calling the host in a `Func::new` method invocation.
    #[inline]
    pub fn take_hostcall_val_storage(&mut self) -> Vec<Val> {
        mem::take(&mut self.hostcall_val_storage)
    }

    /// Restores the vector previously taken by `take_hostcall_val_storage`
    /// above back into the store, allowing it to be used in the future for the
    /// next wasm->host call.
    #[inline]
    pub fn save_hostcall_val_storage(&mut self, storage: Vec<Val>) {
        if storage.capacity() > self.hostcall_val_storage.capacity() {
            self.hostcall_val_storage = storage;
        }
    }

    /// Same as `take_hostcall_val_storage`, but for the direction of the host
    /// calling wasm.
    #[inline]
    pub fn take_wasm_val_raw_storage(&mut self) -> Vec<ValRaw> {
        mem::take(&mut self.wasm_val_raw_storage)
    }

    /// Same as `save_hostcall_val_storage`, but for the direction of the host
    /// calling wasm.
    #[inline]
    pub fn save_wasm_val_raw_storage(&mut self, storage: Vec<ValRaw>) {
        if storage.capacity() > self.wasm_val_raw_storage.capacity() {
            self.wasm_val_raw_storage = storage;
        }
    }

    pub(crate) fn push_rooted_funcs(&mut self, funcs: Arc<[Definition]>) {
        self.rooted_host_funcs.push(funcs);
    }

    /// Translates a WebAssembly fault at the native `pc` and native `addr` to a
    /// WebAssembly-relative fault.
    ///
    /// This function may abort the process if `addr` is not found to actually
    /// reside in any linear memory. In such a situation it means that the
    /// segfault was erroneously caught by Wasmtime and is possibly indicative
    /// of a code generator bug.
    ///
    /// This function returns `None` for dynamically-bounds-checked-memories
    /// with spectre mitigations enabled since the hardware fault address is
    /// always zero in these situations which means that the trapping context
    /// doesn't have enough information to report the fault address.
    pub(crate) fn wasm_fault(
        &self,
        pc: usize,
        addr: usize,
    ) -> Option<crate::runtime::vm::WasmFault> {
        // There are a few instances where a "close to zero" pointer is loaded
        // and we expect that to happen:
        //
        // * Explicitly bounds-checked memories with spectre-guards enabled will
        //   cause out-of-bounds accesses to get routed to address 0, so allow
        //   wasm instructions to fault on the null address.
        // * `call_indirect` when invoking a null function pointer may load data
        //   from the a `VMFuncRef` whose address is null, meaning any field of
        //   `VMFuncRef` could be the address of the fault.
        //
        // In these situations where the address is so small it won't be in any
        // instance, so skip the checks below.
        if addr <= mem::size_of::<VMFuncRef>() {
            const _: () = {
                // static-assert that `VMFuncRef` isn't too big to ensure that
                // it lives solely within the first page as we currently only
                // have the guarantee that the first page of memory is unmapped,
                // no more.
                assert!(mem::size_of::<VMFuncRef>() <= 512);
            };
            return None;
        }

        // Search all known instances in this store for this address. Note that
        // this is probably not the speediest way to do this. Traps, however,
        // are generally not expected to be super fast and additionally stores
        // probably don't have all that many instances or memories.
        //
        // If this loop becomes hot in the future, however, it should be
        // possible to precompute maps about linear memories in a store and have
        // a quicker lookup.
        let mut fault = None;
        for instance in self.instances.iter() {
            if let Some(f) = instance.handle.wasm_fault(addr) {
                assert!(fault.is_none());
                fault = Some(f);
            }
        }
        if fault.is_some() {
            return fault;
        }

        cfg_if::cfg_if! {
            if #[cfg(any(feature = "std", unix, windows))] {
                // With the standard library a rich error can be printed here
                // to stderr and the native abort path is used.
                eprintln!(
                    "\
Wasmtime caught a segfault for a wasm program because the faulting instruction
is allowed to segfault due to how linear memories are implemented. The address
that was accessed, however, is not known to any linear memory in use within this
Store. This may be indicative of a critical bug in Wasmtime's code generation
because all addresses which are known to be reachable from wasm won't reach this
message.

    pc:      0x{pc:x}
    address: 0x{addr:x}

This is a possible security issue because WebAssembly has accessed something it
shouldn't have been able to. Other accesses may have succeeded and this one just
happened to be caught. The process will now be aborted to prevent this damage
from going any further and to alert what's going on. If this is a security
issue please reach out to the Wasmtime team via its security policy
at https://bytecodealliance.org/security.
"
                );
                std::process::abort();
            } else if #[cfg(panic = "abort")] {
                // Without the standard library but with `panic=abort` then
                // it's safe to panic as that's known to halt execution. For
                // now avoid the above error message as well since without
                // `std` it's probably best to be a bit more size-conscious.
                let _ = pc;
                panic!("invalid fault");
            } else {
                // Without `std` and with `panic = "unwind"` there's no way to
                // abort the process portably, so flag a compile time error.
                //
                // NB: if this becomes a problem in the future one option would
                // be to extend the `capi.rs` module for no_std platforms, but
                // it remains yet to be seen at this time if this is hit much.
                compile_error!("either `std` or `panic=abort` must be enabled");
                None
            }
        }
    }

    /// Retrieve the store's protection key.
    #[inline]
    pub(crate) fn get_pkey(&self) -> Option<ProtectionKey> {
        self.pkey
    }

    #[inline]
    #[cfg(feature = "component-model")]
    pub(crate) fn component_resource_state(
        &mut self,
    ) -> (
        &mut crate::runtime::vm::component::CallContexts,
        &mut crate::runtime::vm::component::ResourceTable,
        &mut crate::component::HostResourceData,
    ) {
        (
            &mut self.component_calls,
            &mut self.component_host_table,
            &mut self.host_resource_data,
        )
    }

    #[cfg(feature = "component-model")]
    pub(crate) fn push_component_instance(&mut self, instance: crate::component::Instance) {
        // We don't actually need the instance itself right now, but it seems
        // like something we will almost certainly eventually want to keep
        // around, so force callers to provide it.
        let _ = instance;

        self.num_component_instances += 1;
    }

    #[cfg(not(feature = "async"))]
    pub(crate) fn async_guard_range(&self) -> core::ops::Range<*mut u8> {
        core::ptr::null_mut()..core::ptr::null_mut()
    }

    pub(crate) fn executor(&mut self) -> ExecutorRef<'_> {
        match &mut self.executor {
            Executor::Interpreter(i) => ExecutorRef::Interpreter(i.as_interpreter_ref()),
            #[cfg(has_host_compiler_backend)]
            Executor::Native => ExecutorRef::Native,
        }
    }

    pub(crate) fn unwinder(&self) -> &'static dyn Unwind {
        match &self.executor {
            Executor::Interpreter(_) => &crate::runtime::vm::UnwindPulley,
            #[cfg(has_host_compiler_backend)]
            Executor::Native => &crate::runtime::vm::UnwindHost,
        }
    }
}

unsafe impl<T> crate::runtime::vm::VMStore for StoreInner<T> {
    fn store_opaque(&self) -> &StoreOpaque {
        &self.inner
    }

    fn store_opaque_mut(&mut self) -> &mut StoreOpaque {
        &mut self.inner
    }

    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> Result<bool, anyhow::Error> {
        match self.limiter {
            Some(ResourceLimiterInner::Sync(ref mut limiter)) => {
                limiter(&mut self.data).memory_growing(current, desired, maximum)
            }
            #[cfg(feature = "async")]
            Some(ResourceLimiterInner::Async(ref mut limiter)) => unsafe {
                self.inner
                    .async_cx()
                    .expect("ResourceLimiterAsync requires async Store")
                    .block_on(
                        limiter(&mut self.data)
                            .memory_growing(current, desired, maximum)
                            .as_mut(),
                    )?
            },
            None => Ok(true),
        }
    }

    fn memory_grow_failed(&mut self, error: anyhow::Error) -> Result<()> {
        match self.limiter {
            Some(ResourceLimiterInner::Sync(ref mut limiter)) => {
                limiter(&mut self.data).memory_grow_failed(error)
            }
            #[cfg(feature = "async")]
            Some(ResourceLimiterInner::Async(ref mut limiter)) => {
                limiter(&mut self.data).memory_grow_failed(error)
            }
            None => {
                log::debug!("ignoring memory growth failure error: {error:?}");
                Ok(())
            }
        }
    }

    fn table_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> Result<bool, anyhow::Error> {
        // Need to borrow async_cx before the mut borrow of the limiter.
        // self.async_cx() panicks when used with a non-async store, so
        // wrap this in an option.
        #[cfg(feature = "async")]
        let async_cx = if self.async_support()
            && matches!(self.limiter, Some(ResourceLimiterInner::Async(_)))
        {
            Some(self.async_cx().unwrap())
        } else {
            None
        };

        match self.limiter {
            Some(ResourceLimiterInner::Sync(ref mut limiter)) => {
                limiter(&mut self.data).table_growing(current, desired, maximum)
            }
            #[cfg(feature = "async")]
            Some(ResourceLimiterInner::Async(ref mut limiter)) => unsafe {
                async_cx
                    .expect("ResourceLimiterAsync requires async Store")
                    .block_on(limiter(&mut self.data).table_growing(current, desired, maximum))?
            },
            None => Ok(true),
        }
    }

    fn table_grow_failed(&mut self, error: anyhow::Error) -> Result<()> {
        match self.limiter {
            Some(ResourceLimiterInner::Sync(ref mut limiter)) => {
                limiter(&mut self.data).table_grow_failed(error)
            }
            #[cfg(feature = "async")]
            Some(ResourceLimiterInner::Async(ref mut limiter)) => {
                limiter(&mut self.data).table_grow_failed(error)
            }
            None => {
                log::debug!("ignoring table growth failure: {error:?}");
                Ok(())
            }
        }
    }

    fn out_of_gas(&mut self) -> Result<()> {
        if !self.refuel() {
            return Err(Trap::OutOfFuel.into());
        }
        #[cfg(feature = "async")]
        if self.fuel_yield_interval.is_some() {
            self.async_yield_impl()?;
        }
        Ok(())
    }

    #[cfg(target_has_atomic = "64")]
    fn new_epoch(&mut self) -> Result<u64, anyhow::Error> {
        // Temporarily take the configured behavior to avoid mutably borrowing
        // multiple times.
        let mut behavior = self.epoch_deadline_behavior.take();
        let delta_result = match &mut behavior {
            None => Err(Trap::Interrupt.into()),
            Some(callback) => callback((&mut *self).as_context_mut()).and_then(|update| {
                let delta = match update {
                    UpdateDeadline::Continue(delta) => delta,

                    #[cfg(feature = "async")]
                    UpdateDeadline::Yield(delta) => {
                        assert!(
                            self.async_support(),
                            "cannot use `UpdateDeadline::Yield` without enabling async support in the config"
                        );
                        // Do the async yield. May return a trap if future was
                        // canceled while we're yielded.
                        self.async_yield_impl()?;
                        delta
                    }
                };

                // Set a new deadline and return the new epoch deadline so
                // the Wasm code doesn't have to reload it.
                self.set_epoch_deadline(delta);
                Ok(self.get_epoch_deadline())
            })
        };

        // Put back the original behavior which was replaced by `take`.
        self.epoch_deadline_behavior = behavior;
        delta_result
    }

    #[cfg(feature = "gc")]
    fn maybe_async_gc(&mut self, root: Option<VMGcRef>) -> Result<Option<VMGcRef>> {
        let mut scope = crate::RootScope::new(self);
        let store = scope.as_context_mut().0;
        let store_id = store.id();
        let root = root.map(|r| store.gc_roots_mut().push_lifo_root(store_id, r));

        if store.async_support() {
            #[cfg(feature = "async")]
            unsafe {
                let async_cx = store.async_cx();
                let future = store.gc_async();
                async_cx
                    .expect("attempted to pull async context during shutdown")
                    .block_on(future)?;
            }
        } else {
            (**store).gc();
        }

        let root = match root {
            None => None,
            Some(r) => {
                let r = r
                    .get_gc_ref(store)
                    .expect("still in scope")
                    .unchecked_copy();
                Some(store.gc_store_mut()?.clone_gc_ref(&r))
            }
        };

        Ok(root)
    }

    #[cfg(not(feature = "gc"))]
    fn maybe_async_gc(&mut self, root: Option<VMGcRef>) -> Result<Option<VMGcRef>> {
        Ok(root)
    }

    #[cfg(feature = "component-model")]
    fn component_calls(&mut self) -> &mut crate::runtime::vm::component::CallContexts {
        &mut self.component_calls
    }
}

impl<T> StoreInner<T> {
    #[cfg(target_has_atomic = "64")]
    pub(crate) fn set_epoch_deadline(&mut self, delta: u64) {
        // Set a new deadline based on the "epoch deadline delta".
        //
        // Safety: this is safe because the epoch deadline in the
        // `VMStoreContext` is accessed only here and by Wasm guest code
        // running in this store, and we have a `&mut self` here.
        //
        // Also, note that when this update is performed while Wasm is
        // on the stack, the Wasm will reload the new value once we
        // return into it.
        let epoch_deadline = unsafe {
            self.vm_store_context_ptr()
                .as_mut()
                .epoch_deadline
                .get_mut()
        };
        *epoch_deadline = self.engine().current_epoch() + delta;
    }

    #[cfg(target_has_atomic = "64")]
    fn epoch_deadline_trap(&mut self) {
        self.epoch_deadline_behavior = None;
    }

    #[cfg(target_has_atomic = "64")]
    fn epoch_deadline_callback(
        &mut self,
        callback: Box<dyn FnMut(StoreContextMut<T>) -> Result<UpdateDeadline> + Send + Sync>,
    ) {
        self.epoch_deadline_behavior = Some(callback);
    }

    fn get_epoch_deadline(&self) -> u64 {
        // Safety: this is safe because, as above, it is only invoked
        // from within `new_epoch` which is called from guest Wasm
        // code, which will have an exclusive borrow on the Store.
        let epoch_deadline = unsafe {
            self.vm_store_context_ptr()
                .as_mut()
                .epoch_deadline
                .get_mut()
        };
        *epoch_deadline
    }
}

impl<T: Default> Default for Store<T> {
    fn default() -> Store<T> {
        Store::new(&Engine::default(), T::default())
    }
}

impl<T: fmt::Debug> fmt::Debug for Store<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let inner = &**self.inner as *const StoreInner<T>;
        f.debug_struct("Store")
            .field("inner", &inner)
            .field("data", &self.inner.data)
            .finish()
    }
}

impl<T> Drop for Store<T> {
    fn drop(&mut self) {
        self.inner.flush_fiber_stack();

        // for documentation on this `unsafe`, see `into_data`.
        unsafe {
            ManuallyDrop::drop(&mut self.inner.data);
            ManuallyDrop::drop(&mut self.inner);
        }
    }
}

impl Drop for StoreOpaque {
    fn drop(&mut self) {
        // NB it's important that this destructor does not access `self.data`.
        // That is deallocated by `Drop for Store<T>` above.

        unsafe {
            let allocator = self.engine.allocator();
            let ondemand = OnDemandInstanceAllocator::default();
            for instance in self.instances.iter_mut() {
                if let StoreInstanceKind::Dummy = instance.kind {
                    ondemand.deallocate_module(&mut instance.handle);
                } else {
                    allocator.deallocate_module(&mut instance.handle);
                }
            }
            ondemand.deallocate_module(&mut self.default_caller);

            #[cfg(feature = "gc")]
            if let Some(gc_store) = self.gc_store.take() {
                debug_assert!(self.engine.features().gc_types());
                allocator.deallocate_gc_heap(gc_store.allocation_index, gc_store.gc_heap);
            }

            #[cfg(feature = "component-model")]
            {
                for _ in 0..self.num_component_instances {
                    allocator.decrement_component_instance_count();
                }
            }

            // See documentation for these fields on `StoreOpaque` for why they
            // must be dropped in this order.
            ManuallyDrop::drop(&mut self.store_data);
            ManuallyDrop::drop(&mut self.rooted_host_funcs);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{get_fuel, refuel, set_fuel};
    use std::num::NonZeroU64;

    struct FuelTank {
        pub consumed_fuel: i64,
        pub reserve_fuel: u64,
        pub yield_interval: Option<NonZeroU64>,
    }

    impl FuelTank {
        fn new() -> Self {
            FuelTank {
                consumed_fuel: 0,
                reserve_fuel: 0,
                yield_interval: None,
            }
        }
        fn get_fuel(&self) -> u64 {
            get_fuel(self.consumed_fuel, self.reserve_fuel)
        }
        fn refuel(&mut self) -> bool {
            refuel(
                &mut self.consumed_fuel,
                &mut self.reserve_fuel,
                self.yield_interval,
            )
        }
        fn set_fuel(&mut self, fuel: u64) {
            set_fuel(
                &mut self.consumed_fuel,
                &mut self.reserve_fuel,
                self.yield_interval,
                fuel,
            );
        }
    }

    #[test]
    fn smoke() {
        let mut tank = FuelTank::new();
        tank.set_fuel(10);
        assert_eq!(tank.consumed_fuel, -10);
        assert_eq!(tank.reserve_fuel, 0);

        tank.yield_interval = NonZeroU64::new(10);
        tank.set_fuel(25);
        assert_eq!(tank.consumed_fuel, -10);
        assert_eq!(tank.reserve_fuel, 15);
    }

    #[test]
    fn does_not_lose_precision() {
        let mut tank = FuelTank::new();
        tank.set_fuel(u64::MAX);
        assert_eq!(tank.get_fuel(), u64::MAX);

        tank.set_fuel(i64::MAX as u64);
        assert_eq!(tank.get_fuel(), i64::MAX as u64);

        tank.set_fuel(i64::MAX as u64 + 1);
        assert_eq!(tank.get_fuel(), i64::MAX as u64 + 1);
    }

    #[test]
    fn yielding_does_not_lose_precision() {
        let mut tank = FuelTank::new();

        tank.yield_interval = NonZeroU64::new(10);
        tank.set_fuel(u64::MAX);
        assert_eq!(tank.get_fuel(), u64::MAX);
        assert_eq!(tank.consumed_fuel, -10);
        assert_eq!(tank.reserve_fuel, u64::MAX - 10);

        tank.yield_interval = NonZeroU64::new(u64::MAX);
        tank.set_fuel(u64::MAX);
        assert_eq!(tank.get_fuel(), u64::MAX);
        assert_eq!(tank.consumed_fuel, -i64::MAX);
        assert_eq!(tank.reserve_fuel, u64::MAX - (i64::MAX as u64));

        tank.yield_interval = NonZeroU64::new((i64::MAX as u64) + 1);
        tank.set_fuel(u64::MAX);
        assert_eq!(tank.get_fuel(), u64::MAX);
        assert_eq!(tank.consumed_fuel, -i64::MAX);
        assert_eq!(tank.reserve_fuel, u64::MAX - (i64::MAX as u64));
    }

    #[test]
    fn refueling() {
        // It's possible to fuel to have consumed over the limit as some instructions can consume
        // multiple units of fuel at once. Refueling should be strict in it's consumption and not
        // add more fuel than there is.
        let mut tank = FuelTank::new();

        tank.yield_interval = NonZeroU64::new(10);
        tank.reserve_fuel = 42;
        tank.consumed_fuel = 4;
        assert!(tank.refuel());
        assert_eq!(tank.reserve_fuel, 28);
        assert_eq!(tank.consumed_fuel, -10);

        tank.yield_interval = NonZeroU64::new(1);
        tank.reserve_fuel = 8;
        tank.consumed_fuel = 4;
        assert_eq!(tank.get_fuel(), 4);
        assert!(tank.refuel());
        assert_eq!(tank.reserve_fuel, 3);
        assert_eq!(tank.consumed_fuel, -1);
        assert_eq!(tank.get_fuel(), 4);

        tank.yield_interval = NonZeroU64::new(10);
        tank.reserve_fuel = 3;
        tank.consumed_fuel = 4;
        assert_eq!(tank.get_fuel(), 0);
        assert!(!tank.refuel());
        assert_eq!(tank.reserve_fuel, 3);
        assert_eq!(tank.consumed_fuel, 4);
        assert_eq!(tank.get_fuel(), 0);
    }
}
