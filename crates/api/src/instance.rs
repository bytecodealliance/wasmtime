use crate::trampoline::StoreInstanceHandle;
use crate::{Export, Extern, Func, Global, Memory, Module, Store, Table, Trap};
use anyhow::{bail, Error, Result};
use std::any::Any;
use std::mem;
use wasmtime_environ::EntityIndex;
use wasmtime_jit::{CompiledModule, Resolver};
use wasmtime_runtime::{InstantiationError, SignatureRegistry, VMContext, VMFunctionBody};

struct SimpleResolver<'a> {
    imports: &'a [Extern],
}

impl Resolver for SimpleResolver<'_> {
    fn resolve(&mut self, idx: u32, _name: &str, _field: &str) -> Option<wasmtime_runtime::Export> {
        self.imports
            .get(idx as usize)
            .map(|i| i.get_wasmtime_export())
    }
}

fn instantiate(
    store: &Store,
    compiled_module: &CompiledModule,
    imports: &[Extern],
    sig_registry: &SignatureRegistry,
    host: Box<dyn Any>,
) -> Result<StoreInstanceHandle, Error> {
    let mut resolver = SimpleResolver { imports };
    unsafe {
        let config = store.engine().config();
        let instance = compiled_module.instantiate(
            &mut resolver,
            sig_registry,
            config.memory_creator.as_ref().map(|a| a as _),
            host,
        )?;

        // After we've created the `InstanceHandle` we still need to run
        // initialization to set up data/elements/etc. We do this after adding
        // the `InstanceHandle` to the store though. This is required for safety
        // because the start function (for example) may trap, but element
        // initializers may have run which placed elements into other instance's
        // tables. This means that from this point on, regardless of whether
        // initialization is successful, we need to keep the instance alive.
        let instance = store.add_instance(instance);
        instance
            .initialize(
                config.validating_config.operator_config.enable_bulk_memory,
                &compiled_module.data_initializers(),
            )
            .map_err(|e| -> Error {
                match e {
                    InstantiationError::StartTrap(trap) | InstantiationError::Trap(trap) => {
                        Trap::from_jit(trap).into()
                    }
                    other => other.into(),
                }
            })?;

        // If a start function is present, now that we've got our compiled
        // instance we can invoke it. Make sure we use all the trap-handling
        // configuration in `store` as well.
        if let Some(start) = instance.module().start_func {
            let f = match instance.lookup_by_declaration(&EntityIndex::Function(start)) {
                wasmtime_runtime::Export::Function(f) => f,
                _ => unreachable!(), // valid modules shouldn't hit this
            };
            super::func::catch_traps(instance.vmctx_ptr(), store, || {
                mem::transmute::<
                    *const VMFunctionBody,
                    unsafe extern "C" fn(*mut VMContext, *mut VMContext),
                >(f.address)(f.vmctx, instance.vmctx_ptr())
            })?;
        }

        Ok(instance)
    }
}

/// An instantiated WebAssembly module.
///
/// This type represents the instantiation of a [`Module`]. Once instantiated
/// you can access the [`exports`](Instance::exports) which are of type
/// [`Extern`] and provide the ability to call functions, set globals, read
/// memory, etc. This is where all the fun stuff happens!
///
/// An [`Instance`] is created from two inputs, a [`Module`] and a list of
/// imports, provided as a list of [`Extern`] values. The [`Module`] is the wasm
/// code that was compiled and we're instantiating, and the [`Extern`] imports
/// are how we're satisfying the imports of the module provided. On successful
/// instantiation an [`Instance`] will automatically invoke the wasm `start`
/// function.
///
/// When interacting with any wasm code you'll want to make an [`Instance`] to
/// call any code or execute anything!
#[derive(Clone)]
pub struct Instance {
    pub(crate) handle: StoreInstanceHandle,
    module: Module,
}

impl Instance {
    /// Creates a new [`Instance`] from the previously compiled [`Module`] and
    /// list of `imports` specified.
    ///
    /// This method instantiates the `module` provided with the `imports`,
    /// following the procedure in the [core specification][inst] to
    /// instantiate. Instantiation can fail for a number of reasons (many
    /// specified below), but if successful the `start` function will be
    /// automatically run (if provided) and then the [`Instance`] will be
    /// returned.
    ///
    /// ## Providing Imports
    ///
    /// The `imports` array here is a bit tricky. The entries in the list of
    /// `imports` are intended to correspond 1:1 with the list of imports
    /// returned by [`Module::imports`]. Before calling [`Instance::new`] you'll
    /// want to inspect the return value of [`Module::imports`] and, for each
    /// import type, create an [`Extern`] which corresponds to that type.
    /// These [`Extern`] values are all then collected into a list and passed to
    /// this function.
    ///
    /// Note that this function is intentionally relatively low level. It is the
    /// intention that we'll soon provide a [higher level API][issue] which will
    /// be much more ergonomic for instantiating modules. If you need the full
    /// power of customization of imports, though, this is the method for you!
    ///
    /// ## Errors
    ///
    /// This function can fail for a number of reasons, including, but not
    /// limited to:
    ///
    /// * The number of `imports` provided doesn't match the number of imports
    ///   returned by the `module`'s [`Module::imports`] method.
    /// * The type of any [`Extern`] doesn't match the corresponding
    ///   [`ExternType`] entry that it maps to.
    /// * The `start` function in the instance, if present, traps.
    /// * Module/instance resource limits are exceeded.
    ///
    /// When instantiation fails it's recommended to inspect the return value to
    /// see why it failed, or bubble it upwards. If you'd like to specifically
    /// check for trap errors, you can use `error.downcast::<Trap>()`.
    ///
    /// [inst]: https://webassembly.github.io/spec/core/exec/modules.html#exec-instantiation
    /// [issue]: https://github.com/bytecodealliance/wasmtime/issues/727
    /// [`ExternType`]: crate::ExternType
    pub fn new(module: &Module, imports: &[Extern]) -> Result<Instance, Error> {
        let store = module.store();

        // For now we have a restriction that the `Store` that we're working
        // with is the same for everything involved here.
        for import in imports {
            if !import.comes_from_same_store(store) {
                bail!("cross-`Store` instantiation is not currently supported");
            }
        }

        if imports.len() != module.imports().len() {
            bail!(
                "wrong number of imports provided, {} != {}",
                imports.len(),
                module.imports().len()
            );
        }

        let info = module.register_frame_info();
        let handle = instantiate(
            store,
            module.compiled_module(),
            imports,
            store.compiler().signatures(),
            Box::new(info),
        )?;

        Ok(Instance {
            handle,
            module: module.clone(),
        })
    }

    /// Returns the associated [`Store`] that this `Instance` is compiled into.
    ///
    /// This is the [`Store`] that generally serves as a sort of global cache
    /// for various instance-related things.
    pub fn store(&self) -> &Store {
        self.module.store()
    }

    /// Returns the list of exported items from this [`Instance`].
    pub fn exports<'instance>(
        &'instance self,
    ) -> impl ExactSizeIterator<Item = Export<'instance>> + 'instance {
        self.handle.exports().map(move |(name, entity_index)| {
            let export = self.handle.lookup_by_declaration(entity_index);
            let extern_ = Extern::from_wasmtime_export(export, self.handle.clone());
            Export::new(name, extern_)
        })
    }

    /// Looks up an exported [`Extern`] value by name.
    ///
    /// This method will search the module for an export named `name` and return
    /// the value, if found.
    ///
    /// Returns `None` if there was no export named `name`.
    pub fn get_export(&self, name: &str) -> Option<Extern> {
        let export = self.handle.lookup(&name)?;
        Some(Extern::from_wasmtime_export(export, self.handle.clone()))
    }

    /// Looks up an exported [`Func`] value by name.
    ///
    /// Returns `None` if there was no export named `name`, or if there was but
    /// it wasn't a function.
    pub fn get_func(&self, name: &str) -> Option<Func> {
        self.get_export(name)?.into_func()
    }

    /// Looks up an exported [`Table`] value by name.
    ///
    /// Returns `None` if there was no export named `name`, or if there was but
    /// it wasn't a table.
    pub fn get_table(&self, name: &str) -> Option<Table> {
        self.get_export(name)?.into_table()
    }

    /// Looks up an exported [`Memory`] value by name.
    ///
    /// Returns `None` if there was no export named `name`, or if there was but
    /// it wasn't a memory.
    pub fn get_memory(&self, name: &str) -> Option<Memory> {
        self.get_export(name)?.into_memory()
    }

    /// Looks up an exported [`Global`] value by name.
    ///
    /// Returns `None` if there was no export named `name`, or if there was but
    /// it wasn't a global.
    pub fn get_global(&self, name: &str) -> Option<Global> {
        self.get_export(name)?.into_global()
    }
}
