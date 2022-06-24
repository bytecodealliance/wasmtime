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

use crate::linker::Definition;
use crate::module::BareModuleInfo;
use crate::{module::ModuleRegistry, Engine, Module, Trap, Val, ValRaw};
use anyhow::{bail, Result};
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;
use std::future::Future;
use std::marker;
use std::mem::{self, ManuallyDrop};
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::ptr;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::task::{Context, Poll};
use wasmtime_runtime::{
    InstanceAllocationRequest, InstanceAllocator, InstanceHandle, ModuleInfo,
    OnDemandInstanceAllocator, SignalHandler, StorePtr, VMCallerCheckedAnyfunc, VMContext,
    VMExternRef, VMExternRefActivationsTable, VMRuntimeLimits, VMSharedSignatureIndex,
    VMTrampoline,
};

mod context;
pub use self::context::*;
mod data;
pub use self::data::*;

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
/// recommended to have a [`Store`] correspond roughly to the lifetime of a "main
/// instance" that an embedding is interested in executing.
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
    epoch_deadline_behavior: EpochDeadline<T>,
    // for comments about `ManuallyDrop`, see `Store::into_data`
    data: ManuallyDrop<T>,
}

enum ResourceLimiterInner<T> {
    Sync(Box<dyn FnMut(&mut T) -> &mut (dyn crate::ResourceLimiter) + Send + Sync>),
    #[cfg(feature = "async")]
    Async(Box<dyn FnMut(&mut T) -> &mut (dyn crate::ResourceLimiterAsync) + Send + Sync>),
}

/// An object that can take callbacks when the runtime enters or exits hostcalls.
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait CallHookHandler<T>: Send {
    /// A callback to run when wasmtime is about to enter a host call, or when about to
    /// exit the hostcall.
    async fn handle_call_event(&self, t: &mut T, ch: CallHook) -> Result<(), crate::Trap>;
}

enum CallHookInner<T> {
    Sync(Box<dyn FnMut(&mut T, CallHook) -> Result<(), crate::Trap> + Send + Sync>),
    #[cfg(feature = "async")]
    Async(Box<dyn CallHookHandler<T> + Send + Sync>),
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
    runtime_limits: VMRuntimeLimits,
    instances: Vec<StoreInstance>,
    signal_handler: Option<Box<SignalHandler<'static>>>,
    externref_activations_table: VMExternRefActivationsTable,
    modules: ModuleRegistry,

    // See documentation on `StoreOpaque::lookup_trampoline` for what these
    // fields are doing.
    host_trampolines: HashMap<VMSharedSignatureIndex, VMTrampoline>,
    host_func_trampolines_registered: usize,

    // Numbers of resources instantiated in this store, and their limits
    instance_count: usize,
    instance_limit: usize,
    memory_count: usize,
    memory_limit: usize,
    table_count: usize,
    table_limit: usize,
    /// An adjustment to add to the fuel consumed value in `runtime_limits` above
    /// to get the true amount of fuel consumed.
    fuel_adj: i64,
    #[cfg(feature = "async")]
    async_state: AsyncState,
    out_of_gas_behavior: OutOfGas,
    /// Indexed data within this `Store`, used to store information about
    /// globals, functions, memories, etc.
    ///
    /// Note that this is `ManuallyDrop` because it needs to be dropped before
    /// `rooted_host_funcs` below. This structure contains pointers which are
    /// otherwise kept alive by the `Arc` references in `rooted_host_funcs`.
    store_data: ManuallyDrop<StoreData>,
    default_callee: InstanceHandle,

    /// Used to optimzed wasm->host calls when the host function is defined with
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
}

#[cfg(feature = "async")]
struct AsyncState {
    current_suspend:
        UnsafeCell<*const wasmtime_fiber::Suspend<Result<(), Trap>, (), Result<(), Trap>>>,
    current_poll_cx: UnsafeCell<*mut Context<'static>>,
}

// Lots of pesky unsafe cells and pointers in this structure. This means we need
// to declare explicitly that we use this in a threadsafe fashion.
#[cfg(feature = "async")]
unsafe impl Send for AsyncState {}
#[cfg(feature = "async")]
unsafe impl Sync for AsyncState {}

/// An RAII type to automatically mark a region of code as unsafe for GC.
pub(crate) struct AutoAssertNoGc<T>
where
    T: std::ops::DerefMut<Target = StoreOpaque>,
{
    #[cfg(debug_assertions)]
    prev_okay: bool,
    store: T,
}

impl<T> AutoAssertNoGc<T>
where
    T: std::ops::DerefMut<Target = StoreOpaque>,
{
    pub fn new(mut store: T) -> Self {
        drop(&mut store);
        #[cfg(debug_assertions)]
        {
            let prev_okay = store.externref_activations_table.set_gc_okay(false);
            return AutoAssertNoGc { store, prev_okay };
        }
        #[cfg(not(debug_assertions))]
        {
            return AutoAssertNoGc { store };
        }
    }
}

impl<T> std::ops::Deref for AutoAssertNoGc<T>
where
    T: std::ops::DerefMut<Target = StoreOpaque>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.store
    }
}

impl<T> std::ops::DerefMut for AutoAssertNoGc<T>
where
    T: std::ops::DerefMut<Target = StoreOpaque>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.store
    }
}

impl<T> Drop for AutoAssertNoGc<T>
where
    T: std::ops::DerefMut<Target = StoreOpaque>,
{
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        {
            self.store
                .externref_activations_table
                .set_gc_okay(self.prev_okay);
        }
    }
}

/// Used to associate instances with the store.
///
/// This is needed to track if the instance was allocated explicitly with the on-demand
/// instance allocator.
struct StoreInstance {
    handle: InstanceHandle,
    // Stores whether or not to use the on-demand allocator to deallocate the instance
    ondemand: bool,
}

#[derive(Copy, Clone)]
enum OutOfGas {
    Trap,
    InjectFuel {
        injection_count: u64,
        fuel_to_inject: u64,
    },
}

/// What to do when the engine epoch reaches the deadline for a Store
/// during execution of a function using that store.
enum EpochDeadline<T> {
    /// Return early with a trap.
    Trap,
    /// Call a custom deadline handler.
    Callback(Box<dyn FnMut(&mut T) -> Result<u64> + Send + Sync>),
    /// Extend the deadline by the specified number of ticks after
    /// yielding to the async executor loop.
    #[cfg(feature = "async")]
    YieldAndExtendDeadline { delta: u64 },
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
        // Wasmtime uses the callee argument to host functions to learn about
        // the original pointer to the `Store` itself, allowing it to
        // reconstruct a `StoreContextMut<T>`. When we initially call a `Func`,
        // however, there's no "callee" to provide. To fix this we allocate a
        // single "default callee" for the entire `Store`. This is then used as
        // part of `Func::call` to guarantee that the `callee: *mut VMContext`
        // is never null.
        let default_callee = unsafe {
            let module = Arc::new(wasmtime_environ::Module::default());
            let shim = BareModuleInfo::empty(module).into_traitobj();
            OnDemandInstanceAllocator::default()
                .allocate(InstanceAllocationRequest {
                    host_state: Box::new(()),
                    imports: Default::default(),
                    store: StorePtr::empty(),
                    runtime_info: &shim,
                })
                .expect("failed to allocate default callee")
        };

        let mut inner = Box::new(StoreInner {
            inner: StoreOpaque {
                _marker: marker::PhantomPinned,
                engine: engine.clone(),
                runtime_limits: Default::default(),
                instances: Vec::new(),
                signal_handler: None,
                externref_activations_table: VMExternRefActivationsTable::new(),
                modules: ModuleRegistry::default(),
                host_trampolines: HashMap::default(),
                host_func_trampolines_registered: 0,
                instance_count: 0,
                instance_limit: crate::DEFAULT_INSTANCE_LIMIT,
                memory_count: 0,
                memory_limit: crate::DEFAULT_MEMORY_LIMIT,
                table_count: 0,
                table_limit: crate::DEFAULT_TABLE_LIMIT,
                fuel_adj: 0,
                #[cfg(feature = "async")]
                async_state: AsyncState {
                    current_suspend: UnsafeCell::new(ptr::null()),
                    current_poll_cx: UnsafeCell::new(ptr::null_mut()),
                },
                out_of_gas_behavior: OutOfGas::Trap,
                store_data: ManuallyDrop::new(StoreData::new()),
                default_callee,
                hostcall_val_storage: Vec::new(),
                wasm_val_raw_storage: Vec::new(),
                rooted_host_funcs: ManuallyDrop::new(Vec::new()),
            },
            limiter: None,
            call_hook: None,
            epoch_deadline_behavior: EpochDeadline::Trap,
            data: ManuallyDrop::new(data),
        });

        // Once we've actually allocated the store itself we can configure the
        // trait object pointer of the default callee. Note the erasure of the
        // lifetime here into `'static`, so in general usage of this trait
        // object must be strictly bounded to the `Store` itself, and is a
        // variant that we have to maintain throughout Wasmtime.
        unsafe {
            let traitobj = std::mem::transmute::<
                *mut (dyn wasmtime_runtime::Store + '_),
                *mut (dyn wasmtime_runtime::Store + 'static),
            >(&mut *inner);
            inner.default_callee.set_store(traitobj);
        }

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
            std::mem::forget(self);
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

    /// Configures the [`ResourceLimiterAsync`](crate::ResourceLimiterAsync)
    /// used to limit resource creation within this [`Store`].
    ///
    /// This method is an asynchronous variant of the [`Store::limiter`] method
    /// where the embedder can block the wasm request for more resources with
    /// host `async` execution of futures.
    ///
    /// By using a [`ResourceLimiterAsync`](`crate::ResourceLimiterAsync`)
    /// with a [`Store`], you can no longer use
    /// [`Memory::new`](`crate::Memory::new`),
    /// [`Memory::grow`](`crate::Memory::grow`),
    /// [`Table::new`](`crate::Table::new`), and
    /// [`Table::grow`](`crate::Table::grow`). Instead, you must use their
    /// `async` variants: [`Memory::new_async`](`crate::Memory::new_async`),
    /// [`Memory::grow_async`](`crate::Memory::grow_async`),
    /// [`Table::new_async`](`crate::Table::new_async`), and
    /// [`Table::grow_async`](`crate::Table::grow_async`).
    ///
    /// Note that this limiter is only used to limit the creation/growth of
    /// resources in the future, this does not retroactively attempt to apply
    /// limits to the [`Store`]. Additionally this must be used with an async
    /// [`Store`] configured via
    /// [`Config::async_support`](crate::Config::async_support).
    #[cfg(feature = "async")]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "async")))]
    pub fn limiter_async(
        &mut self,
        mut limiter: impl FnMut(&mut T) -> &mut (dyn crate::ResourceLimiterAsync)
            + Send
            + Sync
            + 'static,
    ) {
        debug_assert!(self.inner.async_support());
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
        inner.limiter = Some(ResourceLimiterInner::Async(Box::new(limiter)));
    }

    #[cfg_attr(nightlydoc, doc(cfg(feature = "async")))]
    /// Configures an async function that runs on calls and returns between
    /// WebAssembly and host code. For the non-async equivalent of this method,
    /// see [`Store::call_hook`].
    ///
    /// The function is passed a [`CallHook`] argument, which indicates which
    /// state transition the VM is making.
    ///
    /// This function's future may return a [`Trap`]. If a trap is returned
    /// when an import was called, it is immediately raised as-if the host
    /// import had returned the trap. If a trap is returned after wasm returns
    /// to the host then the wasm function's result is ignored and this trap is
    /// returned instead.
    ///
    /// After this function returns a trap, it may be called for subsequent
    /// returns to host or wasm code as the trap propagates to the root call.
    #[cfg(feature = "async")]
    pub fn call_hook_async(&mut self, hook: impl CallHookHandler<T> + Send + Sync + 'static) {
        self.inner.call_hook = Some(CallHookInner::Async(Box::new(hook)));
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
    pub fn call_hook(
        &mut self,
        hook: impl FnMut(&mut T, CallHook) -> Result<(), Trap> + Send + Sync + 'static,
    ) {
        self.inner.call_hook = Some(CallHookInner::Sync(Box::new(hook)));
    }

    /// Returns the [`Engine`] that this store is associated with.
    pub fn engine(&self) -> &Engine {
        self.inner.engine()
    }

    /// Perform garbage collection of `ExternRef`s.
    ///
    /// Note that it is not required to actively call this function. GC will
    /// automatically happen when internal buffers fill up. This is provided if
    /// fine-grained control over the GC is desired.
    pub fn gc(&mut self) {
        self.inner.gc()
    }

    /// Returns the amount of fuel consumed by this store's execution so far.
    ///
    /// If fuel consumption is not enabled via
    /// [`Config::consume_fuel`](crate::Config::consume_fuel) then this
    /// function will return `None`. Also note that fuel, if enabled, must be
    /// originally configured via [`Store::add_fuel`].
    pub fn fuel_consumed(&self) -> Option<u64> {
        self.inner.fuel_consumed()
    }

    /// Adds fuel to this [`Store`] for wasm to consume while executing.
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
    /// Note that at this time when fuel is entirely consumed it will cause
    /// wasm to trap. More usages of fuel are planned for the future.
    ///
    /// # Panics
    ///
    /// This function will panic if the store's [`Config`](crate::Config) did
    /// not have fuel consumption enabled.
    pub fn add_fuel(&mut self, fuel: u64) -> Result<()> {
        self.inner.add_fuel(fuel)
    }

    /// Synthetically consumes fuel from this [`Store`].
    ///
    /// For this method to work fuel consumption must be enabled via
    /// [`Config::consume_fuel`](crate::Config::consume_fuel).
    ///
    /// WebAssembly execution will automatically consume fuel but if so desired
    /// the embedder can also consume fuel manually to account for relative
    /// costs of host functions, for example.
    ///
    /// This function will attempt to consume `fuel` units of fuel from within
    /// this store. If the remaining amount of fuel allows this then `Ok(N)` is
    /// returned where `N` is the amount of remaining fuel. Otherwise an error
    /// is returned and no fuel is consumed.
    ///
    /// # Errors
    ///
    /// This function will return an either either if fuel consumption via
    /// [`Config`](crate::Config) is disabled or if `fuel` exceeds the amount
    /// of remaining fuel within this store.
    pub fn consume_fuel(&mut self, fuel: u64) -> Result<u64> {
        self.inner.consume_fuel(fuel)
    }

    /// Configures a [`Store`] to generate a [`Trap`] whenever it runs out of
    /// fuel.
    ///
    /// When a [`Store`] is configured to consume fuel with
    /// [`Config::consume_fuel`](crate::Config::consume_fuel) this method will
    /// configure what happens when fuel runs out. Specifically a WebAssembly
    /// trap will be raised and the current execution of WebAssembly will be
    /// aborted.
    ///
    /// This is the default behavior for running out of fuel.
    pub fn out_of_fuel_trap(&mut self) {
        self.inner.out_of_fuel_trap()
    }

    /// Configures a [`Store`] to yield execution of async WebAssembly code
    /// periodically.
    ///
    /// When a [`Store`] is configured to consume fuel with
    /// [`Config::consume_fuel`](crate::Config::consume_fuel) this method will
    /// configure what happens when fuel runs out. Specifically executing
    /// WebAssembly will be suspended and control will be yielded back to the
    /// caller. This is only suitable with use of a store associated with an [async
    /// config](crate::Config::async_support) because only then are futures used and yields
    /// are possible.
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
    /// The `fuel_to_inject` parameter indicates how much fuel should be
    /// automatically re-injected after fuel runs out. This is how much fuel
    /// will be consumed between yields of an async future.
    ///
    /// The `injection_count` parameter indicates how many times this fuel will
    /// be injected. Multiplying the two parameters is the total amount of fuel
    /// this store is allowed before wasm traps.
    ///
    /// # Panics
    ///
    /// This method will panic if it is not called on a store associated with an [async
    /// config](crate::Config::async_support).
    pub fn out_of_fuel_async_yield(&mut self, injection_count: u64, fuel_to_inject: u64) {
        self.inner
            .out_of_fuel_async_yield(injection_count, fuel_to_inject)
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
    /// `delta_beyond_current` ticks in the future. The deadline can
    /// be set explicitly via this method, or refilled automatically
    /// on a yield if configured via
    /// [`epoch_deadline_async_yield_and_update()`](Store::epoch_deadline_async_yield_and_update). After
    /// this method is invoked, the deadline is reached when
    /// [`Engine::increment_epoch()`] has been invoked at least
    /// `ticks_beyond_current` times.
    ///
    /// See documentation on
    /// [`Config::epoch_interruption()`](crate::Config::epoch_interruption)
    /// for an introduction to epoch-based interruption.
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
    /// See documentation on
    /// [`Config::epoch_interruption()`](crate::Config::epoch_interruption)
    /// for an introduction to epoch-based interruption.
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
    /// This function should return a positive `delta`, which is used to
    /// update the new epoch, setting it to the current epoch plus
    /// `delta` ticks. Alternatively, the callback may return an error,
    /// which will terminate execution.
    ///
    /// This setting is intended to allow for coarse-grained
    /// interruption, but not a deterministic deadline of a fixed,
    /// finite interval. For deterministic interruption, see the
    /// "fuel" mechanism instead.
    ///
    /// See documentation on
    /// [`Config::epoch_interruption()`](crate::Config::epoch_interruption)
    /// for an introduction to epoch-based interruption.
    pub fn epoch_deadline_callback(
        &mut self,
        callback: impl FnMut(&mut T) -> Result<u64> + Send + Sync + 'static,
    ) {
        self.inner.epoch_deadline_callback(Box::new(callback));
    }

    #[cfg_attr(nightlydoc, doc(cfg(feature = "async")))]
    /// Configures epoch-deadline expiration to yield to the async
    /// caller and the update the deadline.
    ///
    /// When epoch-interruption-instrumented code is executed on this
    /// store and the epoch deadline is reached before completion,
    /// with the store configured in this way, execution will yield
    /// (the future will return `Pending` but re-awake itself for
    /// later execution) and, upon resuming, the store will be
    /// configured with an epoch deadline equal to the current epoch
    /// plus `delta` ticks.
    ///
    /// This setting is intended to allow for cooperative timeslicing
    /// of multiple CPU-bound Wasm guests in different stores, all
    /// executing under the control of an async executor. To drive
    /// this, stores should be configured to "yield and update"
    /// automatically with this function, and some external driver (a
    /// thread that wakes up periodically, or a timer
    /// signal/interrupt) should call
    /// [`Engine::increment_epoch()`](crate::Engine::increment_epoch).
    ///
    /// See documentation on
    /// [`Config::epoch_interruption()`](crate::Config::epoch_interruption)
    /// for an introduction to epoch-based interruption.
    #[cfg(feature = "async")]
    pub fn epoch_deadline_async_yield_and_update(&mut self, delta: u64) {
        self.inner.epoch_deadline_async_yield_and_update(delta);
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
    pub fn data(&self) -> &T {
        self.0.data()
    }

    /// Returns the fuel consumed by this store.
    ///
    /// For more information see [`Store::fuel_consumed`].
    pub fn fuel_consumed(&self) -> Option<u64> {
        self.0.fuel_consumed()
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
    pub fn gc(&mut self) {
        self.0.gc()
    }

    /// Returns the fuel consumed by this store.
    ///
    /// For more information see [`Store::fuel_consumed`].
    pub fn fuel_consumed(&self) -> Option<u64> {
        self.0.fuel_consumed()
    }

    /// Inject more fuel into this store to be consumed when executing wasm code.
    ///
    /// For more information see [`Store::add_fuel`]
    pub fn add_fuel(&mut self, fuel: u64) -> Result<()> {
        self.0.add_fuel(fuel)
    }

    /// Synthetically consume fuel from this store.
    ///
    /// For more information see [`Store::consume_fuel`]
    pub fn consume_fuel(&mut self, fuel: u64) -> Result<u64> {
        self.0.consume_fuel(fuel)
    }

    /// Configures this `Store` to trap whenever fuel runs out.
    ///
    /// For more information see [`Store::out_of_fuel_trap`]
    pub fn out_of_fuel_trap(&mut self) {
        self.0.out_of_fuel_trap()
    }

    /// Configures this `Store` to yield while executing futures whenever fuel
    /// runs out.
    ///
    /// For more information see [`Store::out_of_fuel_async_yield`]
    pub fn out_of_fuel_async_yield(&mut self, injection_count: u64, fuel_to_inject: u64) {
        self.0
            .out_of_fuel_async_yield(injection_count, fuel_to_inject)
    }

    /// Sets the epoch deadline to a certain number of ticks in the future.
    ///
    /// For more information see [`Store::set_epoch_deadline`].
    pub fn set_epoch_deadline(&mut self, ticks_beyond_current: u64) {
        self.0.set_epoch_deadline(ticks_beyond_current);
    }

    /// Configures epoch-deadline expiration to trap.
    ///
    /// For more information see [`Store::epoch_deadline_trap`].
    pub fn epoch_deadline_trap(&mut self) {
        self.0.epoch_deadline_trap();
    }

    #[cfg_attr(nightlydoc, doc(cfg(feature = "async")))]
    /// Configures epoch-deadline expiration to yield to the async
    /// caller and the update the deadline.
    ///
    /// For more information see
    /// [`Store::epoch_deadline_async_yield_and_update`].
    #[cfg(feature = "async")]
    pub fn epoch_deadline_async_yield_and_update(&mut self, delta: u64) {
        self.0.epoch_deadline_async_yield_and_update(delta);
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

    pub fn call_hook(&mut self, s: CallHook) -> Result<(), Trap> {
        match &mut self.call_hook {
            Some(CallHookInner::Sync(hook)) => hook(&mut self.data, s),

            #[cfg(feature = "async")]
            Some(CallHookInner::Async(handler)) => unsafe {
                Ok(self
                    .inner
                    .async_cx()
                    .ok_or(Trap::new("couldn't grab async_cx for call hook"))?
                    .block_on(handler.handle_call_event(&mut self.data, s).as_mut())??)
            },

            None => Ok(()),
        }
    }
}

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
        let memories = module.memory_plans.len() - module.num_imported_memories;
        let tables = module.table_plans.len() - module.num_imported_tables;

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
    pub(crate) fn modules_mut(&mut self) -> &mut ModuleRegistry {
        &mut self.modules
    }

    pub unsafe fn add_instance(&mut self, handle: InstanceHandle, ondemand: bool) -> InstanceId {
        self.instances.push(StoreInstance {
            handle: handle.clone(),
            ondemand,
        });
        InstanceId(self.instances.len() - 1)
    }

    pub fn instance(&self, id: InstanceId) -> &InstanceHandle {
        &self.instances[id.0].handle
    }

    pub fn instance_mut(&mut self, id: InstanceId) -> &mut InstanceHandle {
        &mut self.instances[id.0].handle
    }

    #[cfg_attr(not(target_os = "linux"), allow(dead_code))] // not used on all platforms
    pub fn set_signal_handler(&mut self, handler: Option<Box<SignalHandler<'static>>>) {
        self.signal_handler = handler;
    }

    #[inline]
    pub fn runtime_limits(&self) -> &VMRuntimeLimits {
        &self.runtime_limits
    }

    #[inline]
    pub fn externref_activations_table(&mut self) -> &mut VMExternRefActivationsTable {
        &mut self.externref_activations_table
    }

    pub fn gc(&mut self) {
        // For this crate's API, we ensure that `set_stack_canary` invariants
        // are upheld for all host-->Wasm calls.
        unsafe { wasmtime_runtime::gc(&self.modules, &mut self.externref_activations_table) }
    }

    /// Looks up the corresponding `VMTrampoline` which can be used to enter
    /// wasm given an anyfunc function pointer.
    ///
    /// This is a somewhat complicated implementation at this time, unfortnately.
    /// Trampolines are a sort of side-channel of information which is
    /// specifically juggled by the `wasmtime` crate in a careful fashion. The
    /// sources for trampolines are:
    ///
    /// * Compiled modules - each compiled module has a trampoline for all
    ///   signatures of functions that escape the module (e.g. exports and
    ///   `ref.func`-able functions)
    /// * `Func::new` - host-defined functions with a dynamic signature get an
    ///   on-the-fly-compiled trampoline (e.g. JIT-compiled as part of the
    ///   `Func::new` call).
    /// * `Func::wrap` - host-defined functions where the trampoline is
    ///   monomorphized in Rust and compiled by LLVM.
    ///
    /// The purpose of this function is that given some wasm function pointer we
    /// need to find the trampoline for it. For compiled wasm modules this is
    /// pretty easy, the code pointer of the function pointer will point us
    /// at a wasm module which has a table of trampolines-by-type that we can
    /// lookup.
    ///
    /// If this lookup fails, however, then we're trying to get the trampoline
    /// for a wasm function pointer defined by the host. The trampoline isn't
    /// actually stored in the wasm function pointer itself so we need
    /// side-channels of information. To achieve this a lazy scheme is
    /// implemented here based on the assumption that most trampoline lookups
    /// happen for wasm-defined functions, not host-defined functions.
    ///
    /// The `Store` already has a list of all functions in
    /// `self.store_data().funcs`, it's just not indexed in a nice fashion by
    /// type index or similar. To solve this there's an internal map in each
    /// store, `host_trampolines`, which maps from a type index to the
    /// store-owned trampoline. The actual population of this map, however, is
    /// deferred to this function itself.
    ///
    /// Most of the time we are looking up a Wasm function's trampoline when
    /// calling this function, and we don't want to make insertion of a host
    /// function into the store more expensive than it has to be. We could
    /// update the `host_trampolines` whenever a host function is inserted into
    /// the store, but this is a relatively expensive hash map insertion.
    /// Instead the work is deferred until we actually look up that trampoline
    /// in this method.
    ///
    /// This all means that if the lookup of the trampoline fails within
    /// `self.host_trampolines` we lazily populate `self.host_trampolines` by
    /// iterating over `self.store_data().funcs`, inserting trampolines as we
    /// go. If we find the right trampoline then it's returned.
    pub fn lookup_trampoline(&mut self, anyfunc: &VMCallerCheckedAnyfunc) -> VMTrampoline {
        // First try to see if the `anyfunc` belongs to any module. Each module
        // has its own map of trampolines-per-type-index and the code pointer in
        // the `anyfunc` will enable us to quickly find a module.
        if let Some(trampoline) = self.modules.lookup_trampoline(anyfunc) {
            return trampoline;
        }

        // Next consult the list of store-local host trampolines. This is
        // primarily populated by functions created by `Func::new` or similar
        // creation functions, host-defined functions.
        if let Some(trampoline) = self.host_trampolines.get(&anyfunc.type_index) {
            return *trampoline;
        }

        // If no trampoline was found then it means that it hasn't been loaded
        // into `host_trampolines` yet. Skip over all the ones we've looked at
        // so far and start inserting into `self.host_trampolines`, returning
        // the actual trampoline once found.
        for f in self
            .store_data
            .funcs()
            .skip(self.host_func_trampolines_registered)
        {
            self.host_func_trampolines_registered += 1;
            self.host_trampolines.insert(f.sig_index(), f.trampoline());
            if f.sig_index() == anyfunc.type_index {
                return f.trampoline();
            }
        }

        // If reached this is a bug in Wasmtime. Lookup of a trampoline should
        // only happen for wasm functions or host functions, all of which should
        // be indexed by the above.
        panic!("trampoline missing")
    }

    /// Yields the async context, assuming that we are executing on a fiber and
    /// that fiber is not in the process of dying. This function will return
    /// None in the latter case (the fiber is dying), and panic if
    /// `async_support()` is false.
    #[cfg(feature = "async")]
    #[inline]
    pub fn async_cx(&self) -> Option<AsyncCx> {
        debug_assert!(self.async_support());

        let poll_cx_box_ptr = self.async_state.current_poll_cx.get();
        if poll_cx_box_ptr.is_null() {
            return None;
        }

        let poll_cx_inner_ptr = unsafe { *poll_cx_box_ptr };
        if poll_cx_inner_ptr.is_null() {
            return None;
        }

        Some(AsyncCx {
            current_suspend: self.async_state.current_suspend.get(),
            current_poll_cx: poll_cx_box_ptr,
        })
    }

    pub fn fuel_consumed(&self) -> Option<u64> {
        if !self.engine.config().tunables.consume_fuel {
            return None;
        }
        let consumed = unsafe { *self.runtime_limits.fuel_consumed.get() };
        Some(u64::try_from(self.fuel_adj + consumed).unwrap())
    }

    fn out_of_fuel_trap(&mut self) {
        self.out_of_gas_behavior = OutOfGas::Trap;
    }

    fn out_of_fuel_async_yield(&mut self, injection_count: u64, fuel_to_inject: u64) {
        assert!(
            self.async_support(),
            "cannot use `out_of_fuel_async_yield` without enabling async support in the config"
        );
        self.out_of_gas_behavior = OutOfGas::InjectFuel {
            injection_count,
            fuel_to_inject,
        };
    }

    /// Yields execution to the caller on out-of-gas or epoch interruption.
    ///
    /// This only works on async futures and stores, and assumes that we're
    /// executing on a fiber. This will yield execution back to the caller once.
    #[cfg(feature = "async")]
    fn async_yield_impl(&mut self) -> Result<(), Trap> {
        // Small future that yields once and then returns ()
        #[derive(Default)]
        struct Yield {
            yielded: bool,
        }

        impl Future for Yield {
            type Output = ();

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
                if self.yielded {
                    Poll::Ready(())
                } else {
                    // Flag ourselves as yielded to return next time, and also
                    // flag the waker that we're already ready to get
                    // re-enqueued for another poll.
                    self.yielded = true;
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
        }

        let mut future = Yield::default();

        // When control returns, we have a `Result<(), Trap>` passed
        // in from the host fiber. If this finished successfully then
        // we were resumed normally via a `poll`, so keep going.  If
        // the future was dropped while we were yielded, then we need
        // to clean up this fiber. Do so by raising a trap which will
        // abort all wasm and get caught on the other side to clean
        // things up.
        unsafe {
            self.async_cx()
                .expect("attempted to pull async context during shutdown")
                .block_on(Pin::new_unchecked(&mut future))
        }
    }

    fn add_fuel(&mut self, fuel: u64) -> Result<()> {
        anyhow::ensure!(
            self.engine().config().tunables.consume_fuel,
            "fuel is not configured in this store"
        );

        // Fuel is stored as an i64, so we need to cast it. If the provided fuel
        // value overflows that just assume that i64::max will suffice. Wasm
        // execution isn't fast enough to burn through i64::max fuel in any
        // reasonable amount of time anyway.
        let fuel = i64::try_from(fuel).unwrap_or(i64::max_value());
        let adj = self.fuel_adj;
        let consumed_ptr = unsafe { &mut *self.runtime_limits.fuel_consumed.get() };

        match (consumed_ptr.checked_sub(fuel), adj.checked_add(fuel)) {
            // If we succesfully did arithmetic without overflowing then we can
            // just update our fields.
            (Some(consumed), Some(adj)) => {
                self.fuel_adj = adj;
                *consumed_ptr = consumed;
            }

            // Otherwise something overflowed. Make sure that we preserve the
            // amount of fuel that's already consumed, but otherwise assume that
            // we were given infinite fuel.
            _ => {
                self.fuel_adj = i64::max_value();
                *consumed_ptr = (*consumed_ptr + adj) - i64::max_value();
            }
        }

        Ok(())
    }

    fn consume_fuel(&mut self, fuel: u64) -> Result<u64> {
        let consumed_ptr = unsafe { &mut *self.runtime_limits.fuel_consumed.get() };
        match i64::try_from(fuel)
            .ok()
            .and_then(|fuel| consumed_ptr.checked_add(fuel))
        {
            Some(consumed) if consumed < 0 => {
                *consumed_ptr = consumed;
                Ok(u64::try_from(-consumed).unwrap())
            }
            _ => bail!("not enough fuel remaining in store"),
        }
    }

    #[inline]
    pub fn signal_handler(&self) -> Option<*const SignalHandler<'static>> {
        let handler = self.signal_handler.as_ref()?;
        Some(&**handler as *const _)
    }

    #[inline]
    pub fn vmruntime_limits(&self) -> *mut VMRuntimeLimits {
        &self.runtime_limits as *const VMRuntimeLimits as *mut VMRuntimeLimits
    }

    pub unsafe fn insert_vmexternref_without_gc(&mut self, r: VMExternRef) {
        self.externref_activations_table.insert_without_gc(r);
    }

    #[inline]
    pub fn default_callee(&self) -> *mut VMContext {
        self.default_callee.vmctx_ptr()
    }

    pub fn traitobj(&self) -> *mut dyn wasmtime_runtime::Store {
        self.default_callee.store()
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
}

impl<T> StoreContextMut<'_, T> {
    /// Executes a synchronous computation `func` asynchronously on a new fiber.
    ///
    /// This function will convert the synchronous `func` into an asynchronous
    /// future. This is done by running `func` in a fiber on a separate native
    /// stack which can be suspended and resumed from.
    ///
    /// Most of the nitty-gritty here is how we juggle the various contexts
    /// necessary to suspend the fiber later on and poll sub-futures. It's hoped
    /// that the various comments are illuminating as to what's going on here.
    #[cfg(feature = "async")]
    pub(crate) async fn on_fiber<R>(
        &mut self,
        func: impl FnOnce(&mut StoreContextMut<'_, T>) -> R + Send,
    ) -> Result<R, Trap>
    where
        T: Send,
    {
        let config = self.engine().config();
        debug_assert!(self.0.async_support());
        debug_assert!(config.async_stack_size > 0);

        let mut slot = None;
        let future = {
            let current_poll_cx = self.0.async_state.current_poll_cx.get();
            let current_suspend = self.0.async_state.current_suspend.get();
            let stack = self
                .engine()
                .allocator()
                .allocate_fiber_stack()
                .map_err(|e| Trap::from(anyhow::Error::from(e)))?;

            let engine = self.engine().clone();
            let slot = &mut slot;
            let fiber = wasmtime_fiber::Fiber::new(stack, move |keep_going, suspend| {
                // First check and see if we were interrupted/dropped, and only
                // continue if we haven't been.
                keep_going?;

                // Configure our store's suspension context for the rest of the
                // execution of this fiber. Note that a raw pointer is stored here
                // which is only valid for the duration of this closure.
                // Consequently we at least replace it with the previous value when
                // we're done. This reset is also required for correctness because
                // otherwise our value will overwrite another active fiber's value.
                // There should be a test that segfaults in `async_functions.rs` if
                // this `Replace` is removed.
                unsafe {
                    let _reset = Reset(current_suspend, *current_suspend);
                    *current_suspend = suspend;

                    *slot = Some(func(self));
                    Ok(())
                }
            })
            .map_err(|e| Trap::from(anyhow::Error::from(e)))?;

            // Once we have the fiber representing our synchronous computation, we
            // wrap that in a custom future implementation which does the
            // translation from the future protocol to our fiber API.
            FiberFuture {
                fiber,
                current_poll_cx,
                engine,
            }
        };
        future.await?;

        return Ok(slot.unwrap());

        struct FiberFuture<'a> {
            fiber: wasmtime_fiber::Fiber<'a, Result<(), Trap>, (), Result<(), Trap>>,
            current_poll_cx: *mut *mut Context<'static>,
            engine: Engine,
        }

        // This is surely the most dangerous `unsafe impl Send` in the entire
        // crate. There are two members in `FiberFuture` which cause it to not
        // be `Send`. One is `current_poll_cx` and is entirely uninteresting.
        // This is just used to manage `Context` pointers across `await` points
        // in the future, and requires raw pointers to get it to happen easily.
        // Nothing too weird about the `Send`-ness, values aren't actually
        // crossing threads.
        //
        // The really interesting piece is `fiber`. Now the "fiber" here is
        // actual honest-to-god Rust code which we're moving around. What we're
        // doing is the equivalent of moving our thread's stack to another OS
        // thread. Turns out we, in general, have no idea what's on the stack
        // and would generally have no way to verify that this is actually safe
        // to do!
        //
        // Thankfully, though, Wasmtime has the power. Without being glib it's
        // actually worth examining what's on the stack. It's unfortunately not
        // super-local to this function itself. Our closure to `Fiber::new` runs
        // `func`, which is given to us from the outside. Thankfully, though, we
        // have tight control over this. Usage of `on_fiber` is typically done
        // *just* before entering WebAssembly itself, so we'll have a few stack
        // frames of Rust code (all in Wasmtime itself) before we enter wasm.
        //
        // Once we've entered wasm, well then we have a whole bunch of wasm
        // frames on the stack. We've got this nifty thing called Cranelift,
        // though, which allows us to also have complete control over everything
        // on the stack!
        //
        // Finally, when wasm switches back to the fiber's starting pointer
        // (this future we're returning) then it means wasm has reentered Rust.
        // Suspension can only happen via the `block_on` function of an
        // `AsyncCx`. This, conveniently, also happens entirely in Wasmtime
        // controlled code!
        //
        // There's an extremely important point that should be called out here.
        // User-provided futures **are not on the stack** during suspension
        // points. This is extremely crucial because we in general cannot reason
        // about Send/Sync for stack-local variables since rustc doesn't analyze
        // them at all. With our construction, though, we are guaranteed that
        // Wasmtime owns all stack frames between the stack of a fiber and when
        // the fiber suspends (and it could move across threads). At this time
        // the only user-provided piece of data on the stack is the future
        // itself given to us. Lo-and-behold as you might notice the future is
        // required to be `Send`!
        //
        // What this all boils down to is that we, as the authors of Wasmtime,
        // need to be extremely careful that on the async fiber stack we only
        // store Send things. For example we can't start using `Rc` willy nilly
        // by accident and leave a copy in TLS somewhere. (similarly we have to
        // be ready for TLS to change while we're executing wasm code between
        // suspension points).
        //
        // While somewhat onerous it shouldn't be too too hard (the TLS bit is
        // the hardest bit so far). This does mean, though, that no user should
        // ever have to worry about the `Send`-ness of Wasmtime. If rustc says
        // it's ok, then it's ok.
        //
        // With all that in mind we unsafely assert here that wasmtime is
        // correct. We declare the fiber as only containing Send data on its
        // stack, despite not knowing for sure at compile time that this is
        // correct. That's what `unsafe` in Rust is all about, though, right?
        unsafe impl Send for FiberFuture<'_> {}

        impl Future for FiberFuture<'_> {
            type Output = Result<(), Trap>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
                // We need to carry over this `cx` into our fiber's runtime
                // for when it tries to poll sub-futures that are created. Doing
                // this must be done unsafely, however, since `cx` is only alive
                // for this one singular function call. Here we do a `transmute`
                // to extend the lifetime of `Context` so it can be stored in
                // our `Store`, and then we replace the current polling context
                // with this one.
                //
                // Note that the replace is done for weird situations where
                // futures might be switching contexts and there's multiple
                // wasmtime futures in a chain of futures.
                //
                // On exit from this function, though, we reset the polling
                // context back to what it was to signify that `Store` no longer
                // has access to this pointer.
                unsafe {
                    let _reset = Reset(self.current_poll_cx, *self.current_poll_cx);
                    *self.current_poll_cx =
                        std::mem::transmute::<&mut Context<'_>, *mut Context<'static>>(cx);

                    // After that's set up we resume execution of the fiber, which
                    // may also start the fiber for the first time. This either
                    // returns `Ok` saying the fiber finished (yay!) or it returns
                    // `Err` with the payload passed to `suspend`, which in our case
                    // is `()`. If `Err` is returned that means the fiber polled a
                    // future but it said "Pending", so we propagate that here.
                    match self.fiber.resume(Ok(())) {
                        Ok(result) => Poll::Ready(result),
                        Err(()) => Poll::Pending,
                    }
                }
            }
        }

        // Dropping futures is pretty special in that it means the future has
        // been requested to be cancelled. Here we run the risk of dropping an
        // in-progress fiber, and if we were to do nothing then the fiber would
        // leak all its owned stack resources.
        //
        // To handle this we implement `Drop` here and, if the fiber isn't done,
        // resume execution of the fiber saying "hey please stop you're
        // interrupted". Our `Trap` created here (which has the stack trace
        // of whomever dropped us) will then get propagated in whatever called
        // `block_on`, and the idea is that the trap propagates all the way back
        // up to the original fiber start, finishing execution.
        //
        // We don't actually care about the fiber's return value here (no one's
        // around to look at it), we just assert the fiber finished to
        // completion.
        impl Drop for FiberFuture<'_> {
            fn drop(&mut self) {
                if !self.fiber.done() {
                    let result = self.fiber.resume(Err(Trap::new("future dropped")));
                    // This resumption with an error should always complete the
                    // fiber. While it's technically possible for host code to catch
                    // the trap and re-resume, we'd ideally like to signal that to
                    // callers that they shouldn't be doing that.
                    debug_assert!(result.is_ok());
                }

                unsafe {
                    self.engine
                        .allocator()
                        .deallocate_fiber_stack(self.fiber.stack());
                }
            }
        }
    }
}

#[cfg(feature = "async")]
pub struct AsyncCx {
    current_suspend: *mut *const wasmtime_fiber::Suspend<Result<(), Trap>, (), Result<(), Trap>>,
    current_poll_cx: *mut *mut Context<'static>,
}

#[cfg(feature = "async")]
impl AsyncCx {
    /// Blocks on the asynchronous computation represented by `future` and
    /// produces the result here, in-line.
    ///
    /// This function is designed to only work when it's currently executing on
    /// a native fiber. This fiber provides the ability for us to handle the
    /// future's `Pending` state as "jump back to whomever called the fiber in
    /// an asynchronous fashion and propagate `Pending`". This tight coupling
    /// with `on_fiber` below is what powers the asynchronicity of calling wasm.
    /// Note that the asynchronous part only applies to host functions, wasm
    /// itself never really does anything asynchronous at this time.
    ///
    /// This function takes a `future` and will (appear to) synchronously wait
    /// on the result. While this function is executing it will fiber switch
    /// to-and-from the original frame calling `on_fiber` which should be a
    /// guarantee due to how async stores are configured.
    ///
    /// The return value here is either the output of the future `T`, or a trap
    /// which represents that the asynchronous computation was cancelled. It is
    /// not recommended to catch the trap and try to keep executing wasm, so
    /// we've tried to liberally document this.
    pub unsafe fn block_on<U>(
        &self,
        mut future: Pin<&mut (dyn Future<Output = U> + Send)>,
    ) -> Result<U, Trap> {
        // Take our current `Suspend` context which was configured as soon as
        // our fiber started. Note that we must load it at the front here and
        // save it on our stack frame. While we're polling the future other
        // fibers may be started for recursive computations, and the current
        // suspend context is only preserved at the edges of the fiber, not
        // during the fiber itself.
        //
        // For a little bit of extra safety we also replace the current value
        // with null to try to catch any accidental bugs on our part early.
        // This is all pretty unsafe so we're trying to be careful...
        //
        // Note that there should be a segfaulting test  in `async_functions.rs`
        // if this `Reset` is removed.
        let suspend = *self.current_suspend;
        let _reset = Reset(self.current_suspend, suspend);
        *self.current_suspend = ptr::null();
        assert!(!suspend.is_null());

        loop {
            let future_result = {
                let poll_cx = *self.current_poll_cx;
                let _reset = Reset(self.current_poll_cx, poll_cx);
                *self.current_poll_cx = ptr::null_mut();
                assert!(!poll_cx.is_null());
                future.as_mut().poll(&mut *poll_cx)
            };

            match future_result {
                Poll::Ready(t) => break Ok(t),
                Poll::Pending => {}
            }

            let before = wasmtime_runtime::TlsRestore::take().map_err(Trap::from_runtime_box)?;
            let res = (*suspend).suspend(());
            before.replace().map_err(Trap::from_runtime_box)?;
            res?;
        }
    }
}

unsafe impl<T> wasmtime_runtime::Store for StoreInner<T> {
    fn vmruntime_limits(&self) -> *mut VMRuntimeLimits {
        <StoreOpaque>::vmruntime_limits(self)
    }

    fn epoch_ptr(&self) -> *const AtomicU64 {
        self.engine.epoch_counter() as *const _
    }

    fn externref_activations_table(
        &mut self,
    ) -> (
        &mut VMExternRefActivationsTable,
        &dyn wasmtime_runtime::ModuleInfoLookup,
    ) {
        let inner = &mut self.inner;
        (&mut inner.externref_activations_table, &inner.modules)
    }

    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> Result<bool, anyhow::Error> {
        match self.limiter {
            Some(ResourceLimiterInner::Sync(ref mut limiter)) => {
                Ok(limiter(&mut self.data).memory_growing(current, desired, maximum))
            }
            #[cfg(feature = "async")]
            Some(ResourceLimiterInner::Async(ref mut limiter)) => unsafe {
                Ok(self
                    .inner
                    .async_cx()
                    .expect("ResourceLimiterAsync requires async Store")
                    .block_on(
                        limiter(&mut self.data)
                            .memory_growing(current, desired, maximum)
                            .as_mut(),
                    )?)
            },
            None => Ok(true),
        }
    }

    fn memory_grow_failed(&mut self, error: &anyhow::Error) {
        match self.limiter {
            Some(ResourceLimiterInner::Sync(ref mut limiter)) => {
                limiter(&mut self.data).memory_grow_failed(error)
            }
            #[cfg(feature = "async")]
            Some(ResourceLimiterInner::Async(ref mut limiter)) => {
                limiter(&mut self.data).memory_grow_failed(error)
            }
            None => {}
        }
    }

    fn table_growing(
        &mut self,
        current: u32,
        desired: u32,
        maximum: Option<u32>,
    ) -> Result<bool, anyhow::Error> {
        // Need to borrow async_cx before the mut borrow of the limiter.
        // self.async_cx() panicks when used with a non-async store, so
        // wrap this in an option.
        #[cfg(feature = "async")]
        let async_cx = if self.async_support() {
            Some(self.async_cx().unwrap())
        } else {
            None
        };

        match self.limiter {
            Some(ResourceLimiterInner::Sync(ref mut limiter)) => {
                Ok(limiter(&mut self.data).table_growing(current, desired, maximum))
            }
            #[cfg(feature = "async")]
            Some(ResourceLimiterInner::Async(ref mut limiter)) => unsafe {
                Ok(async_cx
                    .expect("ResourceLimiterAsync requires async Store")
                    .block_on(
                        limiter(&mut self.data)
                            .table_growing(current, desired, maximum)
                            .as_mut(),
                    )?)
            },
            None => Ok(true),
        }
    }

    fn table_grow_failed(&mut self, error: &anyhow::Error) {
        match self.limiter {
            Some(ResourceLimiterInner::Sync(ref mut limiter)) => {
                limiter(&mut self.data).table_grow_failed(error)
            }
            #[cfg(feature = "async")]
            Some(ResourceLimiterInner::Async(ref mut limiter)) => {
                limiter(&mut self.data).table_grow_failed(error)
            }
            None => {}
        }
    }

    fn out_of_gas(&mut self) -> Result<(), anyhow::Error> {
        return match &mut self.out_of_gas_behavior {
            OutOfGas::Trap => Err(anyhow::Error::new(OutOfGasError)),
            #[cfg(feature = "async")]
            OutOfGas::InjectFuel {
                injection_count,
                fuel_to_inject,
            } => {
                if *injection_count == 0 {
                    return Err(anyhow::Error::new(OutOfGasError));
                }
                *injection_count -= 1;
                let fuel = *fuel_to_inject;
                self.async_yield_impl()?;
                if fuel > 0 {
                    self.add_fuel(fuel).unwrap();
                }
                Ok(())
            }
            #[cfg(not(feature = "async"))]
            OutOfGas::InjectFuel { .. } => unreachable!(),
        };

        #[derive(Debug)]
        struct OutOfGasError;

        impl fmt::Display for OutOfGasError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("all fuel consumed by WebAssembly")
            }
        }

        impl std::error::Error for OutOfGasError {}
    }

    fn new_epoch(&mut self) -> Result<u64, anyhow::Error> {
        return match &mut self.epoch_deadline_behavior {
            EpochDeadline::Trap => {
                let trap = Trap::new_wasm(wasmtime_environ::TrapCode::Interrupt, None);
                Err(anyhow::Error::from(trap))
            }
            EpochDeadline::Callback(callback) => {
                let delta = callback(&mut self.data)?;
                // Set a new deadline and return the new epoch deadline so
                // the Wasm code doesn't have to reload it.
                self.set_epoch_deadline(delta);
                Ok(self.get_epoch_deadline())
            }
            #[cfg(feature = "async")]
            EpochDeadline::YieldAndExtendDeadline { delta } => {
                let delta = *delta;
                // Do the async yield. May return a trap if future was
                // canceled while we're yielded.
                self.async_yield_impl()?;
                // Set a new deadline.
                self.set_epoch_deadline(delta);

                // Return the new epoch deadline so the Wasm code
                // doesn't have to reload it.
                Ok(self.get_epoch_deadline())
            }
        };
    }
}

impl<T> StoreInner<T> {
    pub(crate) fn set_epoch_deadline(&mut self, delta: u64) {
        // Set a new deadline based on the "epoch deadline delta".
        //
        // Safety: this is safe because the epoch deadline in the
        // `VMRuntimeLimits` is accessed only here and by Wasm guest code
        // running in this store, and we have a `&mut self` here.
        //
        // Also, note that when this update is performed while Wasm is
        // on the stack, the Wasm will reload the new value once we
        // return into it.
        let epoch_deadline = unsafe { (*self.vmruntime_limits()).epoch_deadline.get_mut() };
        *epoch_deadline = self.engine().current_epoch() + delta;
    }

    fn epoch_deadline_trap(&mut self) {
        self.epoch_deadline_behavior = EpochDeadline::Trap;
    }

    fn epoch_deadline_callback(
        &mut self,
        callback: Box<dyn FnMut(&mut T) -> Result<u64> + Send + Sync>,
    ) {
        self.epoch_deadline_behavior = EpochDeadline::Callback(callback);
    }

    fn epoch_deadline_async_yield_and_update(&mut self, delta: u64) {
        assert!(
            self.async_support(),
            "cannot use `epoch_deadline_async_yield_and_update` without enabling async support in the config"
        );
        #[cfg(feature = "async")]
        {
            self.epoch_deadline_behavior = EpochDeadline::YieldAndExtendDeadline { delta };
        }
        drop(delta); // suppress warning in non-async build
    }

    fn get_epoch_deadline(&self) -> u64 {
        // Safety: this is safe because, as above, it is only invoked
        // from within `new_epoch` which is called from guest Wasm
        // code, which will have an exclusive borrow on the Store.
        let epoch_deadline = unsafe { (*self.vmruntime_limits()).epoch_deadline.get_mut() };
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
            for instance in self.instances.iter() {
                if instance.ondemand {
                    ondemand.deallocate(&instance.handle);
                } else {
                    allocator.deallocate(&instance.handle);
                }
            }
            ondemand.deallocate(&self.default_callee);

            // See documentation for these fields on `StoreOpaque` for why they
            // must be dropped in this order.
            ManuallyDrop::drop(&mut self.store_data);
            ManuallyDrop::drop(&mut self.rooted_host_funcs);
        }
    }
}

impl wasmtime_runtime::ModuleInfoLookup for ModuleRegistry {
    fn lookup(&self, pc: usize) -> Option<&dyn ModuleInfo> {
        self.lookup_module(pc)
    }
}

struct Reset<T: Copy>(*mut T, T);

impl<T: Copy> Drop for Reset<T> {
    fn drop(&mut self) {
        unsafe {
            *self.0 = self.1;
        }
    }
}
