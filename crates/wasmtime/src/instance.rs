use crate::types::matching;
use crate::{
    Engine, Export, Extern, Func, Global, InstanceType, Memory, Module, Store, Table, Trap,
};
use anyhow::{bail, Context, Error, Result};
use std::mem;
use std::rc::Rc;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::wasm::{
    EntityIndex, FuncIndex, GlobalIndex, InstanceIndex, MemoryIndex, ModuleIndex, TableIndex,
};
use wasmtime_environ::Initializer;
use wasmtime_runtime::{
    Imports, InstantiationError, RuntimeInstance, StackMapRegistry, VMContext,
    VMExternRefActivationsTable, VMFunctionBody, VMFunctionImport, VMGlobalImport, VMMemoryImport,
    VMTableImport,
};

/// Performs all low-level steps necessary for instantiation.
///
/// This function will take all the arguments and attempt to do everything
/// necessary to instantiate the referenced instance. The trickiness of this
/// function stems from the implementation of the module-linking proposal where
/// we're handling nested instances, interleaved imports/aliases, etc. That's
/// all an internal implementation here ideally though!
///
/// * `store` - the store we're instantiating into
/// * `compiled_module` - the module that we're instantiating
/// * `all_modules` - the list of all modules that were part of the compilation
///   of `compiled_module`. This is only applicable in the module linking
///   proposal, otherwise this will just be a list containing `compiled_module`
///   itself.
/// * `type` - the type tables produced during compilation which
///   `compiled_module`'s metadata references.
/// * `define_import` - this function, like the name implies, defines an import
///   into the provided builder. The expected entity that it's defining is also
///   passed in for the top-level case where type-checking is performed. This is
///   fallible because type checks may fail.
fn instantiate(
    store: &Store,
    module: &Module,
    define_import: &mut dyn FnMut(
        &str,
        Option<&str>,
        &EntityIndex,
        &mut ImportsBuilder<'_>,
    ) -> Result<()>,
) -> Result<RuntimeInstance, Error> {
    let compiled_module = module.compiled_module();
    let env_module = compiled_module.module();

    let mut imports = ImportsBuilder::new(store, module);
    for initializer in env_module.initializers.iter() {
        match initializer {
            // Definition of an import depends on how our parent is providing
            // imports, so we delegate to our custom closure. This will resolve
            // to fetching from the import list for the top-level module and
            // otherwise fetching from each nested instance's argument list for
            // submodules.
            Initializer::Import { index, name, field } => {
                define_import(name, field.as_deref(), index, &mut imports)
                    .with_context(|| format!("incompatible import type for `{}`", name))?;
            }

            // Here we lookup our instance handle, find the right export,
            // and then push that item into our own index space. We eschew
            // type-checking since only valid modules should reach this point.
            Initializer::AliasInstanceExport { instance, export } => {
                let export = &imports.instances[*instance][export];
                let item = unsafe { Extern::from_wasmtime_export(export, store) };
                imports.push_extern(&item);
            }

            // Oh boy a recursive instantiation!
            //
            // We use our local index space of modules to find the module to
            // instantiate and argument lookup is defined as looking inside of
            // `args`. Like above with aliases all type checking is eschewed
            // because only valid modules should reach this point.
            //
            // Note that it's thought that due to the acyclic nature of
            // instantiation this can't loop to blow the native stack, and
            // validation should in theory ensure this has a bounded depth.
            // Despite this we may need to change this to a loop instead of
            // recursion one day.
            Initializer::Instantiate { module, args } => {
                let handle = instantiate(
                    store,
                    &imports.modules[*module],
                    &mut |name, field, _, builder| {
                        debug_assert!(field.is_none());
                        let index = args.get(name).expect("should be present after validation");
                        match *index {
                            EntityIndex::Global(i) => {
                                builder.globals.push(imports.globals[i]);
                            }
                            EntityIndex::Function(i) => {
                                builder.functions.push(imports.functions[i]);
                            }
                            EntityIndex::Table(i) => {
                                builder.tables.push(imports.tables[i]);
                            }
                            EntityIndex::Memory(i) => {
                                builder.memories.push(imports.memories[i]);
                            }
                            EntityIndex::Module(i) => {
                                builder.modules.push(imports.modules[i].clone());
                            }
                            EntityIndex::Instance(i) => {
                                builder.instances.push(imports.instances[i].clone());
                            }
                        }
                        Ok(())
                    },
                )?;
                imports.instances.push(handle);
            }

            // A new module is being defined, and the source of this module is
            // our module's list of closed-over-modules.
            //
            // This is used for outer aliases.
            Initializer::DefineModule(upvar_index) => {
                imports
                    .modules
                    .push(module.module_upvar(*upvar_index).clone());
            }

            // A new module is defined, created from a set of compiled
            // artifacts. The new module value will be created with the
            // specified artifacts being closed over as well as the specified
            // set of module values in our index/upvar index spaces being closed
            // over.
            //
            // This is used for defining submodules.
            Initializer::CreateModule {
                artifact_index,
                artifacts,
                modules,
            } => {
                let submodule =
                    module.create_submodule(*artifact_index, artifacts, modules, &imports.modules);
                imports.modules.push(submodule);
            }
        }
    }

    // Register the module just before instantiation to ensure we have a
    // trampoline registered for every signature and to preserve the module's
    // compiled JIT code within the `Store`.
    store.register_module(module);

    let config = store.engine().config();
    let instance = unsafe {
        let instance = compiled_module.instantiate(
            imports.build(),
            &store.lookup_shared_signature(module.types()),
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

    let exports = instance
        .handle
        .module()
        .exports
        .iter()
        .map(|(name, index)| {
            // Note that instances and modules are not handled by
            // `wasmtime_runtime`, they're handled by us in this crate. That
            // means we need to handle that here, otherwise we defer to the
            // instance to load the values.
            let item = match index {
                EntityIndex::Instance(i) => {
                    wasmtime_runtime::Export::Instance(imports.instances[*i].clone())
                }
                EntityIndex::Module(i) => {
                    wasmtime_runtime::Export::Module(Box::new(imports.modules[*i].clone()))
                }
                index => instance.handle.lookup_by_declaration(index),
            };
            (name.clone(), item)
        })
        .collect();
    Ok(Rc::new(exports))
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
    pub(crate) store: Store,
    pub(crate) items: RuntimeInstance,
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

        // Perform some pre-flight checks before we get into the meat of
        // instantiation.
        let expected = module.compiled_module().module().imports().count();
        if expected != imports.len() {
            bail!("expected {} imports, found {}", expected, imports.len());
        }
        for import in imports {
            if !import.comes_from_same_store(store) {
                bail!("cross-`Store` instantiation is not currently supported");
            }
        }

        let mut imports = imports.iter();
        let items = instantiate(store, module, &mut |_name, _field, idx, builder| {
            let import = imports.next().expect("already checked the length");
            builder.define_extern(idx, &import)
        })?;

        Ok(Instance::from_wasmtime(&items, store))
    }

    pub(crate) fn from_wasmtime(handle: &RuntimeInstance, store: &Store) -> Instance {
        Instance {
            items: handle.clone(),
            store: store.clone(),
        }
    }

    /// Returns the type signature of this instance.
    pub fn ty(&self) -> InstanceType {
        let mut ty = InstanceType::new();
        for export in self.exports() {
            ty.add_named_export(export.name(), export.ty());
        }
        ty
    }

    /// Returns the associated [`Store`] that this `Instance` is compiled into.
    ///
    /// This is the [`Store`] that generally serves as a sort of global cache
    /// for various instance-related things.
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Returns the list of exported items from this [`Instance`].
    pub fn exports<'instance>(
        &'instance self,
    ) -> impl ExactSizeIterator<Item = Export<'instance>> + 'instance {
        self.items.iter().map(move |(name, item)| {
            let extern_ = unsafe { Extern::from_wasmtime_export(item, &self.store) };
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
        let export = self.items.get(name)?;
        Some(unsafe { Extern::from_wasmtime_export(export, &self.store) })
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
    instances: PrimaryMap<InstanceIndex, RuntimeInstance>,
    modules: PrimaryMap<ModuleIndex, Module>,

    module: &'a wasmtime_environ::Module,
    matcher: matching::MatchCx<'a>,
}

impl<'a> ImportsBuilder<'a> {
    fn new(store: &'a Store, module: &'a Module) -> ImportsBuilder<'a> {
        let types = module.types();
        let module = module.compiled_module().module();
        ImportsBuilder {
            module,
            matcher: matching::MatchCx { store, types },
            functions: PrimaryMap::with_capacity(module.num_imported_funcs),
            tables: PrimaryMap::with_capacity(module.num_imported_tables),
            memories: PrimaryMap::with_capacity(module.num_imported_memories),
            globals: PrimaryMap::with_capacity(module.num_imported_globals),
            instances: PrimaryMap::with_capacity(module.instances.len()),
            modules: PrimaryMap::with_capacity(module.modules.len()),
        }
    }

    fn define_extern(&mut self, expected: &EntityIndex, actual: &Extern) -> Result<()> {
        let expected_ty = self.module.type_of(*expected);
        self.matcher.extern_(&expected_ty, actual)?;
        self.push_extern(actual);
        Ok(())
    }

    fn push_extern(&mut self, item: &Extern) {
        match item {
            Extern::Func(i) => {
                self.functions.push(i.vmimport());
            }
            Extern::Global(i) => {
                self.globals.push(i.vmimport());
            }
            Extern::Table(i) => {
                self.tables.push(i.vmimport());
            }
            Extern::Memory(i) => {
                self.memories.push(i.vmimport());
            }
            Extern::Instance(i) => {
                debug_assert!(Store::same(i.store(), self.matcher.store));
                self.instances.push(i.items.clone());
            }
            Extern::Module(m) => {
                self.modules.push(m.clone());
            }
        }
    }

    fn build(&mut self) -> Imports<'_> {
        Imports {
            tables: self.tables.values().as_slice(),
            globals: self.globals.values().as_slice(),
            memories: self.memories.values().as_slice(),
            functions: self.functions.values().as_slice(),
        }
    }
}

/// An internal structure to this crate to build an `Instance` from a list of
/// items with names. This is intended to stay private for now, it'll need an
/// audit of APIs if publicly exported.
#[derive(Default)]
pub(crate) struct InstanceBuilder {
    items: RuntimeInstance,
}

impl InstanceBuilder {
    pub(crate) fn new() -> InstanceBuilder {
        InstanceBuilder::default()
    }

    pub(crate) fn insert(&mut self, name: &str, item: impl Into<Extern>) {
        let items = Rc::get_mut(&mut self.items).unwrap();
        let export = match item.into() {
            Extern::Func(i) => wasmtime_runtime::Export::Function(i.wasmtime_export().clone()),
            Extern::Memory(i) => wasmtime_runtime::Export::Memory(i.wasmtime_export().clone()),
            Extern::Table(i) => wasmtime_runtime::Export::Table(i.wasmtime_export().clone()),
            Extern::Global(i) => wasmtime_runtime::Export::Global(i.wasmtime_export().clone()),
            Extern::Instance(i) => wasmtime_runtime::Export::Instance(i.items.clone()),
            Extern::Module(i) => wasmtime_runtime::Export::Module(Box::new(i.clone())),
        };
        items.insert(name.to_string(), export);
    }

    pub(crate) fn finish(self, store: &Store) -> Instance {
        Instance {
            store: store.clone(),
            items: self.items,
        }
    }
}
