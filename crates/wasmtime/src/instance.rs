use crate::trampoline::StoreInstanceHandle;
use crate::{Engine, Export, Extern, Func, Global, Memory, Module, Store, Table, Trap};
use anyhow::{bail, Error, Result};
use std::mem;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::wasm::{EntityIndex, FuncIndex, GlobalIndex, MemoryIndex, TableIndex};
use wasmtime_jit::CompiledModule;
use wasmtime_runtime::{
    Imports, InstantiationError, StackMapRegistry, VMContext, VMExternRefActivationsTable,
    VMFunctionBody, VMFunctionImport, VMGlobalImport, VMMemoryImport, VMTableImport,
};

fn instantiate(
    store: &Store,
    compiled_module: &CompiledModule,
    all_modules: &[CompiledModule],
    imports: &mut ImportsBuilder<'_>,
) -> Result<StoreInstanceHandle, Error> {
    let env_module = compiled_module.module();

    // The first part of instantiating any module is to first follow any
    // `instantiate` instructions it has as part of the module linking
    // proposal. Here we iterate overall those instructions and create the
    // instances as necessary.
    for instance in env_module.instances.values() {
        let (module_idx, args) = match instance {
            wasmtime_environ::Instance::Instantiate { module, args } => (*module, args),
            wasmtime_environ::Instance::Import(_) => continue,
        };
        // Translate the `module_idx` to a top-level module `usize` and then
        // use that to extract the child `&CompiledModule` itself. Then we can
        // iterate over each of the arguments provided to satisfy its imports.
        //
        // Note that we directly reach into `imports` below based on indexes
        // and push raw value into how to instantiate our submodule. This should
        // be safe due to wasm validation ensuring that all our indices are
        // in-bounds and all the expected types and such line up.
        let module_idx = compiled_module.submodule_idx(module_idx);
        let compiled_module = &all_modules[module_idx];
        let mut builder = ImportsBuilder::new(compiled_module.module(), store);
        for arg in args {
            match *arg {
                EntityIndex::Global(i) => {
                    builder.globals.push(imports.globals[i]);
                }
                EntityIndex::Table(i) => {
                    builder.tables.push(imports.tables[i]);
                }
                EntityIndex::Function(i) => {
                    builder.functions.push(imports.functions[i]);
                }
                EntityIndex::Memory(i) => {
                    builder.memories.push(imports.memories[i]);
                }
                EntityIndex::Module(_) => unimplemented!(),
                EntityIndex::Instance(_) => unimplemented!(),
            }
        }
        instantiate(store, compiled_module, all_modules, &mut builder)?;
    }

    // Register the module just before instantiation to ensure we have a
    // trampoline registered for every signature and to preserve the module's
    // compiled JIT code within the `Store`.
    store.register_module(compiled_module);

    let config = store.engine().config();
    let instance = unsafe {
        let instance = compiled_module.instantiate(
            imports.imports(),
            &store.lookup_shared_signature(compiled_module.module()),
            config.memory_creator.as_ref().map(|a| a as _),
            store.interrupts(),
            Box::new(()),
            store.externref_activations_table() as *const VMExternRefActivationsTable as *mut _,
            store.stack_map_registry() as *const StackMapRegistry as *mut _,
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
                config.features.bulk_memory,
                &compiled_module.data_initializers(),
            )
            .map_err(|e| -> Error {
                match e {
                    InstantiationError::Trap(trap) => Trap::from_runtime(store, trap).into(),
                    other => other.into(),
                }
            })?;

        instance
    };

    let start_func = instance.handle.module().start_func;

    // If a start function is present, invoke it. Make sure we use all the
    // trap-handling configuration in `store` as well.
    if let Some(start) = start_func {
        let f = match instance
            .handle
            .lookup_by_declaration(&EntityIndex::Function(start))
        {
            wasmtime_runtime::Export::Function(f) => f,
            _ => unreachable!(), // valid modules shouldn't hit this
        };
        let vmctx_ptr = instance.handle.vmctx_ptr();
        unsafe {
            super::func::invoke_wasm_and_catch_traps(vmctx_ptr, store, || {
                mem::transmute::<
                    *const VMFunctionBody,
                    unsafe extern "C" fn(*mut VMContext, *mut VMContext),
                >(f.anyfunc.as_ref().func_ptr.as_ptr())(
                    f.anyfunc.as_ref().vmctx, vmctx_ptr
                )
            })?;
        }
    }

    Ok(instance)
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
    /// Per the WebAssembly spec, instantiation includes running the module's
    /// start function, if it has one (not to be confused with the `_start`
    /// function, which is not run).
    ///
    /// Note that this is a low-level function that just performance an
    /// instantiation. See the `Linker` struct for an API which provides a
    /// convenient way to link imports and provides automatic Command and Reactor
    /// behavior.
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
    pub fn new(store: &Store, module: &Module, imports: &[Extern]) -> Result<Instance, Error> {
        if !Engine::same(store.engine(), module.engine()) {
            bail!("cross-`Engine` instantiation is not currently supported");
        }

        let mut builder = ImportsBuilder::new(module.compiled_module().module(), store);
        for import in imports {
            // For now we have a restriction that the `Store` that we're working
            // with is the same for everything involved here.
            if !import.comes_from_same_store(store) {
                bail!("cross-`Store` instantiation is not currently supported");
            }
            match import {
                Extern::Global(e) => builder.global(e)?,
                Extern::Func(e) => builder.func(e)?,
                Extern::Table(e) => builder.table(e)?,
                Extern::Memory(e) => builder.memory(e)?,
            }
        }
        builder.validate_all_imports_provided()?;
        let handle = instantiate(
            store,
            module.compiled_module(),
            &module.compiled,
            &mut builder,
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
        &self.handle.store
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

struct ImportsBuilder<'a> {
    functions: PrimaryMap<FuncIndex, VMFunctionImport>,
    tables: PrimaryMap<TableIndex, VMTableImport>,
    memories: PrimaryMap<MemoryIndex, VMMemoryImport>,
    globals: PrimaryMap<GlobalIndex, VMGlobalImport>,
    module: &'a wasmtime_environ::Module,
    imports: std::slice::Iter<'a, (String, Option<String>, EntityIndex)>,
    store: &'a Store,
}

impl<'a> ImportsBuilder<'a> {
    fn new(module: &'a wasmtime_environ::Module, store: &'a Store) -> ImportsBuilder<'a> {
        ImportsBuilder {
            imports: module.imports.iter(),
            module,
            store,
            functions: PrimaryMap::with_capacity(module.num_imported_funcs),
            tables: PrimaryMap::with_capacity(module.num_imported_tables),
            memories: PrimaryMap::with_capacity(module.num_imported_memories),
            globals: PrimaryMap::with_capacity(module.num_imported_globals),
        }
    }

    fn next_import(
        &mut self,
        found: &str,
        get: impl FnOnce(&wasmtime_environ::Module, &EntityIndex) -> Option<bool>,
    ) -> Result<()> {
        match self.imports.next() {
            Some((module, field, idx)) => {
                let error = match get(self.module, idx) {
                    Some(true) => return Ok(()),
                    Some(false) => {
                        anyhow::anyhow!("{} types incompatible", found)
                    }
                    None => {
                        let desc = match idx {
                            EntityIndex::Table(_) => "table",
                            EntityIndex::Function(_) => "func",
                            EntityIndex::Memory(_) => "memory",
                            EntityIndex::Global(_) => "global",
                            EntityIndex::Instance(_) => "instance",
                            EntityIndex::Module(_) => "module",
                        };
                        anyhow::anyhow!("expected {}, but found {}", desc, found)
                    }
                };
                let import_name = match field {
                    Some(name) => format!("{}/{}", module, name),
                    None => module.to_string(),
                };
                Err(error.context(format!("incompatible import type for {}", import_name)))
            }
            None => bail!("too many imports provided"),
        }
    }

    fn global(&mut self, global: &Global) -> Result<()> {
        self.next_import("global", |m, e| match e {
            EntityIndex::Global(i) => Some(global.matches_expected(&m.globals[*i])),
            _ => None,
        })?;
        self.globals.push(global.vmimport());
        Ok(())
    }

    fn memory(&mut self, mem: &Memory) -> Result<()> {
        self.next_import("memory", |m, e| match e {
            EntityIndex::Memory(i) => Some(mem.matches_expected(&m.memory_plans[*i])),
            _ => None,
        })?;
        self.memories.push(mem.vmimport());
        Ok(())
    }

    fn table(&mut self, table: &Table) -> Result<()> {
        self.next_import("table", |m, e| match e {
            EntityIndex::Table(i) => Some(table.matches_expected(&m.table_plans[*i])),
            _ => None,
        })?;
        self.tables.push(table.vmimport());
        Ok(())
    }

    fn func(&mut self, func: &Func) -> Result<()> {
        let store = self.store;
        self.next_import("func", |m, e| match e {
            EntityIndex::Function(i) => Some(
                // Look up the `i`th function's type from the module in our
                // signature registry. If it's not present then we have no
                // functions registered with that type, so `func` is guaranteed
                // to not match.
                match store
                    .signatures()
                    .borrow()
                    .lookup(&m.signatures[m.functions[*i]])
                {
                    Some(ty) => func.matches_expected(ty),
                    None => false,
                },
            ),
            _ => None,
        })?;
        self.functions.push(func.vmimport());
        Ok(())
    }

    fn validate_all_imports_provided(&mut self) -> Result<()> {
        if self.imports.next().is_some() {
            bail!("not enough imports provided");
        }
        Ok(())
    }

    fn imports(&self) -> Imports<'_> {
        Imports {
            tables: self.tables.values().as_slice(),
            globals: self.globals.values().as_slice(),
            memories: self.memories.values().as_slice(),
            functions: self.functions.values().as_slice(),
        }
    }
}
