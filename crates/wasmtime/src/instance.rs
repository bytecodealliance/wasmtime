use crate::trampoline::StoreInstanceHandle;
use crate::types::matching;
use crate::{
    Engine, Export, Extern, Func, Global, InstanceType, Memory, Module, Store, Table, Trap,
    TypedFunc,
};
use anyhow::{anyhow, bail, Context, Error, Result};
use std::mem;
use std::rc::Rc;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::wasm::{
    EntityIndex, FuncIndex, GlobalIndex, InstanceIndex, MemoryIndex, ModuleIndex, TableIndex,
};
use wasmtime_environ::Initializer;
use wasmtime_runtime::{
    Imports, InstanceAllocationRequest, InstantiationError, RuntimeInstance, VMContext,
    VMExternRefActivationsTable, VMFunctionBody, VMFunctionImport, VMGlobalImport, VMMemoryImport,
    VMTableImport,
};

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
    /// # Panics
    ///
    /// This function will panic if called with a store associated with a
    /// [`asynchronous config`](crate::Config::async_support).
    ///
    /// [inst]: https://webassembly.github.io/spec/core/exec/modules.html#exec-instantiation
    /// [issue]: https://github.com/bytecodealliance/wasmtime/issues/727
    /// [`ExternType`]: crate::ExternType
    pub fn new(store: &Store, module: &Module, imports: &[Extern]) -> Result<Instance, Error> {
        assert!(
            !store.async_support(),
            "cannot use `new` when async support is enabled on the config"
        );

        // NB: this is the same code as `Instance::new_async`. It's intentionally
        // small but should be kept in sync (modulo the async bits).
        let mut i = Instantiator::new(store, module, imports)?;
        loop {
            if let Some((instance, items)) = i.step()? {
                Instantiator::start_raw(&instance)?;
                if let Some(items) = items {
                    break Ok(Instance::from_wasmtime(&items, store));
                }
            }
        }
    }

    /// Same as [`Instance::new`], except for usage in [asynchronous stores].
    ///
    /// For more details about this function see the documentation on
    /// [`Instance::new`]. The only difference between these two methods is that
    /// this one will asynchronously invoke the wasm start function in case it
    /// calls any imported function which is an asynchronous host function (e.g.
    /// created with [`Func::new_async`](crate::Func::new_async).
    ///
    /// # Panics
    ///
    /// This function will panic if called with a store associated with a [`synchronous
    /// config`](crate::Config::new). This is only compatible with stores associated with
    /// an [`asynchronous config`](crate::Config::async_support).
    #[cfg(feature = "async")]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "async")))]
    pub async fn new_async(
        store: &Store,
        module: &Module,
        imports: &[Extern],
    ) -> Result<Instance, Error> {
        assert!(
            store.async_support(),
            "cannot use `new_async` without enabling async support on the config"
        );

        // NB: this is the same code as `Instance::new`. It's intentionally
        // small but should be kept in sync (modulo the async bits).
        let mut i = Instantiator::new(store, module, imports)?;
        loop {
            if let Some((instance, items)) = i.step()? {
                if instance.handle.module().start_func.is_some() {
                    store
                        .on_fiber(|| Instantiator::start_raw(&instance))
                        .await??;
                }
                if let Some(items) = items {
                    break Ok(Instance::from_wasmtime(&items, store));
                }
            }
        }
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

    pub(crate) fn wasmtime_export(&self) -> &RuntimeInstance {
        &self.items
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

    /// Looks up an exported [`Func`] value by name and with its type.
    ///
    /// This function is a convenience wrapper over [`Instance::get_func`] and
    /// [`Func::typed`]. For more information see the linked documentation.
    ///
    /// Returns an error if `name` isn't a function export or if the export's
    /// type did not match `Params` or `Results`
    pub fn get_typed_func<Params, Results>(&self, name: &str) -> Result<TypedFunc<Params, Results>>
    where
        Params: crate::WasmParams,
        Results: crate::WasmResults,
    {
        let f = self
            .get_export(name)
            .and_then(|f| f.into_func())
            .ok_or_else(|| anyhow!("failed to find function export `{}`", name))?;
        Ok(f.typed::<Params, Results>()?.clone())
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

struct Instantiator<'a> {
    in_progress: Vec<ImportsBuilder<'a>>,
    cur: ImportsBuilder<'a>,
    store: &'a Store,
}

struct ImportsBuilder<'a> {
    src: ImportSource<'a>,
    functions: PrimaryMap<FuncIndex, VMFunctionImport>,
    tables: PrimaryMap<TableIndex, VMTableImport>,
    memories: PrimaryMap<MemoryIndex, VMMemoryImport>,
    globals: PrimaryMap<GlobalIndex, VMGlobalImport>,
    instances: PrimaryMap<InstanceIndex, RuntimeInstance>,
    modules: PrimaryMap<ModuleIndex, Module>,
    initializer: usize,
    module: Module,
}

enum ImportSource<'a> {
    Runtime(&'a [Extern]),
    Outer { initializer: usize },
}

impl<'a> Instantiator<'a> {
    /// Creates a new instantiation context used to process all the initializer
    /// directives of a module.
    ///
    /// This doesn't do much work itself beyond setting things up.
    fn new(store: &'a Store, module: &Module, imports: &'a [Extern]) -> Result<Instantiator<'a>> {
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

        Ok(Instantiator {
            in_progress: Vec::new(),
            cur: ImportsBuilder::new(module, ImportSource::Runtime(imports)),
            store,
        })
    }

    /// Processes the next initializer for the next instance being created
    /// without running any wasm code.
    ///
    /// This function will process module initializers, handling recursive
    /// instantiations of modules for module linking if necessary as well. This
    /// does not actually execute any WebAssembly code, which means that it
    /// will return whenever an instance is created (because its `start`
    /// function may need to be executed).
    ///
    /// If this function returns `None`, then it simply needs to be called
    /// again to execute the next initializer. Otherwise this function has two
    /// return values:
    ///
    /// * The first is the raw handle to the instance that was just created.
    ///   This instance must have its start function executed by the caller.
    /// * The second is an optional list of items to get wrapped up in an
    ///   `Instance`. This is only `Some` for the outermost instance that was
    ///   created. If this is `None` callers need to keep calling this function
    ///   since the instance created was simply for a recursive instance
    ///   defined here.
    fn step(&mut self) -> Result<Option<(StoreInstanceHandle, Option<RuntimeInstance>)>> {
        if self.cur.initializer == 0 {
            self.store.bump_resource_counts(&self.cur.module)?;
        }

        // Read the current module's initializer and move forward the
        // initializer pointer as well.
        self.cur.initializer += 1;
        match self
            .cur
            .module
            .env_module()
            .initializers
            .get(self.cur.initializer - 1)
        {
            Some(Initializer::Import { index, name, field }) => {
                match &mut self.cur.src {
                    // If imports are coming from the runtime-provided list
                    // (e.g. the root module being instantiated) then we
                    // need to typecheck each item here before recording it.
                    //
                    // Note the `unwrap` here should be ok given the validation
                    // above in `Instantiation::new`.
                    ImportSource::Runtime(list) => {
                        let (head, remaining) = list.split_first().unwrap();
                        *list = remaining;
                        let expected_ty =
                            self.cur.module.compiled_module().module().type_of(*index);
                        matching::MatchCx {
                            signatures: self.cur.module.signatures(),
                            types: self.cur.module.types(),
                            store: self.store,
                        }
                        .extern_(&expected_ty, head)
                        .with_context(|| {
                            let extra = match field {
                                Some(name) => format!("::{}", name),
                                None => String::new(),
                            };
                            format!("incompatible import type for `{}{}`", name, extra)
                        })?;
                        self.cur.push(head);
                    }

                    // Otherwise if arguments are coming from our outer
                    // instance due to a recursive instantiation then we
                    // look in the previous initializer's mapping of
                    // arguments to figure out where to load the item from.
                    // Note that no typechecking is necessary here due to
                    // validation.
                    ImportSource::Outer { initializer } => {
                        debug_assert!(field.is_none());
                        let outer = self.in_progress.last().unwrap();
                        let args = match &outer.module.env_module().initializers[*initializer] {
                            Initializer::Instantiate { args, .. } => args,
                            _ => unreachable!(),
                        };
                        let index = args.get(name).expect("should be present after validation");
                        match *index {
                            EntityIndex::Global(i) => {
                                self.cur.globals.push(outer.globals[i]);
                            }
                            EntityIndex::Function(i) => {
                                self.cur.functions.push(outer.functions[i]);
                            }
                            EntityIndex::Table(i) => {
                                self.cur.tables.push(outer.tables[i]);
                            }
                            EntityIndex::Memory(i) => {
                                self.cur.memories.push(outer.memories[i]);
                            }
                            EntityIndex::Module(i) => {
                                self.cur.modules.push(outer.modules[i].clone());
                            }
                            EntityIndex::Instance(i) => {
                                self.cur.instances.push(outer.instances[i].clone());
                            }
                        }
                    }
                }
            }

            // Here we lookup our instance handle, find the right export,
            // and then push that item into our own index space. We eschew
            // type-checking since only valid modules should reach this point.
            Some(Initializer::AliasInstanceExport { instance, export }) => {
                let export = &self.cur.instances[*instance][export];
                let item = unsafe { Extern::from_wasmtime_export(export, self.store) };
                self.cur.push(&item);
            }

            // A recursive instantiation of an instance.
            //
            // The `module` argument is used to create an import builder
            // object, and we specify that the source of imports for the builder is
            // this initializer's position so we can look at the `args` payload
            // later.
            //
            // Once that's set up we save off `self.cur` into
            // `self.in_progress` and start the instantiation of the child
            // instance on the next execution of this function.
            Some(Initializer::Instantiate { module, args: _ }) => {
                let module = &self.cur.modules[*module];
                let imports = ImportsBuilder::new(
                    module,
                    ImportSource::Outer {
                        initializer: self.cur.initializer - 1,
                    },
                );
                let prev = mem::replace(&mut self.cur, imports);
                self.in_progress.push(prev);
            }

            // A new module is being defined, and the source of this module is
            // our module's list of closed-over-modules.
            //
            // This is used for outer aliases.
            Some(Initializer::DefineModule(upvar_index)) => {
                self.cur
                    .modules
                    .push(self.cur.module.module_upvar(*upvar_index).clone());
            }

            // A new module is defined, created from a set of compiled
            // artifacts. The new module value will be created with the
            // specified artifacts being closed over as well as the specified
            // set of module values in our index/upvar index spaces being closed
            // over.
            //
            // This is used for defining submodules.
            Some(Initializer::CreateModule {
                artifact_index,
                artifacts,
                modules,
            }) => {
                let submodule = self.cur.module.create_submodule(
                    *artifact_index,
                    artifacts,
                    modules,
                    &self.cur.modules,
                );
                self.cur.modules.push(submodule);
            }

            // All initializers have been processed, which means we're ready to
            // perform the actual raw instantiation with the raw import values.
            // Once that's done if there's an in-progress module we record the
            // instance in the index space. Otherwise this is the final module
            // and we return the items out.
            //
            // Note that in all cases we return the raw instance handle to get
            // the start function executed by the outer context.
            None => {
                let instance = self.instantiate_raw()?;
                let items = self.runtime_instance(&instance);
                let items = match self.in_progress.pop() {
                    Some(imports) => {
                        self.cur = imports;
                        self.cur.instances.push(items);
                        None
                    }
                    None => Some(items),
                };
                return Ok(Some((instance, items)));
            }
        }

        Ok(None)
    }

    fn instantiate_raw(&self) -> Result<StoreInstanceHandle> {
        let compiled_module = self.cur.module.compiled_module();

        // Register the module just before instantiation to ensure we keep the module
        // properly referenced while in use by the store.
        self.store.modules().borrow_mut().register(&self.cur.module);

        unsafe {
            let engine = self.store.engine();
            let allocator = engine.allocator();

            let instance = allocator.allocate(InstanceAllocationRequest {
                module: compiled_module.module().clone(),
                finished_functions: compiled_module.finished_functions(),
                imports: self.cur.build(),
                shared_signatures: self.cur.module.signatures().as_module_map().into(),
                host_state: Box::new(()),
                interrupts: self.store.interrupts(),
                externref_activations_table: self.store.externref_activations_table()
                    as *const VMExternRefActivationsTable
                    as *mut _,
                module_info_lookup: Some(self.store.module_info_lookup()),
                limiter: self.store.limiter().as_ref(),
            })?;

            // After we've created the `InstanceHandle` we still need to run
            // initialization to set up data/elements/etc. We do this after adding
            // the `InstanceHandle` to the store though. This is required for safety
            // because the start function (for example) may trap, but element
            // initializers may have run which placed elements into other instance's
            // tables. This means that from this point on, regardless of whether
            // initialization is successful, we need to keep the instance alive.
            let instance = self.store.add_instance(instance, false);
            allocator
                .initialize(&instance.handle, engine.config().features.bulk_memory)
                .map_err(|e| -> Error {
                    match e {
                        InstantiationError::Trap(trap) => {
                            Trap::from_runtime(self.store, trap).into()
                        }
                        other => other.into(),
                    }
                })?;

            Ok(instance)
        }
    }

    fn start_raw(instance: &StoreInstanceHandle) -> Result<()> {
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
                super::func::invoke_wasm_and_catch_traps(&instance.store, || {
                    mem::transmute::<
                        *const VMFunctionBody,
                        unsafe extern "C" fn(*mut VMContext, *mut VMContext),
                    >(f.anyfunc.as_ref().func_ptr.as_ptr())(
                        f.anyfunc.as_ref().vmctx, vmctx_ptr
                    )
                })?;
            }
        }
        Ok(())
    }

    fn runtime_instance(&self, instance: &StoreInstanceHandle) -> RuntimeInstance {
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
                        wasmtime_runtime::Export::Instance(self.cur.instances[*i].clone())
                    }
                    EntityIndex::Module(i) => {
                        wasmtime_runtime::Export::Module(Box::new(self.cur.modules[*i].clone()))
                    }
                    index => instance.handle.lookup_by_declaration(index),
                };
                (name.clone(), item)
            })
            .collect();
        Rc::new(exports)
    }
}

impl<'a> ImportsBuilder<'a> {
    fn new(module: &Module, src: ImportSource<'a>) -> ImportsBuilder<'a> {
        let raw = module.compiled_module().module();
        ImportsBuilder {
            src,
            functions: PrimaryMap::with_capacity(raw.num_imported_funcs),
            tables: PrimaryMap::with_capacity(raw.num_imported_tables),
            memories: PrimaryMap::with_capacity(raw.num_imported_memories),
            globals: PrimaryMap::with_capacity(raw.num_imported_globals),
            instances: PrimaryMap::with_capacity(raw.instances.len()),
            modules: PrimaryMap::with_capacity(raw.modules.len()),
            module: module.clone(),
            initializer: 0,
        }
    }

    fn push(&mut self, item: &Extern) {
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
                self.instances.push(i.items.clone());
            }
            Extern::Module(m) => {
                self.modules.push(m.clone());
            }
        }
    }

    fn build(&self) -> Imports<'_> {
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
