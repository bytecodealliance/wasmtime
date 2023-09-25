use crate::store::{StoreData, StoreOpaque, Stored};
use crate::trampoline::generate_table_export;
use crate::{AsContext, AsContextMut, ExternRef, Func, TableType, Val};
use anyhow::{anyhow, bail, Result};
use wasmtime_runtime::{self as runtime};

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
pub struct Table(pub(super) Stored<wasmtime_runtime::ExportTable>);

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
            (*table.wasmtime_table(store, std::iter::empty())).fill(0, init, ty.minimum())?;

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
            wasmtime_runtime::Instance::from_vmctx(export.vmctx, |handle| {
                let idx = handle.table_index(&*export.definition);
                handle.get_defined_table_with_lazy_init(idx, lazy_init_range)
            })
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
                    let func = Func::from_caller_checked_func_ref(store, f);
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
            runtime::Table::copy(dst_table, src_table, dst_index, src_index, len)?;
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
            (*table).fill(dst, val, len)?;
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
