use crate::sig_registry::SignatureRegistry;
use crate::trampoline::StoreInstanceHandle;
use crate::Engine;
use crate::Module;
use anyhow::{bail, Result};
use std::cell::RefCell;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::{Rc, Weak};
use std::sync::Arc;
use wasmtime_environ::wasm;
use wasmtime_runtime::{
    InstanceHandle, RuntimeMemoryCreator, SignalHandler, StackMapRegistry, VMExternRef,
    VMExternRefActivationsTable, VMInterrupts, VMSharedSignatureIndex,
};

/// A `Store` is a collection of WebAssembly instances and host-defined items.
///
/// All WebAssembly instances and items will be attached to and refer to a
/// `Store`. For example instances, functions, globals, and tables are all
/// attached to a `Store`. Instances are created by instantiating a [`Module`]
/// within a `Store`.
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
    jit_code_ranges: RefCell<Vec<(usize, usize)>>,
    externref_activations_table: VMExternRefActivationsTable,
    stack_map_registry: StackMapRegistry,
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
                jit_code_ranges: RefCell::new(Vec::new()),
                externref_activations_table: VMExternRefActivationsTable::new(),
                stack_map_registry: StackMapRegistry::default(),
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
        module: &'a wasmtime_environ::Module,
    ) -> impl Fn(wasm::SignatureIndex) -> VMSharedSignatureIndex + 'a {
        move |index| {
            self.signatures()
                .borrow()
                .lookup(&module.signatures[index])
                .expect("signature not previously registered")
        }
    }

    /// Returns whether or not the given address falls within the JIT code
    /// managed by the compiler
    pub(crate) fn is_in_jit_code(&self, addr: usize) -> bool {
        self.inner
            .jit_code_ranges
            .borrow()
            .iter()
            .any(|(start, end)| *start <= addr && addr < *end)
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
        self.register_jit_code(module);

        // We need to know about all the stack maps of all instantiated modules
        // so when performing a GC we know about all wasm frames that we find
        // on the stack.
        self.register_stack_maps(module);

        // Signatures are loaded into our `SignatureRegistry` here
        // once-per-module (and once-per-signature). This allows us to create
        // a `Func` wrapper for any function in the module, which requires that
        // we know about the signature and trampoline for all instances.
        self.register_signatures(module);
    }

    fn register_jit_code(&self, module: &Module) {
        let mut ranges = module.compiled_module().jit_code_ranges();
        // Checking of we already registered JIT code ranges by searching
        // first range start.
        match ranges.next() {
            None => (),
            Some(first) => {
                if !self.is_in_jit_code(first.0) {
                    // The range is not registered -- add all ranges (including
                    // first one) to the jit_code_ranges.
                    let mut jit_code_ranges = self.inner.jit_code_ranges.borrow_mut();
                    jit_code_ranges.push(first);
                    jit_code_ranges.extend(ranges);
                }
            }
        }
    }

    fn register_stack_maps(&self, module: &Module) {
        let module = &module.compiled_module();
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
        let module = module.compiled_module().module();
        let mut signatures = self.signatures().borrow_mut();
        for (index, wasm) in module.signatures.iter() {
            signatures.register(wasm, trampolines[index]);
        }
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

    pub(crate) fn weak(&self) -> Weak<StoreInner> {
        Rc::downgrade(&self.inner)
    }

    pub(crate) fn upgrade(weak: &Weak<StoreInner>) -> Option<Self> {
        let inner = weak.upgrade()?;
        Some(Self { inner })
    }

    pub(crate) fn signal_handler(&self) -> std::cell::Ref<'_, Option<Box<SignalHandler<'static>>>> {
        self.inner.signal_handler.borrow()
    }

    pub(crate) fn signal_handler_mut(
        &self,
    ) -> std::cell::RefMut<'_, Option<Box<SignalHandler<'static>>>> {
        self.inner.signal_handler.borrow_mut()
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
