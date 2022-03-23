use crate::linker::Definition;
use crate::store::{InstanceId, StoreOpaque, Stored};
use crate::types::matching;
use crate::{
    AsContextMut, Engine, Export, Extern, Func, Global, Memory, Module, StoreContextMut, Table,
    Trap, TypedFunc,
};
use anyhow::{anyhow, bail, Context, Error, Result};
use std::mem;
use std::sync::Arc;
use wasmtime_environ::{
    EntityIndex, EntityType, FuncIndex, GlobalIndex, MemoryIndex, PrimaryMap, TableIndex,
};
use wasmtime_runtime::{
    Imports, InstanceAllocationRequest, InstantiationError, StorePtr, VMContext, VMFunctionBody,
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
pub struct Instance(Stored<InstanceData>);

pub(crate) struct InstanceData {
    /// The id of the instance within the store, used to find the original
    /// `InstanceHandle`.
    id: InstanceId,
    /// A lazily-populated list of exports of this instance. The order of
    /// exports here matches the order of the exports in the the original
    /// module.
    exports: Vec<Option<Extern>>,
}

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
        // This unsafety comes from `Instantiator::new` where we must typecheck
        // first, which we are sure to do here.
        let mut store = store.as_context_mut();
        let mut i = unsafe {
            typecheck_externs(store.0, module, imports)?;
            Instantiator::new(store.0, module, ImportSource::Externs(imports))?
        };
        assert!(
            !store.0.async_support(),
            "cannot use `new` when async support is enabled on the config"
        );

        i.run(&mut store)
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
        // See `new` for unsafety comments
        let mut store = store.as_context_mut();
        let mut i = unsafe {
            typecheck_externs(store.0, module, imports)?;
            Instantiator::new(store.0, module, ImportSource::Externs(imports))?
        };
        let mut store = store.as_context_mut();
        assert!(
            store.0.async_support(),
            "cannot use `new_async` without enabling async support on the config"
        );
        store
            .on_fiber(|store| i.run(&mut store.as_context_mut()))
            .await?
    }

    pub(crate) fn from_wasmtime(handle: InstanceData, store: &mut StoreOpaque) -> Instance {
        Instance(store.store_data_mut().insert(handle))
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
        self._exports(store.into().0)
    }

    fn _exports<'a>(
        &'a self,
        store: &'a mut StoreOpaque,
    ) -> impl ExactSizeIterator<Item = Export<'a>> + 'a {
        // If this is an `Instantiated` instance then all the `exports` may not
        // be filled in. Fill them all in now if that's the case.
        let InstanceData { exports, id, .. } = &store[self.0];
        if exports.iter().any(|e| e.is_none()) {
            let module = Arc::clone(store.instance(*id).module());
            for name in module.exports.keys() {
                self._get_export(store, name);
            }
        }

        let data = &store.store_data()[self.0];
        let module = store.instance(data.id).module();
        module
            .exports
            .iter()
            .zip(&data.exports)
            .map(|((name, _), export)| Export::new(name, export.clone().unwrap()))
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
    ///
    /// # Why does `get_export` take a mutable context?
    ///
    /// This method requires a mutable context because an instance's exports are
    /// lazily populated, and we cache them as they are accessed. This makes
    /// instantiating a module faster, but also means this method requires a
    /// mutable context.
    pub fn get_export(&self, mut store: impl AsContextMut, name: &str) -> Option<Extern> {
        self._get_export(store.as_context_mut().0, name)
    }

    fn _get_export(&self, store: &mut StoreOpaque, name: &str) -> Option<Extern> {
        // Instantiated instances will lazily fill in exports, so we process
        // all that lazy logic here.
        let data = &store[self.0];

        let instance = store.instance(data.id);
        let (i, _, &index) = instance.module().exports.get_full(name)?;
        if let Some(export) = &data.exports[i] {
            return Some(export.clone());
        }

        let id = data.id;
        let instance = store.instance_mut(id); // reborrow the &mut Instancehandle
        let item =
            unsafe { Extern::from_wasmtime_export(instance.lookup_by_declaration(&index), store) };
        let data = &mut store[self.0];
        data.exports[i] = Some(item.clone());
        Some(item)
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
        Ok(f.typed::<Params, Results, _>(store)
            .with_context(|| format!("failed to convert function `{}` to given type", name))?)
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
    imports: ImportSource<'a>,
    functions: PrimaryMap<FuncIndex, VMFunctionImport>,
    tables: PrimaryMap<TableIndex, VMTableImport>,
    memories: PrimaryMap<MemoryIndex, VMMemoryImport>,
    globals: PrimaryMap<GlobalIndex, VMGlobalImport>,
    module: &'a Module,
}

enum ImportSource<'a> {
    Externs(&'a [Extern]),
    Definitions(&'a [Definition]),
}

impl<'a> Instantiator<'a> {
    /// Creates a new instantiation context used to process all the initializer
    /// directives of a module.
    ///
    /// This doesn't do much work itself beyond setting things up.
    ///
    /// # Unsafety
    ///
    /// This function is unsafe for a few reasons:
    ///
    /// * This assumes that `imports` has already been typechecked and is of the
    ///   appropriate length. It is memory unsafe if the types of `imports` are
    ///   not what `module` expects.
    ///
    /// * The `imports` must be safely able to get inserted into `store`. This
    ///   only applies if `ImportSource::Definitions` is used because this will
    ///   internally call `Definition::to_extern` which requires that any
    ///   host functions in the list were created with an original `T` as the
    ///   store that's being inserted into.
    ///
    /// * The `imports` must all come from the `store` specified.
    unsafe fn new(
        store: &StoreOpaque,
        module: &'a Module,
        imports: ImportSource<'a>,
    ) -> Result<Instantiator<'a>> {
        if !Engine::same(store.engine(), module.engine()) {
            bail!("cross-`Engine` instantiation is not currently supported");
        }

        let raw = module.compiled_module().module();
        Ok(Instantiator {
            imports,
            functions: PrimaryMap::with_capacity(raw.num_imported_funcs),
            tables: PrimaryMap::with_capacity(raw.num_imported_tables),
            memories: PrimaryMap::with_capacity(raw.num_imported_memories),
            globals: PrimaryMap::with_capacity(raw.num_imported_globals),
            module,
        })
    }

    fn run<T>(&mut self, store: &mut StoreContextMut<'_, T>) -> Result<Instance, Error> {
        let (instance, start) = self.resolve_imports(store.0)?;
        if let Some(start) = start {
            Instantiator::start_raw(store, instance, start)?;
        }
        Ok(instance)
    }

    /// Resolve all the imports for the module being instantiated, extracting
    /// the raw representations and building up the `PrimaryMap` instance for
    /// each set of exports.
    fn resolve_imports(
        &mut self,
        store: &mut StoreOpaque,
    ) -> Result<(Instance, Option<FuncIndex>)> {
        store.bump_resource_counts(&self.module)?;

        // Read the current module's initializer and move forward the
        // initializer pointer as well.
        let num_imports = self.module.env_module().initializers.len();
        match &self.imports {
            ImportSource::Externs(list) => {
                assert_eq!(list.len(), num_imports);
                for item in list.iter() {
                    self.push(item.clone(), store);
                }
            }
            ImportSource::Definitions(list) => {
                assert_eq!(list.len(), num_imports);
                for item in list.iter() {
                    // This unsafety is encapsulated with
                    // `Instantiator::new`, documented above.
                    self.push(unsafe { item.to_extern(store) }, store);
                }
            }
        }

        // All initializers have been processed, which means we're ready to
        // perform the actual raw instantiation with the raw import values. This
        // will register everything except the start function's completion and
        // the finished instance will be returned.
        self.instantiate_raw(store)
    }

    fn instantiate_raw(
        &mut self,
        store: &mut StoreOpaque,
    ) -> Result<(Instance, Option<FuncIndex>)> {
        let compiled_module = self.module.compiled_module();

        // Register the module just before instantiation to ensure we keep the module
        // properly referenced while in use by the store.
        store.modules_mut().register(&self.module);

        unsafe {
            // The first thing we do is issue an instance allocation request
            // to the instance allocator. This, on success, will give us an
            // instance handle.
            //
            // Note that the `host_state` here is a pointer back to the
            // `Instance` we'll be returning from this function. This is a
            // circular reference so we can't construct it before we construct
            // this instance, so we determine what the ID is and then assert
            // it's the same later when we do actually insert it.
            let instance_to_be = store.store_data().next_id::<InstanceData>();

            let mut instance_handle =
                store
                    .engine()
                    .allocator()
                    .allocate(InstanceAllocationRequest {
                        runtime_info: &self.module.runtime_info(),
                        imports: self.build(),
                        host_state: Box::new(Instance(instance_to_be)),
                        store: StorePtr::new(store.traitobj()),
                    })?;

            // The instance still has lots of setup, for example
            // data/elements/start/etc. This can all fail, but even on failure
            // the instance may persist some state via previous successful
            // initialization. For this reason once we have an instance handle
            // we immediately insert it into the store to keep it alive.
            //
            // Note that we `clone` the instance handle just to make easier
            // working the the borrow checker here easier. Technically the `&mut
            // instance` has somewhat of a borrow on `store` (which
            // conflicts with the borrow on `store.engine`) but this doesn't
            // matter in practice since initialization isn't even running any
            // code here anyway.
            let id = store.add_instance(instance_handle.clone(), false);

            // Additionally, before we start doing fallible instantiation, we
            // do one more step which is to insert an `InstanceData`
            // corresponding to this instance. This `InstanceData` can be used
            // via `Caller::get_export` if our instance's state "leaks" into
            // other instances, even if we don't return successfully from this
            // function.
            //
            // We don't actually load all exports from the instance at this
            // time, instead preferring to lazily load them as they're demanded.
            // For module/instance exports, though, those aren't actually
            // stored in the instance handle so we need to immediately handle
            // those here.
            let instance = {
                let exports = compiled_module
                    .module()
                    .exports
                    .values()
                    .map(|_index| None)
                    .collect();
                let data = InstanceData { id, exports };
                Instance::from_wasmtime(data, store)
            };

            // double-check our guess of what the new instance's ID would be
            // was actually correct.
            assert_eq!(instance.0, instance_to_be);

            // Now that we've recorded all information we need to about this
            // instance within a `Store` we can start performing fallible
            // initialization. Note that we still defer the `start` function to
            // later since that may need to run asynchronously.
            //
            // If this returns an error (or if the start function traps) then
            // any other initialization which may have succeeded which placed
            // items from this instance into other instances should be ok when
            // those items are loaded and run we'll have all the metadata to
            // look at them.
            store
                .engine()
                .allocator()
                .initialize(
                    &mut instance_handle,
                    compiled_module.module(),
                    store.engine().config().features.bulk_memory,
                )
                .map_err(|e| -> Error {
                    match e {
                        InstantiationError::Trap(trap) => Trap::from_runtime(trap).into(),
                        other => other.into(),
                    }
                })?;

            Ok((instance, compiled_module.module().start_func))
        }
    }

    fn start_raw<T>(
        store: &mut StoreContextMut<'_, T>,
        instance: Instance,
        start: FuncIndex,
    ) -> Result<()> {
        let id = store.0.store_data()[instance.0].id;
        // If a start function is present, invoke it. Make sure we use all the
        // trap-handling configuration in `store` as well.
        let instance = store.0.instance_mut(id);
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

    fn push(&mut self, item: Extern, store: &mut StoreOpaque) {
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

/// An instance, pre-instantiation, that is ready to be instantiated.
///
/// This structure represents an instance *just before* it was instantiated,
/// after all type-checking and imports have been resolved. The only thing left
/// to do for this instance is to actually run the process of instantiation.
///
/// Note that an `InstancePre` may not be tied to any particular [`Store`] if
/// none of the imports it closed over are tied to any particular [`Store`].
///
/// This structure is created through the [`Linker::instantiate_pre`] method,
/// which also has some more information and examples.
///
/// [`Store`]: crate::Store
/// [`Linker::instantiate_pre`]: crate::Linker::instantiate_pre
pub struct InstancePre<T> {
    module: Module,
    items: Vec<Definition>,
    /// A count of `Definition::HostFunc` entries in `items` above to
    /// preallocate space in a `Store` up front for all entries to be inserted.
    host_funcs: usize,
    _marker: std::marker::PhantomData<fn() -> T>,
}

/// InstancePre's clone does not require T: Clone
impl<T> Clone for InstancePre<T> {
    fn clone(&self) -> Self {
        Self {
            module: self.module.clone(),
            items: self.items.clone(),
            host_funcs: self.host_funcs,
            _marker: self._marker,
        }
    }
}

impl<T> InstancePre<T> {
    pub(crate) unsafe fn new(
        store: &mut StoreOpaque,
        module: &Module,
        items: Vec<Definition>,
    ) -> Result<InstancePre<T>> {
        typecheck_defs(store, module, &items)?;
        let host_funcs = items
            .iter()
            .filter(|i| match i {
                Definition::HostFunc(_) => true,
                _ => false,
            })
            .count();
        Ok(InstancePre {
            module: module.clone(),
            items,
            host_funcs,
            _marker: std::marker::PhantomData,
        })
    }

    /// Instantiates this instance, creating a new instance within the provided
    /// `store`.
    ///
    /// This function will run the actual process of instantiation to
    /// completion. This will use all of the previously-closed-over items as
    /// imports to instantiate the module that this was originally created with.
    ///
    /// For more information about instantiation see [`Instance::new`].
    ///
    /// # Panics
    ///
    /// Panics if any import closed over by this [`InstancePre`] isn't owned by
    /// `store`, or if `store` has async support enabled. Additionally this
    /// function will panic if the `store` provided comes from a different
    /// [`Engine`] than the [`InstancePre`] originally came from.
    pub fn instantiate(&self, mut store: impl AsContextMut<Data = T>) -> Result<Instance> {
        // For the unsafety here the typecheck happened at creation time of this
        // structure and then othrewise the `T` of `InstancePre<T>` connects any
        // host functions we have in our definition list to the `store` that was
        // passed in.
        let mut store = store.as_context_mut();
        let mut instantiator = unsafe {
            self.verify_store_and_reserve_space(&mut store.0)?;
            Instantiator::new(
                store.0,
                &self.module,
                ImportSource::Definitions(&self.items),
            )?
        };
        instantiator.run(&mut store)
    }

    /// Creates a new instance, running the start function asynchronously
    /// instead of inline.
    ///
    /// For more information about asynchronous instantiation see the
    /// documentation on [`Instance::new_async`].
    ///
    /// # Panics
    ///
    /// Panics if any import closed over by this [`InstancePre`] isn't owned by
    /// `store`, or if `store` does not have async support enabled.
    #[cfg(feature = "async")]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "async")))]
    pub async fn instantiate_async(
        &self,
        mut store: impl AsContextMut<Data = T>,
    ) -> Result<Instance>
    where
        T: Send,
    {
        // For the unsafety here see above
        let mut store = store.as_context_mut();
        let mut i = unsafe {
            self.verify_store_and_reserve_space(&mut store.0)?;
            Instantiator::new(
                store.0,
                &self.module,
                ImportSource::Definitions(&self.items),
            )?
        };

        let mut store = store.as_context_mut();
        assert!(
            store.0.async_support(),
            "cannot use `new_async` without enabling async support on the config"
        );
        store
            .on_fiber(|store| i.run(&mut store.as_context_mut()))
            .await?
    }

    fn verify_store_and_reserve_space(&self, store: &mut StoreOpaque) -> Result<()> {
        for import in self.items.iter() {
            if !import.comes_from_same_store(store) {
                bail!("cross-`Store` instantiation is not currently supported");
            }
        }
        // Any linker-defined function of the `Definition::HostFunc` variant
        // will insert a function into the store automatically as part of
        // instantiation, so reserve space here to make insertion more efficient
        // as it won't have to realloc during the instantiation.
        store.store_data_mut().reserve_funcs(self.host_funcs);
        Ok(())
    }
}

fn typecheck_externs(store: &mut StoreOpaque, module: &Module, imports: &[Extern]) -> Result<()> {
    for import in imports {
        if !import.comes_from_same_store(store) {
            bail!("cross-`Store` instantiation is not currently supported");
        }
    }
    typecheck(store, module, imports, |cx, ty, item| cx.extern_(ty, item))
}

fn typecheck_defs(store: &mut StoreOpaque, module: &Module, imports: &[Definition]) -> Result<()> {
    for import in imports {
        if !import.comes_from_same_store(store) {
            bail!("cross-`Store` instantiation is not currently supported");
        }
    }
    typecheck(store, module, imports, |cx, ty, item| {
        cx.definition(ty, item)
    })
}

fn typecheck<I>(
    store: &mut StoreOpaque,
    module: &Module,
    imports: &[I],
    check: impl Fn(&matching::MatchCx<'_>, &EntityType, &I) -> Result<()>,
) -> Result<()> {
    let env_module = module.compiled_module().module();
    let expected = env_module.imports().count();
    if expected != imports.len() {
        bail!("expected {} imports, found {}", expected, imports.len());
    }
    let cx = matching::MatchCx {
        signatures: module.signatures(),
        types: module.types(),
        store: store,
        engine: store.engine(),
    };
    for ((name, field, expected_ty), actual) in env_module.imports().zip(imports) {
        check(&cx, &expected_ty, actual)
            .with_context(|| format!("incompatible import type for `{name}::{field}`"))?;
    }
    Ok(())
}
