use crate::linker::Definition;
use crate::store::{InstanceId, StoreOpaque, Stored};
use crate::types::matching;
use crate::{
    AsContextMut, Engine, Export, Extern, Func, Global, Memory, Module, SharedMemory,
    StoreContextMut, Table, Trap, TypedFunc,
};
use anyhow::{anyhow, bail, Context, Error, Result};
use std::mem;
use std::sync::Arc;
use wasmtime_environ::{EntityType, FuncIndex, GlobalIndex, MemoryIndex, PrimaryMap, TableIndex};
use wasmtime_runtime::{
    Imports, InstanceAllocationRequest, InstantiationError, StorePtr, VMContext, VMFunctionBody,
    VMFunctionImport, VMGlobalImport, VMMemoryImport, VMOpaqueContext, VMTableImport,
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
        let mut store = store.as_context_mut();
        let imports = Instance::typecheck_externs(store.0, module, imports)?;
        // Note that the unsafety here should be satisfied by the call to
        // `typecheck_externs` above which satisfies the condition that all
        // the imports are valid for this module.
        unsafe { Instance::new_started(&mut store, module, imports.as_ref()) }
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
        let mut store = store.as_context_mut();
        let imports = Instance::typecheck_externs(store.0, module, imports)?;
        // See `new` for notes on this unsafety
        unsafe { Instance::new_started_async(&mut store, module, imports.as_ref()).await }
    }

    fn typecheck_externs(
        store: &mut StoreOpaque,
        module: &Module,
        imports: &[Extern],
    ) -> Result<OwnedImports> {
        for import in imports {
            if !import.comes_from_same_store(store) {
                bail!("cross-`Store` instantiation is not currently supported");
            }
        }
        typecheck(store, module, imports, |cx, ty, item| cx.extern_(ty, item))?;
        let mut owned_imports = OwnedImports::new(module);
        for import in imports {
            owned_imports.push(import, store);
        }
        Ok(owned_imports)
    }

    /// Internal function to create an instance and run the start function.
    ///
    /// This function's unsafety is the same as `Instance::new_raw`.
    pub(crate) unsafe fn new_started<T>(
        store: &mut StoreContextMut<'_, T>,
        module: &Module,
        imports: Imports<'_>,
    ) -> Result<Instance> {
        assert!(
            !store.0.async_support(),
            "must use async instantiation when async support is enabled",
        );

        let (instance, start) = Instance::new_raw(store.0, module, imports)?;
        if let Some(start) = start {
            instance.start_raw(store, start)?;
        }
        Ok(instance)
    }

    /// Internal function to create an instance and run the start function.
    ///
    /// This function's unsafety is the same as `Instance::new_raw`.
    #[cfg(feature = "async")]
    async unsafe fn new_started_async<T>(
        store: &mut StoreContextMut<'_, T>,
        module: &Module,
        imports: Imports<'_>,
    ) -> Result<Instance>
    where
        T: Send,
    {
        // Note that the body of this function is intentionally quite similar
        // to the `new_started` function, and it's intended that the two bodies
        // here are small enough to be ok duplicating.
        assert!(
            store.0.async_support(),
            "must use sync instantiation when async support is disabled",
        );

        store
            .on_fiber(|store| {
                let (instance, start) = Instance::new_raw(store.0, module, imports)?;
                if let Some(start) = start {
                    instance.start_raw(store, start)?;
                }
                Ok(instance)
            })
            .await?
    }

    /// Internal function to create an instance which doesn't have its `start`
    /// function run yet.
    ///
    /// This is not intended to be exposed from Wasmtime, it's intended to
    /// refactor out common code from `new_started` and `new_started_async`.
    ///
    /// Note that this step needs to be run on a fiber in async mode even
    /// though it doesn't do any blocking work because an async resource
    /// limiter may need to yield.
    ///
    /// # Unsafety
    ///
    /// This method is unsafe because it does not type-check the `imports`
    /// provided. The `imports` provided must be suitable for the module
    /// provided as well.
    unsafe fn new_raw(
        store: &mut StoreOpaque,
        module: &Module,
        imports: Imports<'_>,
    ) -> Result<(Instance, Option<FuncIndex>)> {
        if !Engine::same(store.engine(), module.engine()) {
            bail!("cross-`Engine` instantiation is not currently supported");
        }
        store.bump_resource_counts(module)?;

        let compiled_module = module.compiled_module();

        // Register the module just before instantiation to ensure we keep the module
        // properly referenced while in use by the store.
        store.modules_mut().register_module(module);

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
                    runtime_info: &module.runtime_info(),
                    imports,
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
            let exports = vec![None; compiled_module.module().exports.len()];
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
                    InstantiationError::Trap(trap) => Trap::new_wasm(trap, None).into(),
                    other => other.into(),
                }
            })?;

        Ok((instance, compiled_module.module().start_func))
    }

    pub(crate) fn from_wasmtime(handle: InstanceData, store: &mut StoreOpaque) -> Instance {
        Instance(store.store_data_mut().insert(handle))
    }

    fn start_raw<T>(&self, store: &mut StoreContextMut<'_, T>, start: FuncIndex) -> Result<()> {
        let id = store.0.store_data()[self.0].id;
        // If a start function is present, invoke it. Make sure we use all the
        // trap-handling configuration in `store` as well.
        let instance = store.0.instance_mut(id);
        let f = instance.get_exported_func(start);
        let vmctx = instance.vmctx_ptr();
        unsafe {
            super::func::invoke_wasm_and_catch_traps(store, |_default_callee| {
                mem::transmute::<
                    *const VMFunctionBody,
                    unsafe extern "C" fn(*mut VMOpaqueContext, *mut VMContext),
                >(f.anyfunc.as_ref().func_ptr.as_ptr())(
                    f.anyfunc.as_ref().vmctx, vmctx
                )
            })?;
        }
        Ok(())
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
            unsafe { Extern::from_wasmtime_export(instance.get_export_by_index(index), store) };
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

    /// Looks up an exported [`SharedMemory`] value by name.
    ///
    /// Returns `None` if there was no export named `name`, or if there was but
    /// it wasn't a shared memory.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this instance.
    pub fn get_shared_memory(
        &self,
        mut store: impl AsContextMut,
        name: &str,
    ) -> Option<SharedMemory> {
        let mut store = store.as_context_mut();
        self.get_export(&mut store, name)?.into_shared_memory()
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

    #[cfg(feature = "component-model")]
    pub(crate) fn id(&self, store: &StoreOpaque) -> InstanceId {
        store[self.0].id
    }
}

pub(crate) struct OwnedImports {
    functions: PrimaryMap<FuncIndex, VMFunctionImport>,
    tables: PrimaryMap<TableIndex, VMTableImport>,
    memories: PrimaryMap<MemoryIndex, VMMemoryImport>,
    globals: PrimaryMap<GlobalIndex, VMGlobalImport>,
}

impl OwnedImports {
    fn new(module: &Module) -> OwnedImports {
        let mut ret = OwnedImports::empty();
        ret.reserve(module);
        return ret;
    }

    pub(crate) fn empty() -> OwnedImports {
        OwnedImports {
            functions: PrimaryMap::new(),
            tables: PrimaryMap::new(),
            memories: PrimaryMap::new(),
            globals: PrimaryMap::new(),
        }
    }

    pub(crate) fn reserve(&mut self, module: &Module) {
        let raw = module.compiled_module().module();
        self.functions.reserve(raw.num_imported_funcs);
        self.tables.reserve(raw.num_imported_tables);
        self.memories.reserve(raw.num_imported_memories);
        self.globals.reserve(raw.num_imported_globals);
    }

    #[cfg(feature = "component-model")]
    pub(crate) fn clear(&mut self) {
        self.functions.clear();
        self.tables.clear();
        self.memories.clear();
        self.globals.clear();
    }

    fn push(&mut self, item: &Extern, store: &mut StoreOpaque) {
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
            Extern::SharedMemory(i) => {
                self.memories.push(i.vmimport(store));
            }
        }
    }

    /// Note that this is unsafe as the validity of `item` is not verified and
    /// it contains a bunch of raw pointers.
    #[cfg(feature = "component-model")]
    pub(crate) unsafe fn push_export(&mut self, item: &wasmtime_runtime::Export) {
        match item {
            wasmtime_runtime::Export::Function(f) => {
                let f = f.anyfunc.as_ref();
                self.functions.push(VMFunctionImport {
                    body: f.func_ptr,
                    vmctx: f.vmctx,
                });
            }
            wasmtime_runtime::Export::Global(g) => {
                self.globals.push(VMGlobalImport { from: g.definition });
            }
            wasmtime_runtime::Export::Table(t) => {
                self.tables.push(VMTableImport {
                    from: t.definition,
                    vmctx: t.vmctx,
                });
            }
            wasmtime_runtime::Export::Memory(m) => {
                self.memories.push(VMMemoryImport {
                    from: m.definition,
                    vmctx: m.vmctx,
                    index: m.index,
                });
            }
        }
    }

    pub(crate) fn as_ref(&self) -> Imports<'_> {
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

    /// The items which this `InstancePre` use to instantiate the `module`
    /// provided, passed to `Instance::new_started` after inserting them into a
    /// `Store`.
    ///
    /// Note that this is stored as an `Arc<[T]>` to quickly move a strong
    /// reference to everything internally into a `Store<T>` without having to
    /// clone each individual item.
    items: Arc<[Definition]>,

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
    /// Creates a new `InstancePre` which type-checks the `items` provided and
    /// on success is ready to instantiate a new instance.
    ///
    /// # Unsafety
    ///
    /// This method is unsafe as the `T` of the `InstancePre<T>` is not
    /// guaranteed to be the same as the `T` within the `Store`, the caller must
    /// verify that.
    pub(crate) unsafe fn new(
        store: &mut StoreOpaque,
        module: &Module,
        items: Vec<Definition>,
    ) -> Result<InstancePre<T>> {
        for import in items.iter() {
            if !import.comes_from_same_store(store) {
                bail!("cross-`Store` instantiation is not currently supported");
            }
        }
        typecheck(store, module, &items, |cx, ty, item| {
            cx.definition(ty, item)
        })?;

        let host_funcs = items
            .iter()
            .filter(|i| match i {
                Definition::HostFunc(_) => true,
                _ => false,
            })
            .count();
        Ok(InstancePre {
            module: module.clone(),
            items: items.into(),
            host_funcs,
            _marker: std::marker::PhantomData,
        })
    }

    /// Returns a reference to the module that this [`InstancePre`] will be
    /// instantiating.
    pub fn module(&self) -> &Module {
        &self.module
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
        let mut store = store.as_context_mut();
        let imports =
            pre_instantiate_raw(&mut store.0, &self.module, &self.items, self.host_funcs)?;

        // This unsafety should be handled by the type-checking performed by the
        // constructor of `InstancePre` to assert that all the imports we're passing
        // in match the module we're instantiating.
        unsafe { Instance::new_started(&mut store, &self.module, imports.as_ref()) }
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
        let mut store = store.as_context_mut();
        let imports =
            pre_instantiate_raw(&mut store.0, &self.module, &self.items, self.host_funcs)?;

        // This unsafety should be handled by the type-checking performed by the
        // constructor of `InstancePre` to assert that all the imports we're passing
        // in match the module we're instantiating.
        unsafe { Instance::new_started_async(&mut store, &self.module, imports.as_ref()).await }
    }
}

/// Helper function shared between
/// `InstancePre::{instantiate,instantiate_async}`
///
/// This is an out-of-line function to avoid the generic on `InstancePre` and
/// get this compiled into the `wasmtime` crate to avoid having it monomorphized
/// elsewhere.
fn pre_instantiate_raw(
    store: &mut StoreOpaque,
    module: &Module,
    items: &Arc<[Definition]>,
    host_funcs: usize,
) -> Result<OwnedImports> {
    if host_funcs > 0 {
        // Any linker-defined function of the `Definition::HostFunc` variant
        // will insert a function into the store automatically as part of
        // instantiation, so reserve space here to make insertion more efficient
        // as it won't have to realloc during the instantiation.
        store.store_data_mut().reserve_funcs(host_funcs);

        // The usage of `to_extern_store_rooted` requires that the items are
        // rooted via another means, which happens here by cloning the list of
        // items into the store once. This avoids cloning each individual item
        // below.
        store.push_rooted_funcs(items.clone());
    }

    let mut imports = OwnedImports::new(module);
    for import in items.iter() {
        if !import.comes_from_same_store(store) {
            bail!("cross-`Store` instantiation is not currently supported");
        }
        // This unsafety should be encapsulated in the constructor of
        // `InstancePre` where the `T` of the original item should match the
        // `T` of the store. Additionally the rooting necessary has happened
        // above.
        let item = unsafe { import.to_extern_store_rooted(store) };
        imports.push(&item, store);
    }

    Ok(imports)
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
