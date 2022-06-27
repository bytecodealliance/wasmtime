use crate::store::{StoreData, StoreOpaque, Stored};
use crate::trampoline::{generate_global_export, generate_table_export};
use crate::{
    AsContext, AsContextMut, Engine, ExternRef, ExternType, Func, GlobalType, Memory, Mutability,
    SharedMemory, TableType, Trap, Val, ValType,
};
use anyhow::{anyhow, bail, Result};
use std::mem;
use std::ptr;
use wasmtime_runtime::{self as runtime, InstanceHandle};

// Externals

/// An external item to a WebAssembly module, or a list of what can possibly be
/// exported from a wasm module.
///
/// This is both returned from [`Instance::exports`](crate::Instance::exports)
/// as well as required by [`Instance::new`](crate::Instance::new). In other
/// words, this is the type of extracted values from an instantiated module, and
/// it's also used to provide imported values when instantiating a module.
#[derive(Clone)]
pub enum Extern {
    /// A WebAssembly `func` which can be called.
    Func(Func),
    /// A WebAssembly `global` which acts like a `Cell<T>` of sorts, supporting
    /// `get` and `set` operations.
    Global(Global),
    /// A WebAssembly `table` which is an array of `Val` reference types.
    Table(Table),
    /// A WebAssembly linear memory.
    Memory(Memory),
    /// A WebAssembly shared memory; these are handled separately from
    /// [`Memory`].
    SharedMemory(SharedMemory),
}

impl Extern {
    /// Returns the underlying `Func`, if this external is a function.
    ///
    /// Returns `None` if this is not a function.
    pub fn into_func(self) -> Option<Func> {
        match self {
            Extern::Func(func) => Some(func),
            _ => None,
        }
    }

    /// Returns the underlying `Global`, if this external is a global.
    ///
    /// Returns `None` if this is not a global.
    pub fn into_global(self) -> Option<Global> {
        match self {
            Extern::Global(global) => Some(global),
            _ => None,
        }
    }

    /// Returns the underlying `Table`, if this external is a table.
    ///
    /// Returns `None` if this is not a table.
    pub fn into_table(self) -> Option<Table> {
        match self {
            Extern::Table(table) => Some(table),
            _ => None,
        }
    }

    /// Returns the underlying `Memory`, if this external is a memory.
    ///
    /// Returns `None` if this is not a memory.
    pub fn into_memory(self) -> Option<Memory> {
        match self {
            Extern::Memory(memory) => Some(memory),
            _ => None,
        }
    }

    /// Returns the underlying `SharedMemory`, if this external is a shared
    /// memory.
    ///
    /// Returns `None` if this is not a shared memory.
    pub fn into_shared_memory(self) -> Option<SharedMemory> {
        match self {
            Extern::SharedMemory(memory) => Some(memory),
            _ => None,
        }
    }

    /// Returns the type associated with this `Extern`.
    ///
    /// The `store` argument provided must own this `Extern` and is used to look
    /// up type information.
    ///
    /// # Panics
    ///
    /// Panics if this item does not belong to the `store` provided.
    pub fn ty(&self, store: impl AsContext) -> ExternType {
        let store = store.as_context();
        match self {
            Extern::Func(ft) => ExternType::Func(ft.ty(store)),
            Extern::Memory(ft) => ExternType::Memory(ft.ty(store)),
            Extern::SharedMemory(ft) => ExternType::Memory(ft.ty()),
            Extern::Table(tt) => ExternType::Table(tt.ty(store)),
            Extern::Global(gt) => ExternType::Global(gt.ty(store)),
        }
    }

    pub(crate) unsafe fn from_wasmtime_export(
        wasmtime_export: wasmtime_runtime::Export,
        store: &mut StoreOpaque,
    ) -> Extern {
        match wasmtime_export {
            wasmtime_runtime::Export::Function(f) => {
                Extern::Func(Func::from_wasmtime_function(f, store))
            }
            wasmtime_runtime::Export::Memory(m) => {
                if m.memory.memory.shared {
                    Extern::SharedMemory(SharedMemory::from_wasmtime_memory(m, store))
                } else {
                    Extern::Memory(Memory::from_wasmtime_memory(m, store))
                }
            }
            wasmtime_runtime::Export::Global(g) => {
                Extern::Global(Global::from_wasmtime_global(g, store))
            }
            wasmtime_runtime::Export::Table(t) => {
                Extern::Table(Table::from_wasmtime_table(t, store))
            }
        }
    }

    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        match self {
            Extern::Func(f) => f.comes_from_same_store(store),
            Extern::Global(g) => store.store_data().contains(g.0),
            Extern::Memory(m) => m.comes_from_same_store(store),
            Extern::SharedMemory(m) => Engine::same(m.engine(), store.engine()),
            Extern::Table(t) => store.store_data().contains(t.0),
        }
    }

    pub(crate) fn desc(&self) -> &'static str {
        match self {
            Extern::Func(_) => "function",
            Extern::Table(_) => "table",
            Extern::Memory(_) => "memory",
            Extern::SharedMemory(_) => "shared memory",
            Extern::Global(_) => "global",
        }
    }
}

impl From<Func> for Extern {
    fn from(r: Func) -> Self {
        Extern::Func(r)
    }
}

impl From<Global> for Extern {
    fn from(r: Global) -> Self {
        Extern::Global(r)
    }
}

impl From<Memory> for Extern {
    fn from(r: Memory) -> Self {
        Extern::Memory(r)
    }
}

impl From<SharedMemory> for Extern {
    fn from(r: SharedMemory) -> Self {
        Extern::SharedMemory(r)
    }
}

impl From<Table> for Extern {
    fn from(r: Table) -> Self {
        Extern::Table(r)
    }
}

/// A WebAssembly `global` value which can be read and written to.
///
/// A `global` in WebAssembly is sort of like a global variable within an
/// [`Instance`](crate::Instance). The `global.get` and `global.set`
/// instructions will modify and read global values in a wasm module. Globals
/// can either be imported or exported from wasm modules.
///
/// A [`Global`] "belongs" to the store that it was originally created within
/// (either via [`Global::new`] or via instantiating a
/// [`Module`](crate::Module)). Operations on a [`Global`] only work with the
/// store it belongs to, and if another store is passed in by accident then
/// methods will panic.
#[derive(Copy, Clone, Debug)]
#[repr(transparent)] // here for the C API
pub struct Global(Stored<wasmtime_runtime::ExportGlobal>);

impl Global {
    /// Creates a new WebAssembly `global` value with the provide type `ty` and
    /// initial value `val`.
    ///
    /// The `store` argument will be the owner of the [`Global`] returned. Using
    /// the returned [`Global`] other items in the store may access this global.
    /// For example this could be provided as an argument to
    /// [`Instance::new`](crate::Instance::new) or
    /// [`Linker::define`](crate::Linker::define).
    ///
    /// # Errors
    ///
    /// Returns an error if the `ty` provided does not match the type of the
    /// value `val`, or if `val` comes from a different store than `store`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// let engine = Engine::default();
    /// let mut store = Store::new(&engine, ());
    ///
    /// let ty = GlobalType::new(ValType::I32, Mutability::Const);
    /// let i32_const = Global::new(&mut store, ty, 1i32.into())?;
    /// let ty = GlobalType::new(ValType::F64, Mutability::Var);
    /// let f64_mut = Global::new(&mut store, ty, 2.0f64.into())?;
    ///
    /// let module = Module::new(
    ///     &engine,
    ///     "(module
    ///         (global (import \"\" \"i32-const\") i32)
    ///         (global (import \"\" \"f64-mut\") (mut f64))
    ///     )"
    /// )?;
    ///
    /// let mut linker = Linker::new(&engine);
    /// linker.define("", "i32-const", i32_const)?;
    /// linker.define("", "f64-mut", f64_mut)?;
    ///
    /// let instance = linker.instantiate(&mut store, &module)?;
    /// // ...
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(mut store: impl AsContextMut, ty: GlobalType, val: Val) -> Result<Global> {
        Global::_new(store.as_context_mut().0, ty, val)
    }

    fn _new(store: &mut StoreOpaque, ty: GlobalType, val: Val) -> Result<Global> {
        if !val.comes_from_same_store(store) {
            bail!("cross-`Store` globals are not supported");
        }
        if val.ty() != *ty.content() {
            bail!("value provided does not match the type of this global");
        }
        unsafe {
            let wasmtime_export = generate_global_export(store, &ty, val)?;
            Ok(Global::from_wasmtime_global(wasmtime_export, store))
        }
    }

    /// Returns the underlying type of this `global`.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this global.
    pub fn ty(&self, store: impl AsContext) -> GlobalType {
        let store = store.as_context();
        let ty = &store[self.0].global;
        GlobalType::from_wasmtime_global(&ty)
    }

    /// Returns the current [`Val`] of this global.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this global.
    pub fn get(&self, mut store: impl AsContextMut) -> Val {
        unsafe {
            let store = store.as_context_mut();
            let definition = &*store[self.0].definition;
            match self.ty(&store).content() {
                ValType::I32 => Val::from(*definition.as_i32()),
                ValType::I64 => Val::from(*definition.as_i64()),
                ValType::F32 => Val::F32(*definition.as_u32()),
                ValType::F64 => Val::F64(*definition.as_u64()),
                ValType::ExternRef => Val::ExternRef(
                    definition
                        .as_externref()
                        .clone()
                        .map(|inner| ExternRef { inner }),
                ),
                ValType::FuncRef => {
                    Val::FuncRef(Func::from_raw(store, definition.as_anyfunc() as usize))
                }
                ValType::V128 => Val::V128(*definition.as_u128()),
            }
        }
    }

    /// Attempts to set the current value of this global to [`Val`].
    ///
    /// # Errors
    ///
    /// Returns an error if this global has a different type than `Val`, if
    /// it's not a mutable global, or if `val` comes from a different store than
    /// the one provided.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this global.
    pub fn set(&self, mut store: impl AsContextMut, val: Val) -> Result<()> {
        let store = store.as_context_mut().0;
        let ty = self.ty(&store);
        if ty.mutability() != Mutability::Var {
            bail!("immutable global cannot be set");
        }
        let ty = ty.content();
        if val.ty() != *ty {
            bail!("global of type {:?} cannot be set to {:?}", ty, val.ty());
        }
        if !val.comes_from_same_store(store) {
            bail!("cross-`Store` values are not supported");
        }
        unsafe {
            let definition = &mut *store[self.0].definition;
            match val {
                Val::I32(i) => *definition.as_i32_mut() = i,
                Val::I64(i) => *definition.as_i64_mut() = i,
                Val::F32(f) => *definition.as_u32_mut() = f,
                Val::F64(f) => *definition.as_u64_mut() = f,
                Val::FuncRef(f) => {
                    *definition.as_anyfunc_mut() = f.map_or(ptr::null(), |f| {
                        f.caller_checked_anyfunc(store).as_ptr().cast()
                    });
                }
                Val::ExternRef(x) => {
                    let old = mem::replace(definition.as_externref_mut(), x.map(|x| x.inner));
                    drop(old);
                }
                Val::V128(i) => *definition.as_u128_mut() = i,
            }
        }
        Ok(())
    }

    pub(crate) unsafe fn from_wasmtime_global(
        wasmtime_export: wasmtime_runtime::ExportGlobal,
        store: &mut StoreOpaque,
    ) -> Global {
        Global(store.store_data_mut().insert(wasmtime_export))
    }

    pub(crate) fn wasmtime_ty<'a>(&self, data: &'a StoreData) -> &'a wasmtime_environ::Global {
        &data[self.0].global
    }

    pub(crate) fn vmimport(&self, store: &StoreOpaque) -> wasmtime_runtime::VMGlobalImport {
        wasmtime_runtime::VMGlobalImport {
            from: store[self.0].definition,
        }
    }
}

/// A WebAssembly `table`, or an array of values.
///
/// Like [`Memory`] a table is an indexed array of values, but unlike [`Memory`]
/// it's an array of WebAssembly reference type values rather than bytes. One of
/// the most common usages of a table is a function table for wasm modules (a
/// `funcref` table), where each element has the `ValType::FuncRef` type.
///
/// A [`Table`] "belongs" to the store that it was originally created within
/// (either via [`Table::new`] or via instantiating a
/// [`Module`](crate::Module)). Operations on a [`Table`] only work with the
/// store it belongs to, and if another store is passed in by accident then
/// methods will panic.
#[derive(Copy, Clone, Debug)]
#[repr(transparent)] // here for the C API
pub struct Table(Stored<wasmtime_runtime::ExportTable>);

impl Table {
    /// Creates a new [`Table`] with the given parameters.
    ///
    /// * `store` - the owner of the resulting [`Table`]
    /// * `ty` - the type of this table, containing both the element type as
    ///   well as the initial size and maximum size, if any.
    /// * `init` - the initial value to fill all table entries with, if the
    ///   table starts with an initial size.
    ///
    /// # Errors
    ///
    /// Returns an error if `init` does not match the element type of the table,
    /// or if `init` does not belong to the `store` provided.
    ///
    /// # Panics
    ///
    /// This function will panic when used with a [`Store`](`crate::Store`)
    /// which has a [`ResourceLimiterAsync`](`crate::ResourceLimiterAsync`)
    /// (see also: [`Store::limiter_async`](`crate::Store::limiter_async`).
    /// When using an async resource limiter, use [`Table::new_async`]
    /// instead.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// let engine = Engine::default();
    /// let mut store = Store::new(&engine, ());
    ///
    /// let ty = TableType::new(ValType::FuncRef, 2, None);
    /// let table = Table::new(&mut store, ty, Val::FuncRef(None))?;
    ///
    /// let module = Module::new(
    ///     &engine,
    ///     "(module
    ///         (table (import \"\" \"\") 2 funcref)
    ///         (func $f (result i32)
    ///             i32.const 10)
    ///         (elem (i32.const 0) $f)
    ///     )"
    /// )?;
    ///
    /// let instance = Instance::new(&mut store, &module, &[table.into()])?;
    /// // ...
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(mut store: impl AsContextMut, ty: TableType, init: Val) -> Result<Table> {
        Table::_new(store.as_context_mut().0, ty, init)
    }

    #[cfg_attr(nightlydoc, doc(cfg(feature = "async")))]
    /// Async variant of [`Table::new`]. You must use this variant with
    /// [`Store`](`crate::Store`)s which have a
    /// [`ResourceLimiterAsync`](`crate::ResourceLimiterAsync`).
    ///
    /// # Panics
    ///
    /// This function will panic when used with a non-async
    /// [`Store`](`crate::Store`)
    #[cfg(feature = "async")]
    pub async fn new_async<T>(
        mut store: impl AsContextMut<Data = T>,
        ty: TableType,
        init: Val,
    ) -> Result<Table>
    where
        T: Send,
    {
        let mut store = store.as_context_mut();
        assert!(
            store.0.async_support(),
            "cannot use `new_async` without enabling async support on the config"
        );
        store
            .on_fiber(|store| Table::_new(store.0, ty, init))
            .await?
    }

    fn _new(store: &mut StoreOpaque, ty: TableType, init: Val) -> Result<Table> {
        let wasmtime_export = generate_table_export(store, &ty)?;
        let init = init.into_table_element(store, ty.element())?;
        unsafe {
            let table = Table::from_wasmtime_table(wasmtime_export, store);
            (*table.wasmtime_table(store, std::iter::empty()))
                .fill(0, init, ty.minimum())
                .map_err(|c| Trap::new_wasm(c, None))?;

            Ok(table)
        }
    }

    /// Returns the underlying type of this table, including its element type as
    /// well as the maximum/minimum lower bounds.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this table.
    pub fn ty(&self, store: impl AsContext) -> TableType {
        let store = store.as_context();
        let ty = &store[self.0].table.table;
        TableType::from_wasmtime_table(ty)
    }

    fn wasmtime_table(
        &self,
        store: &mut StoreOpaque,
        lazy_init_range: impl Iterator<Item = u32>,
    ) -> *mut runtime::Table {
        unsafe {
            let export = &store[self.0];
            let mut handle = InstanceHandle::from_vmctx(export.vmctx);
            let idx = handle.table_index(&*export.definition);
            handle.get_defined_table_with_lazy_init(idx, lazy_init_range)
        }
    }

    /// Returns the table element value at `index`.
    ///
    /// Returns `None` if `index` is out of bounds.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this table.
    pub fn get(&self, mut store: impl AsContextMut, index: u32) -> Option<Val> {
        let store = store.as_context_mut().0;
        let table = self.wasmtime_table(store, std::iter::once(index));
        unsafe {
            match (*table).get(index)? {
                runtime::TableElement::FuncRef(f) => {
                    let func = Func::from_caller_checked_anyfunc(store, f);
                    Some(Val::FuncRef(func))
                }
                runtime::TableElement::ExternRef(None) => Some(Val::ExternRef(None)),
                runtime::TableElement::ExternRef(Some(x)) => {
                    Some(Val::ExternRef(Some(ExternRef { inner: x })))
                }
                runtime::TableElement::UninitFunc => {
                    unreachable!("lazy init above should have converted UninitFunc")
                }
            }
        }
    }

    /// Writes the `val` provided into `index` within this table.
    ///
    /// # Errors
    ///
    /// Returns an error if `index` is out of bounds, if `val` does not have
    /// the right type to be stored in this table, or if `val` belongs to a
    /// different store.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this table.
    pub fn set(&self, mut store: impl AsContextMut, index: u32, val: Val) -> Result<()> {
        let store = store.as_context_mut().0;
        let ty = self.ty(&store).element().clone();
        let val = val.into_table_element(store, ty)?;
        let table = self.wasmtime_table(store, std::iter::empty());
        unsafe {
            (*table)
                .set(index, val)
                .map_err(|()| anyhow!("table element index out of bounds"))
        }
    }

    /// Returns the current size of this table.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this table.
    pub fn size(&self, store: impl AsContext) -> u32 {
        self.internal_size(store.as_context().0)
    }

    pub(crate) fn internal_size(&self, store: &StoreOpaque) -> u32 {
        unsafe { (*store[self.0].definition).current_elements }
    }

    /// Grows the size of this table by `delta` more elements, initialization
    /// all new elements to `init`.
    ///
    /// Returns the previous size of this table if successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the table cannot be grown by `delta`, for example
    /// if it would cause the table to exceed its maximum size. Also returns an
    /// error if `init` is not of the right type or if `init` does not belong to
    /// `store`.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this table.
    ///
    /// This function will panic when used with a [`Store`](`crate::Store`)
    /// which has a [`ResourceLimiterAsync`](`crate::ResourceLimiterAsync`)
    /// (see also: [`Store::limiter_async`](`crate::Store::limiter_async`)).
    /// When using an async resource limiter, use [`Table::grow_async`]
    /// instead.
    pub fn grow(&self, mut store: impl AsContextMut, delta: u32, init: Val) -> Result<u32> {
        let store = store.as_context_mut().0;
        let ty = self.ty(&store).element().clone();
        let init = init.into_table_element(store, ty)?;
        let table = self.wasmtime_table(store, std::iter::empty());
        unsafe {
            match (*table).grow(delta, init, store)? {
                Some(size) => {
                    let vm = (*table).vmtable();
                    *store[self.0].definition = vm;
                    Ok(size)
                }
                None => bail!("failed to grow table by `{}`", delta),
            }
        }
    }

    #[cfg_attr(nightlydoc, doc(cfg(feature = "async")))]
    /// Async variant of [`Table::grow`]. Required when using a
    /// [`ResourceLimiterAsync`](`crate::ResourceLimiterAsync`).
    ///
    /// # Panics
    ///
    /// This function will panic when used with a non-async
    /// [`Store`](`crate::Store`).
    #[cfg(feature = "async")]
    pub async fn grow_async<T>(
        &self,
        mut store: impl AsContextMut<Data = T>,
        delta: u32,
        init: Val,
    ) -> Result<u32>
    where
        T: Send,
    {
        let mut store = store.as_context_mut();
        assert!(
            store.0.async_support(),
            "cannot use `grow_async` without enabling async support on the config"
        );
        store
            .on_fiber(|store| self.grow(store, delta, init))
            .await?
    }

    /// Copy `len` elements from `src_table[src_index..]` into
    /// `dst_table[dst_index..]`.
    ///
    /// # Errors
    ///
    /// Returns an error if the range is out of bounds of either the source or
    /// destination tables.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own either `dst_table` or `src_table`.
    pub fn copy(
        mut store: impl AsContextMut,
        dst_table: &Table,
        dst_index: u32,
        src_table: &Table,
        src_index: u32,
        len: u32,
    ) -> Result<()> {
        let store = store.as_context_mut().0;
        if dst_table.ty(&store).element() != src_table.ty(&store).element() {
            bail!("tables do not have the same element type");
        }

        let dst_table = dst_table.wasmtime_table(store, std::iter::empty());
        let src_range = src_index..(src_index.checked_add(len).unwrap_or(u32::MAX));
        let src_table = src_table.wasmtime_table(store, src_range);
        unsafe {
            runtime::Table::copy(dst_table, src_table, dst_index, src_index, len)
                .map_err(|c| Trap::new_wasm(c, None))?;
        }
        Ok(())
    }

    /// Fill `table[dst..(dst + len)]` with the given value.
    ///
    /// # Errors
    ///
    /// Returns an error if
    ///
    /// * `val` is not of the same type as this table's
    ///   element type,
    ///
    /// * the region to be filled is out of bounds, or
    ///
    /// * `val` comes from a different `Store` from this table.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own either `dst_table` or `src_table`.
    pub fn fill(&self, mut store: impl AsContextMut, dst: u32, val: Val, len: u32) -> Result<()> {
        let store = store.as_context_mut().0;
        let ty = self.ty(&store).element().clone();
        let val = val.into_table_element(store, ty)?;

        let table = self.wasmtime_table(store, std::iter::empty());
        unsafe {
            (*table)
                .fill(dst, val, len)
                .map_err(|c| Trap::new_wasm(c, None))?;
        }

        Ok(())
    }

    pub(crate) unsafe fn from_wasmtime_table(
        wasmtime_export: wasmtime_runtime::ExportTable,
        store: &mut StoreOpaque,
    ) -> Table {
        Table(store.store_data_mut().insert(wasmtime_export))
    }

    pub(crate) fn wasmtime_ty<'a>(&self, data: &'a StoreData) -> &'a wasmtime_environ::Table {
        &data[self.0].table.table
    }

    pub(crate) fn vmimport(&self, store: &StoreOpaque) -> wasmtime_runtime::VMTableImport {
        let export = &store[self.0];
        wasmtime_runtime::VMTableImport {
            from: export.definition,
            vmctx: export.vmctx,
        }
    }
}

// Exports

/// An exported WebAssembly value.
///
/// This type is primarily accessed from the
/// [`Instance::exports`](crate::Instance::exports) accessor and describes what
/// names and items are exported from a wasm instance.
#[derive(Clone)]
pub struct Export<'instance> {
    /// The name of the export.
    name: &'instance str,

    /// The definition of the export.
    definition: Extern,
}

impl<'instance> Export<'instance> {
    /// Creates a new export which is exported with the given `name` and has the
    /// given `definition`.
    pub(crate) fn new(name: &'instance str, definition: Extern) -> Export<'instance> {
        Export { name, definition }
    }

    /// Returns the name by which this export is known.
    pub fn name(&self) -> &'instance str {
        self.name
    }

    /// Return the `ExternType` of this export.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this `Extern`.
    pub fn ty(&self, store: impl AsContext) -> ExternType {
        self.definition.ty(store)
    }

    /// Consume this `Export` and return the contained `Extern`.
    pub fn into_extern(self) -> Extern {
        self.definition
    }

    /// Consume this `Export` and return the contained `Func`, if it's a function,
    /// or `None` otherwise.
    pub fn into_func(self) -> Option<Func> {
        self.definition.into_func()
    }

    /// Consume this `Export` and return the contained `Table`, if it's a table,
    /// or `None` otherwise.
    pub fn into_table(self) -> Option<Table> {
        self.definition.into_table()
    }

    /// Consume this `Export` and return the contained `Memory`, if it's a memory,
    /// or `None` otherwise.
    pub fn into_memory(self) -> Option<Memory> {
        self.definition.into_memory()
    }

    /// Consume this `Export` and return the contained `Global`, if it's a global,
    /// or `None` otherwise.
    pub fn into_global(self) -> Option<Global> {
        self.definition.into_global()
    }
}
