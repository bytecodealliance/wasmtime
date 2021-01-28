use crate::frame_info::StoreFrameInfo;
use crate::sig_registry::SignatureRegistry;
use crate::trampoline::StoreInstanceHandle;
use crate::{Engine, Module};
use anyhow::{bail, Result};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::convert::TryFrom;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::{Rc, Weak};
use std::sync::Arc;
use wasmtime_environ::wasm;
use wasmtime_jit::{CompiledModule, ModuleCode, TypeTables};
use wasmtime_runtime::{
    InstanceHandle, RuntimeMemoryCreator, SignalHandler, StackMapRegistry, TrapInfo, VMContext,
    VMExternRef, VMExternRefActivationsTable, VMInterrupts, VMSharedSignatureIndex,
};

/// A `Store` is a collection of WebAssembly instances and host-defined items.
///
/// All WebAssembly instances and items will be attached to and refer to a
/// `Store`. For example instances, functions, globals, and tables are all
/// attached to a `Store`. Instances are created by instantiating a
/// [`Module`](crate::Module) within a `Store`.
///
/// `Store` is not thread-safe and cannot be sent to other threads. All items
/// which refer to a `Store` additionally are not threadsafe and can only be
/// used on the original thread that they were created on.
///
/// A `Store` is not intended to be a long-lived object in a program. No form of
/// GC is implemented at this time so once an instance is created within a
/// `Store` it will not be deallocated until all references to the `Store` have
/// gone away (this includes all references to items in the store). This makes
/// `Store` unsuitable for creating an unbounded number of instances in it
/// because `Store` will never release this memory. It's instead recommended to
/// have a long-lived [`Engine`] and instead create a `Store` for a more scoped
/// portion of your application.
///
/// # Stores and `Clone`
///
/// Using `clone` on a `Store` is a cheap operation. It will not create an
/// entirely new store, but rather just a new reference to the existing object.
/// In other words it's a shallow copy, not a deep copy.
///
/// ## Stores and `Default`
///
/// You can create a store with default configuration settings using
/// `Store::default()`. This will create a brand new [`Engine`] with default
/// ocnfiguration (see [`Config`](crate::Config) for more information).
#[derive(Clone)]
pub struct Store {
    inner: Rc<StoreInner>,
}

pub(crate) struct StoreInner {
    engine: Engine,
    interrupts: Arc<VMInterrupts>,
    signatures: RefCell<SignatureRegistry>,
    instances: RefCell<Vec<InstanceHandle>>,
    signal_handler: RefCell<Option<Box<SignalHandler<'static>>>>,
    externref_activations_table: VMExternRefActivationsTable,
    stack_map_registry: StackMapRegistry,
    /// Information about JIT code which allows us to test if a program counter
    /// is in JIT code, lookup trap information, etc.
    frame_info: RefCell<StoreFrameInfo>,
    /// Set of all compiled modules that we're holding a strong reference to
    /// the module's code for. This includes JIT functions, trampolines, etc.
    modules: RefCell<HashSet<ArcModuleCode>>,

    // Numbers of resources instantiated in this store.
    instance_count: Cell<usize>,
    memory_count: Cell<usize>,
    table_count: Cell<usize>,

    /// An adjustment to add to the fuel consumed value in `interrupts` above
    /// to get the true amount of fuel consumed.
    fuel_adj: Cell<i64>,
}

struct HostInfoKey(VMExternRef);

impl PartialEq for HostInfoKey {
    fn eq(&self, rhs: &Self) -> bool {
        VMExternRef::eq(&self.0, &rhs.0)
    }
}

impl Eq for HostInfoKey {}

impl Hash for HostInfoKey {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        VMExternRef::hash(&self.0, hasher);
    }
}

impl Store {
    /// Creates a new store to be associated with the given [`Engine`].
    pub fn new(engine: &Engine) -> Store {
        // Ensure that wasmtime_runtime's signal handlers are configured. Note
        // that at the `Store` level it means we should perform this
        // once-per-thread. Platforms like Unix, however, only require this
        // once-per-program. In any case this is safe to call many times and
        // each one that's not relevant just won't do anything.
        wasmtime_runtime::init_traps();

        Store {
            inner: Rc::new(StoreInner {
                engine: engine.clone(),
                interrupts: Arc::new(Default::default()),
                signatures: RefCell::new(Default::default()),
                instances: RefCell::new(Vec::new()),
                signal_handler: RefCell::new(None),
                externref_activations_table: VMExternRefActivationsTable::new(),
                stack_map_registry: StackMapRegistry::default(),
                frame_info: Default::default(),
                modules: Default::default(),
                instance_count: Default::default(),
                memory_count: Default::default(),
                table_count: Default::default(),
                fuel_adj: Cell::new(0),
            }),
        }
    }

    pub(crate) fn from_inner(inner: Rc<StoreInner>) -> Store {
        Store { inner }
    }

    /// Returns the [`Engine`] that this store is associated with.
    pub fn engine(&self) -> &Engine {
        &self.inner.engine
    }

    /// Returns an optional reference to a ['RuntimeMemoryCreator']
    pub(crate) fn memory_creator(&self) -> Option<&dyn RuntimeMemoryCreator> {
        self.engine()
            .config()
            .memory_creator
            .as_ref()
            .map(|x| x as _)
    }

    pub(crate) fn signatures(&self) -> &RefCell<SignatureRegistry> {
        &self.inner.signatures
    }

    pub(crate) fn lookup_shared_signature<'a>(
        &'a self,
        types: &'a TypeTables,
    ) -> impl Fn(wasm::SignatureIndex) -> VMSharedSignatureIndex + 'a {
        move |index| {
            self.signatures()
                .borrow()
                .lookup(&types.wasm_signatures[index])
                .expect("signature not previously registered")
        }
    }

    pub(crate) fn register_module(&self, module: &Module) {
        // All modules register their JIT code in a store for two reasons
        // currently:
        //
        // * First we only catch signals/traps if the program counter falls
        //   within the jit code of an instantiated wasm module. This ensures
        //   we don't catch accidental Rust/host segfaults.
        //
        // * Second when generating a backtrace we'll use this mapping to
        //   only generate wasm frames for instruction pointers that fall
        //   within jit code.
        self.register_jit_code(module.compiled_module());

        // We need to know about all the stack maps of all instantiated modules
        // so when performing a GC we know about all wasm frames that we find
        // on the stack.
        self.register_stack_maps(module.compiled_module());

        // Signatures are loaded into our `SignatureRegistry` here
        // once-per-module (and once-per-signature). This allows us to create
        // a `Func` wrapper for any function in the module, which requires that
        // we know about the signature and trampoline for all instances.
        self.register_signatures(module);

        // And finally with a module being instantiated into this `Store` we
        // need to preserve its jit-code. References to this module's code and
        // trampolines are not owning-references so it's our responsibility to
        // keep it all alive within the `Store`.
        self.inner
            .modules
            .borrow_mut()
            .insert(ArcModuleCode(module.compiled_module().code().clone()));
    }

    fn register_jit_code(&self, module: &CompiledModule) {
        let functions = module.finished_functions();
        let first_pc = match functions.values().next() {
            Some(f) => unsafe { (**f).as_ptr() as usize },
            None => return,
        };
        // Only register this module if it hasn't already been registered.
        let mut info = self.inner.frame_info.borrow_mut();
        if !info.contains_pc(first_pc) {
            info.register(module);
        }
    }

    fn register_stack_maps(&self, module: &CompiledModule) {
        self.stack_map_registry()
            .register_stack_maps(module.stack_maps().map(|(func, stack_maps)| unsafe {
                let ptr = (*func).as_ptr();
                let len = (*func).len();
                let start = ptr as usize;
                let end = ptr as usize + len;
                let range = start..end;
                (range, stack_maps)
            }));
    }

    fn register_signatures(&self, module: &Module) {
        let trampolines = module.compiled_module().trampolines();
        let mut signatures = self.signatures().borrow_mut();
        for (index, wasm) in module.types().wasm_signatures.iter() {
            signatures.register(wasm, trampolines[index]);
        }
    }

    pub(crate) fn bump_resource_counts(&self, module: &Module) -> Result<()> {
        let config = self.engine().config();

        fn bump(slot: &Cell<usize>, max: usize, amt: usize, desc: &str) -> Result<()> {
            let new = slot.get().saturating_add(amt);
            if new > max {
                bail!(
                    "resource limit exceeded: {} count too high at {}",
                    desc,
                    new
                );
            }
            slot.set(new);
            Ok(())
        }

        let module = module.env_module();
        let memories = module.memory_plans.len() - module.num_imported_memories;
        let tables = module.table_plans.len() - module.num_imported_tables;

        bump(
            &self.inner.instance_count,
            config.max_instances,
            1,
            "instance",
        )?;
        bump(
            &self.inner.memory_count,
            config.max_memories,
            memories,
            "memory",
        )?;
        bump(&self.inner.table_count, config.max_tables, tables, "table")?;

        Ok(())
    }

    pub(crate) unsafe fn add_instance(&self, handle: InstanceHandle) -> StoreInstanceHandle {
        self.inner.instances.borrow_mut().push(handle.clone());
        StoreInstanceHandle {
            store: self.clone(),
            handle,
        }
    }

    pub(crate) fn existing_instance_handle(&self, handle: InstanceHandle) -> StoreInstanceHandle {
        debug_assert!(self
            .inner
            .instances
            .borrow()
            .iter()
            .any(|i| i.vmctx_ptr() == handle.vmctx_ptr()));
        StoreInstanceHandle {
            store: self.clone(),
            handle,
        }
    }

    pub(crate) unsafe fn existing_vmctx(&self, cx: *mut VMContext) -> StoreInstanceHandle {
        self.existing_instance_handle(InstanceHandle::from_vmctx(cx))
    }

    pub(crate) fn weak(&self) -> Weak<StoreInner> {
        Rc::downgrade(&self.inner)
    }

    pub(crate) fn upgrade(weak: &Weak<StoreInner>) -> Option<Self> {
        let inner = weak.upgrade()?;
        Some(Self { inner })
    }

    pub(crate) fn set_signal_handler(&self, handler: Option<Box<SignalHandler<'static>>>) {
        *self.inner.signal_handler.borrow_mut() = handler;
    }

    pub(crate) fn interrupts(&self) -> &VMInterrupts {
        &self.inner.interrupts
    }

    /// Returns whether the stores `a` and `b` refer to the same underlying
    /// `Store`.
    ///
    /// Because the `Store` type is reference counted multiple clones may point
    /// to the same underlying storage, and this method can be used to determine
    /// whether two stores are indeed the same.
    pub fn same(a: &Store, b: &Store) -> bool {
        Rc::ptr_eq(&a.inner, &b.inner)
    }

    /// Creates an [`InterruptHandle`] which can be used to interrupt the
    /// execution of instances within this `Store`.
    ///
    /// An [`InterruptHandle`] handle is a mechanism of ensuring that guest code
    /// doesn't execute for too long. For example it's used to prevent wasm
    /// programs for executing infinitely in infinite loops or recursive call
    /// chains.
    ///
    /// The [`InterruptHandle`] type is sendable to other threads so you can
    /// interact with it even while the thread with this `Store` is executing
    /// wasm code.
    ///
    /// There's one method on an interrupt handle:
    /// [`InterruptHandle::interrupt`]. This method is used to generate an
    /// interrupt and cause wasm code to exit "soon".
    ///
    /// ## When are interrupts delivered?
    ///
    /// The term "interrupt" here refers to one of two different behaviors that
    /// are interrupted in wasm:
    ///
    /// * The head of every loop in wasm has a check to see if it's interrupted.
    /// * The prologue of every function has a check to see if it's interrupted.
    ///
    /// This interrupt mechanism makes no attempt to signal interrupts to
    /// native code. For example if a host function is blocked, then sending
    /// an interrupt will not interrupt that operation.
    ///
    /// Interrupts are consumed as soon as possible when wasm itself starts
    /// executing. This means that if you interrupt wasm code then it basically
    /// guarantees that the next time wasm is executing on the target thread it
    /// will return quickly (either normally if it were already in the process
    /// of returning or with a trap from the interrupt). Once an interrupt
    /// trap is generated then an interrupt is consumed, and further execution
    /// will not be interrupted (unless another interrupt is set).
    ///
    /// When implementing interrupts you'll want to ensure that the delivery of
    /// interrupts into wasm code is also handled in your host imports and
    /// functionality. Host functions need to either execute for bounded amounts
    /// of time or you'll need to arrange for them to be interrupted as well.
    ///
    /// ## Return Value
    ///
    /// This function returns a `Result` since interrupts are not always
    /// enabled. Interrupts are enabled via the
    /// [`Config::interruptable`](crate::Config::interruptable) method, and if
    /// this store's [`Config`](crate::Config) hasn't been configured to enable
    /// interrupts then an error is returned.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use anyhow::Result;
    /// # use wasmtime::*;
    /// # fn main() -> Result<()> {
    /// // Enable interruptable code via `Config` and then create an interrupt
    /// // handle which we'll use later to interrupt running code.
    /// let engine = Engine::new(Config::new().interruptable(true));
    /// let store = Store::new(&engine);
    /// let interrupt_handle = store.interrupt_handle()?;
    ///
    /// // Compile and instantiate a small example with an infinite loop.
    /// let module = Module::new(&engine, r#"
    ///     (func (export "run") (loop br 0))
    /// "#)?;
    /// let instance = Instance::new(&store, &module, &[])?;
    /// let run = instance
    ///     .get_func("run")
    ///     .ok_or(anyhow::format_err!("failed to find `run` function export"))?
    ///     .get0::<()>()?;
    ///
    /// // Spin up a thread to send us an interrupt in a second
    /// std::thread::spawn(move || {
    ///     std::thread::sleep(std::time::Duration::from_secs(1));
    ///     interrupt_handle.interrupt();
    /// });
    ///
    /// let trap = run().unwrap_err();
    /// assert!(trap.to_string().contains("wasm trap: interrupt"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn interrupt_handle(&self) -> Result<InterruptHandle> {
        if self.engine().config().tunables.interruptable {
            Ok(InterruptHandle {
                interrupts: self.inner.interrupts.clone(),
            })
        } else {
            bail!("interrupts aren't enabled for this `Store`")
        }
    }

    pub(crate) fn externref_activations_table(&self) -> &VMExternRefActivationsTable {
        &self.inner.externref_activations_table
    }

    pub(crate) fn stack_map_registry(&self) -> &StackMapRegistry {
        &self.inner.stack_map_registry
    }

    pub(crate) fn frame_info(&self) -> &RefCell<StoreFrameInfo> {
        &self.inner.frame_info
    }

    /// Perform garbage collection of `ExternRef`s.
    pub fn gc(&self) {
        // For this crate's API, we ensure that `set_stack_canary` invariants
        // are upheld for all host-->Wasm calls, and we register every module
        // used with this store in `self.inner.stack_map_registry`.
        unsafe {
            wasmtime_runtime::gc(
                &self.inner.stack_map_registry,
                &self.inner.externref_activations_table,
            );
        }
    }

    /// Returns the amount of fuel consumed by this store's execution so far.
    ///
    /// If fuel consumption is not enabled via
    /// [`Config::consume_fuel`](crate::Config::consume_fuel) then this
    /// function will return `None`. Also note that fuel, if enabled, must be
    /// originally configured via [`Store::add_fuel`].
    pub fn fuel_consumed(&self) -> Option<u64> {
        if !self.engine().config().tunables.consume_fuel {
            return None;
        }
        let consumed = unsafe { *self.inner.interrupts.fuel_consumed.get() };
        Some(u64::try_from(self.inner.fuel_adj.get() + consumed).unwrap())
    }

    /// Adds fuel to this [`Store`] for wasm to consume while executing.
    ///
    /// For this method to work fuel consumption must be enabled via
    /// [`Config::consume_fuel`](crate::Config::consume_fuel). By default a
    /// [`Store`] starts with 0 fuel for wasm to execute with (meaning it will
    /// immediately trap). This function must be called for the store to have
    /// some fuel to allow WebAssembly to execute.
    ///
    /// Note that at this time when fuel is entirely consumed it will cause
    /// wasm to trap. More usages of fuel are planned for the future.
    ///
    /// # Panics
    ///
    /// This function will panic if the store's [`Config`](crate::Config) did
    /// not have fuel consumption enabled.
    pub fn add_fuel(&self, fuel: u64) {
        assert!(self.engine().config().tunables.consume_fuel);

        // Fuel is stored as an i64, so we need to cast it. If the provided fuel
        // value overflows that just assume that i64::max will suffice. Wasm
        // execution isn't fast enough to burn through i64::max fuel in any
        // reasonable amount of time anyway.
        let fuel = i64::try_from(fuel).unwrap_or(i64::max_value());
        let adj = self.inner.fuel_adj.get();
        let consumed_ptr = unsafe { &mut *self.inner.interrupts.fuel_consumed.get() };

        match (consumed_ptr.checked_sub(fuel), adj.checked_add(fuel)) {
            // If we succesfully did arithmetic without overflowing then we can
            // just update our fields.
            (Some(consumed), Some(adj)) => {
                self.inner.fuel_adj.set(adj);
                *consumed_ptr = consumed;
            }

            // Otherwise something overflowed. Make sure that we preserve the
            // amount of fuel that's already consumed, but otherwise assume that
            // we were given infinite fuel.
            _ => {
                self.inner.fuel_adj.set(i64::max_value());
                *consumed_ptr = (*consumed_ptr + adj) - i64::max_value();
            }
        }
    }
}

unsafe impl TrapInfo for Store {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn is_wasm_trap(&self, addr: usize) -> bool {
        self.frame_info().borrow().lookup_trap_info(addr).is_some()
    }

    fn custom_signal_handler(&self, call: &dyn Fn(&SignalHandler) -> bool) -> bool {
        if let Some(handler) = &*self.inner.signal_handler.borrow() {
            return call(handler);
        }
        false
    }

    fn max_wasm_stack(&self) -> usize {
        self.engine().config().max_wasm_stack
    }

    fn out_of_gas(&self) {
        #[derive(Debug)]
        struct OutOfGas;

        impl fmt::Display for OutOfGas {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("all fuel consumed by WebAssembly")
            }
        }

        impl std::error::Error for OutOfGas {}

        unsafe {
            wasmtime_runtime::raise_lib_trap(wasmtime_runtime::Trap::User(Box::new(OutOfGas)))
        }
    }
}

impl Default for Store {
    fn default() -> Store {
        Store::new(&Engine::default())
    }
}

impl fmt::Debug for Store {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let inner = &*self.inner as *const StoreInner;
        f.debug_struct("Store").field("inner", &inner).finish()
    }
}

impl Drop for StoreInner {
    fn drop(&mut self) {
        for instance in self.instances.get_mut().iter() {
            unsafe {
                instance.dealloc();
            }
        }
    }
}

/// A threadsafe handle used to interrupt instances executing within a
/// particular `Store`.
///
/// This structure is created by the [`Store::interrupt_handle`] method.
pub struct InterruptHandle {
    interrupts: Arc<VMInterrupts>,
}

// The `VMInterrupts` type is a pod-type with no destructor, and we only access
// `interrupts` from other threads, so add in these trait impls which are
// otherwise not available due to the `fuel_consumed` variable in
// `VMInterrupts`.
unsafe impl Send for InterruptHandle {}
unsafe impl Sync for InterruptHandle {}

impl InterruptHandle {
    /// Flags that execution within this handle's original [`Store`] should be
    /// interrupted.
    ///
    /// This will not immediately interrupt execution of wasm modules, but
    /// rather it will interrupt wasm execution of loop headers and wasm
    /// execution of function entries. For more information see
    /// [`Store::interrupt_handle`].
    pub fn interrupt(&self) {
        self.interrupts.interrupt()
    }
}

// Wrapper struct to implement hash/equality based on the pointer value of the
// `Arc` in question.
struct ArcModuleCode(Arc<ModuleCode>);

impl PartialEq for ArcModuleCode {
    fn eq(&self, other: &ArcModuleCode) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for ArcModuleCode {}

impl Hash for ArcModuleCode {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        Arc::as_ptr(&self.0).hash(hasher)
    }
}
