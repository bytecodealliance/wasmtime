use crate::store::{InstanceId, StoreData, StoreOpaque, StoreOpaqueSend, Stored};
use crate::types::matching;
use crate::{
    AsContext, AsContextMut, Engine, Export, Extern, Func, Global, InstanceType, Memory, Module,
    StoreContextMut, Table, Trap, TypedFunc,
};
use anyhow::{anyhow, bail, Context, Error, Result};
use std::mem;
use std::sync::Arc;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::wasm::{
    EntityIndex, FuncIndex, GlobalIndex, InstanceIndex, MemoryIndex, ModuleIndex, TableIndex,
};
use wasmtime_environ::Initializer;
use wasmtime_runtime::{
    Imports, InstanceAllocationRequest, InstantiationError, VMContext, VMFunctionBody,
    VMFunctionImport, VMGlobalImport, VMMemoryImport, VMTableImport,
};

/// An instantiated WebAssembly module.
///
/// This type represents the instantiation of a [`Module`]. Once instantiated
/// you can access the [`exports`](Instance::exports) which are of type
/// [`Extern`] and provide the ability to call functions, set globals, read
/// memory, etc. When interacting with any wasm code you'll want to make an
/// [`Instance`] to call any code or execute anything.
///
/// Instances are owned by a [`Store`](crate::Store) which is passed in at
/// creation time. It's recommended to create instances with
/// [`Linker::instantiate`](crate::Linker::instantiate) or similar
/// [`Linker`](crate::Linker) methods, but a more low-level constructor is also
/// available as [`Instance::new`].
#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct Instance(Stored<RuntimeInstance>);

impl Instance {
    /// Creates a new [`Instance`] from the previously compiled [`Module`] and
    /// list of `imports` specified.
    ///
    /// This method instantiates the `module` provided with the `imports`,
    /// following the procedure in the [core specification][inst] to
    /// instantiate. Instantiation can fail for a number of reasons (many
    /// specified below), but if successful the `start` function will be
    /// automatically run (if specified in the `module`) and then the
    /// [`Instance`] will be returned.
    ///
    /// Per the WebAssembly spec, instantiation includes running the module's
    /// start function, if it has one (not to be confused with the `_start`
    /// function, which is not run).
    ///
    /// Note that this is a low-level function that just performs an
    /// instantiation. See the [`Linker`](crate::Linker) struct for an API which
    /// provides a convenient way to link imports and provides automatic Command
    /// and Reactor behavior.
    ///
    /// ## Providing Imports
    ///
    /// The entries in the list of `imports` are intended to correspond 1:1
    /// with the list of imports returned by [`Module::imports`]. Before
    /// calling [`Instance::new`] you'll want to inspect the return value of
    /// [`Module::imports`] and, for each import type, create an [`Extern`]
    /// which corresponds to that type.  These [`Extern`] values are all then
    /// collected into a list and passed to this function.
    ///
    /// Note that this function is intentionally relatively low level. For an
    /// easier time passing imports by doing name-based resolution it's
    /// recommended to instead use the [`Linker`](crate::Linker) type.
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
    /// [`asynchronous config`](crate::Config::async_support). This function
    /// will also panic if any [`Extern`] supplied is not owned by `store`.
    ///
    /// [inst]: https://webassembly.github.io/spec/core/exec/modules.html#exec-instantiation
    /// [`ExternType`]: crate::ExternType
    pub fn new(
        mut store: impl AsContextMut,
        module: &Module,
        imports: &[Extern],
    ) -> Result<Instance, Error> {
        Instance::_new(&mut store.as_context_mut().opaque(), module, imports)
    }

    fn _new(
        store: &mut StoreOpaque<'_>,
        module: &Module,
        imports: &[Extern],
    ) -> Result<Instance, Error> {
        assert!(
            !store.async_support(),
            "cannot use `new` when async support is enabled on the config"
        );

        // NB: this is the same code as `Instance::new_async`. It's intentionally
        // small but should be kept in sync (modulo the async bits).
        let mut i = Instantiator::new(store, module, imports)?;
        loop {
            if let Some((id, instance)) = i.step(store)? {
                if let Some(start) = store.instance(id).module().start_func {
                    Instantiator::start_raw(store, id, start)?;
                }
                if let Some(instance) = instance {
                    break Ok(instance);
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
    /// This function will panic if called with a store associated with a
    /// [`synchronous config`](crate::Config::new). This is only compatible with
    /// stores associated with an [`asynchronous
    /// config`](crate::Config::async_support).
    ///
    /// This function will also panic, like [`Instance::new`], if any [`Extern`]
    /// specified does not belong to `store`.
    #[cfg(feature = "async")]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "async")))]
    pub async fn new_async<T>(
        mut store: impl AsContextMut<Data = T>,
        module: &Module,
        imports: &[Extern],
    ) -> Result<Instance, Error>
    where
        T: Send,
    {
        Instance::_new_async(store.as_context_mut().opaque_send(), module, imports).await
    }

    #[cfg(feature = "async")]
    async fn _new_async<'a>(
        mut store: StoreOpaqueSend<'a>,
        module: &Module,
        imports: &[Extern],
    ) -> Result<Instance, Error> {
        assert!(
            store.async_support(),
            "cannot use `new_async` without enabling async support on the config"
        );

        // NB: this is the same code as `Instance::new`. It's intentionally
        // small but should be kept in sync (modulo the async bits).
        let mut i = Instantiator::new(&mut store.opaque(), module, imports)?;
        loop {
            let step = i.step(&mut store.opaque())?;
            if let Some((id, instance)) = step {
                let start = store.instance(id).module().start_func;
                if let Some(start) = start {
                    store
                        .on_fiber(|store| Instantiator::start_raw(store, id, start))
                        .await??;
                }
                if let Some(instance) = instance {
                    break Ok(instance);
                }
            }
        }
    }

    pub(crate) fn from_wasmtime(handle: RuntimeInstance, store: &mut StoreOpaque) -> Instance {
        Instance(store.store_data_mut().insert(handle))
    }

    /// Returns the type signature of this instance.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this instance.
    pub fn ty(&self, store: impl AsContext) -> InstanceType {
        let store = store.as_context();
        let items = &store[self.0];
        let mut ty = InstanceType::new();
        for (name, item) in items.iter() {
            ty.add_named_export(name, item.ty(&store));
        }
        ty
    }

    pub(crate) fn items<'a>(&self, store: &'a StoreData) -> &'a RuntimeInstance {
        &store[self.0]
    }

    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        store.store_data().contains(self.0)
    }

    /// Returns the list of exported items from this [`Instance`].
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this instance.
    pub fn exports<'a, T: 'a>(
        &'a self,
        store: impl Into<StoreContextMut<'a, T>>,
    ) -> impl ExactSizeIterator<Item = Export<'a>> + 'a {
        let items = &store.into().store_data()[self.0];
        items
            .iter()
            .map(|(name, item)| Export::new(name, item.clone()))
    }

    /// Looks up an exported [`Extern`] value by name.
    ///
    /// This method will search the module for an export named `name` and return
    /// the value, if found.
    ///
    /// Returns `None` if there was no export named `name`.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this instance.
    pub fn get_export(&self, store: impl AsContextMut, name: &str) -> Option<Extern> {
        let store = store.as_context();
        store[self.0].get(name).cloned()
    }

    /// Looks up an exported [`Func`] value by name.
    ///
    /// Returns `None` if there was no export named `name`, or if there was but
    /// it wasn't a function.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this instance.
    pub fn get_func(&self, store: impl AsContextMut, name: &str) -> Option<Func> {
        self.get_export(store, name)?.into_func()
    }

    /// Looks up an exported [`Func`] value by name and with its type.
    ///
    /// This function is a convenience wrapper over [`Instance::get_func`] and
    /// [`Func::typed`]. For more information see the linked documentation.
    ///
    /// Returns an error if `name` isn't a function export or if the export's
    /// type did not match `Params` or `Results`
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this instance.
    pub fn get_typed_func<Params, Results, S>(
        &self,
        mut store: S,
        name: &str,
    ) -> Result<TypedFunc<Params, Results>>
    where
        Params: crate::WasmParams,
        Results: crate::WasmResults,
        S: AsContextMut,
    {
        let f = self
            .get_export(store.as_context_mut(), name)
            .and_then(|f| f.into_func())
            .ok_or_else(|| anyhow!("failed to find function export `{}`", name))?;
        Ok(f.typed::<Params, Results, _>(store)?)
    }

    /// Looks up an exported [`Table`] value by name.
    ///
    /// Returns `None` if there was no export named `name`, or if there was but
    /// it wasn't a table.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this instance.
    pub fn get_table(&self, store: impl AsContextMut, name: &str) -> Option<Table> {
        self.get_export(store, name)?.into_table()
    }

    /// Looks up an exported [`Memory`] value by name.
    ///
    /// Returns `None` if there was no export named `name`, or if there was but
    /// it wasn't a memory.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this instance.
    pub fn get_memory(&self, store: impl AsContextMut, name: &str) -> Option<Memory> {
        self.get_export(store, name)?.into_memory()
    }

    /// Looks up an exported [`Global`] value by name.
    ///
    /// Returns `None` if there was no export named `name`, or if there was but
    /// it wasn't a global.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this instance.
    pub fn get_global(&self, store: impl AsContextMut, name: &str) -> Option<Global> {
        self.get_export(store, name)?.into_global()
    }
}

struct Instantiator<'a> {
    in_progress: Vec<ImportsBuilder<'a>>,
    cur: ImportsBuilder<'a>,
}

struct ImportsBuilder<'a> {
    src: ImportSource<'a>,
    functions: PrimaryMap<FuncIndex, VMFunctionImport>,
    tables: PrimaryMap<TableIndex, VMTableImport>,
    memories: PrimaryMap<MemoryIndex, VMMemoryImport>,
    globals: PrimaryMap<GlobalIndex, VMGlobalImport>,
    instances: PrimaryMap<InstanceIndex, Instance>,
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
    fn new(
        store: &mut StoreOpaque<'_>,
        module: &Module,
        imports: &'a [Extern],
    ) -> Result<Instantiator<'a>> {
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
            if !import.comes_from_same_store(&store) {
                bail!("cross-`Store` instantiation is not currently supported");
            }
        }

        Ok(Instantiator {
            in_progress: Vec::new(),
            cur: ImportsBuilder::new(module, ImportSource::Runtime(imports)),
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
    fn step(
        &mut self,
        store: &mut StoreOpaque<'_>,
    ) -> Result<Option<(InstanceId, Option<Instance>)>> {
        if self.cur.initializer == 0 {
            store.bump_resource_counts(&self.cur.module)?;
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
                            store_data: store.store_data(),
                            engine: store.engine(),
                        }
                        .extern_(&expected_ty, head)
                        .with_context(|| {
                            let extra = match field {
                                Some(name) => format!("::{}", name),
                                None => String::new(),
                            };
                            format!("incompatible import type for `{}{}`", name, extra)
                        })?;
                        self.cur.push(head.clone(), store);
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
                let instance = self.cur.instances[*instance];
                let export = store[instance.0][export].clone();
                self.cur.push(export, store);
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
                let instance = self.instantiate_raw(store)?;
                let items = self.runtime_instance(store, instance);
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

    fn instantiate_raw(&mut self, store: &mut StoreOpaque<'_>) -> Result<InstanceId> {
        let compiled_module = self.cur.module.compiled_module();

        // Register the module just before instantiation to ensure we keep the module
        // properly referenced while in use by the store.
        store.modules_mut().register(&self.cur.module);

        unsafe {
            let mut instance = store
                .engine()
                .allocator()
                .allocate(InstanceAllocationRequest {
                    module: compiled_module.module().clone(),
                    finished_functions: compiled_module.finished_functions(),
                    imports: self.cur.build(),
                    shared_signatures: self.cur.module.signatures().as_module_map().into(),
                    host_state: Box::new(()),
                    store: Some(store.traitobj),
                })?;

            // After we've created the `InstanceHandle` we still need to run
            // initialization to set up data/elements/etc. We do this after
            // adding the `InstanceHandle` to the store though. This is required
            // for safety because the start function (for example) may trap, but
            // element initializers may have run which placed elements into
            // other instance's tables. This means that from this point on,
            // regardless of whether initialization is successful, we need to
            // keep the instance alive.
            //
            // Note that we `clone` the instance handle just to make easier
            // working the the borrow checker here easier. Technically the `&mut
            // instance` has somewhat of a borrow on `store` (which
            // conflicts with the borrow on `store.engine`) but this doesn't
            // matter in practice since initialization isn't even running any
            // code here anyway.
            let id = store.add_instance(instance.clone(), false);
            store
                .engine()
                .allocator()
                .initialize(
                    &mut instance,
                    compiled_module.module(),
                    store.engine().config().features.bulk_memory,
                )
                .map_err(|e| -> Error {
                    match e {
                        InstantiationError::Trap(trap) => Trap::from_runtime(trap).into(),
                        other => other.into(),
                    }
                })?;

            Ok(id)
        }
    }

    fn start_raw(
        store: &mut StoreOpaque<'_>,
        instance: InstanceId,
        start: FuncIndex,
    ) -> Result<()> {
        // If a start function is present, invoke it. Make sure we use all the
        // trap-handling configuration in `store` as well.
        let instance = store.instance(instance);
        let f = match instance.lookup_by_declaration(&EntityIndex::Function(start)) {
            wasmtime_runtime::Export::Function(f) => f,
            _ => unreachable!(), // valid modules shouldn't hit this
        };
        let vmctx = instance.vmctx_ptr();
        unsafe {
            super::func::invoke_wasm_and_catch_traps(store, |_default_callee| {
                mem::transmute::<
                    *const VMFunctionBody,
                    unsafe extern "C" fn(*mut VMContext, *mut VMContext),
                >(f.anyfunc.as_ref().func_ptr.as_ptr())(
                    f.anyfunc.as_ref().vmctx, vmctx
                )
            })?;
        }
        Ok(())
    }

    fn runtime_instance(&mut self, store: &mut StoreOpaque<'_>, instance: InstanceId) -> Instance {
        // We use an unsafe `clone()` here to work around the borrow checker.
        // Technically our instance is a borrow of `store`, but we need the
        // borrow again later when calling `Extern::from_wasmtime_export` (and a
        // mutable one at that).
        //
        // The mutability in `from_wasmtime_export` only mutates `StoreData`
        // since we're adding ids, but it definitely doesn't deallocate
        // `instance` (nothing does that except `Drop` for `Store`), so this in
        // theory should be safe.
        let instance = unsafe { store.instance(instance).clone() };

        // FIXME(#2916) we should ideally just store the `InstanceId` within the
        // store itself. There should be no reason we have to allocate a hash
        // map here and allocate a bunch of strings, that's quite wasteful if
        // only one or two exports are used. Additionally this can push items
        // into the `Store` which never end up getting used.
        let exports = instance
            .module()
            .exports
            .iter()
            .map(|(name, index)| {
                // Note that instances and modules are not handled by
                // `wasmtime_runtime`, they're handled by us in this crate. That
                // means we need to handle that here, otherwise we defer to the
                // instance to load the values.
                let item = match index {
                    EntityIndex::Instance(i) => Extern::Instance(self.cur.instances[*i].clone()),
                    EntityIndex::Module(i) => Extern::Module(self.cur.modules[*i].clone()),
                    index => unsafe {
                        Extern::from_wasmtime_export(instance.lookup_by_declaration(index), store)
                    },
                };
                (name.clone(), item)
            })
            .collect();
        Instance::from_wasmtime(Arc::new(exports), store)
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

    fn push(&mut self, item: Extern, store: &mut StoreOpaque<'_>) {
        match item {
            Extern::Func(i) => {
                self.functions.push(i.vmimport(store));
            }
            Extern::Global(i) => {
                self.globals.push(i.vmimport(store));
            }
            Extern::Table(i) => {
                self.tables.push(i.vmimport(store));
            }
            Extern::Memory(i) => {
                self.memories.push(i.vmimport(store));
            }
            Extern::Instance(i) => {
                self.instances.push(i);
            }
            Extern::Module(m) => {
                self.modules.push(m);
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

pub(crate) type RuntimeInstance = Arc<indexmap::IndexMap<String, Extern>>;

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
        let items = Arc::get_mut(&mut self.items).unwrap();
        items.insert(name.to_string(), item.into());
    }

    pub(crate) fn finish(self, store: &mut StoreOpaque) -> Instance {
        Instance::from_wasmtime(self.items, store)
    }
}
