use crate::prelude::*;
use crate::runtime::vm::{self as runtime};
use crate::store::{AutoAssertNoGc, StoreData, StoreOpaque, Stored};
use crate::trampoline::generate_table_export;
use crate::vm::ExportTable;
use crate::{AnyRef, AsContext, AsContextMut, ExternRef, Func, HeapType, Ref, TableType};
use core::iter;
use wasmtime_environ::TypeTrace;

/// A WebAssembly `table`, or an array of values.
///
/// Like [`Memory`][crate::Memory] a table is an indexed array of values, but
/// unlike [`Memory`][crate::Memory] it's an array of WebAssembly reference type
/// values rather than bytes. One of the most common usages of a table is a
/// function table for wasm modules (a `funcref` table), where each element has
/// the `ValType::FuncRef` type.
///
/// A [`Table`] "belongs" to the store that it was originally created within
/// (either via [`Table::new`] or via instantiating a
/// [`Module`](crate::Module)). Operations on a [`Table`] only work with the
/// store it belongs to, and if another store is passed in by accident then
/// methods will panic.
#[derive(Copy, Clone, Debug)]
#[repr(transparent)] // here for the C API
pub struct Table(pub(super) Stored<crate::runtime::vm::ExportTable>);

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
    /// let ty = TableType::new(RefType::FUNCREF, 2, None);
    /// let table = Table::new(&mut store, ty, Ref::Func(None))?;
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
    pub fn new(mut store: impl AsContextMut, ty: TableType, init: Ref) -> Result<Table> {
        Table::_new(store.as_context_mut().0, ty, init)
    }

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
        init: Ref,
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

    fn _new(store: &mut StoreOpaque, ty: TableType, init: Ref) -> Result<Table> {
        let wasmtime_export = generate_table_export(store, &ty)?;
        let init = init.into_table_element(store, ty.element())?;
        unsafe {
            let table = Table::from_wasmtime_table(wasmtime_export, store);
            let wasmtime_table = table.wasmtime_table(store, iter::empty());
            (*wasmtime_table).fill(store.optional_gc_store_mut()?, 0, init, ty.minimum())?;
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
        self._ty(store.as_context().0)
    }

    fn _ty(&self, store: &StoreOpaque) -> TableType {
        let ty = &store[self.0].table;
        TableType::from_wasmtime_table(store.engine(), ty)
    }

    fn wasmtime_table(
        &self,
        store: &mut StoreOpaque,
        lazy_init_range: impl Iterator<Item = u64>,
    ) -> *mut runtime::Table {
        unsafe {
            let ExportTable {
                vmctx, definition, ..
            } = store[self.0];
            crate::runtime::vm::Instance::from_vmctx(vmctx, |handle| {
                let idx = handle.table_index(definition.as_ref());
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
    pub fn get(&self, mut store: impl AsContextMut, index: u64) -> Option<Ref> {
        let mut store = AutoAssertNoGc::new(store.as_context_mut().0);
        let table = self.wasmtime_table(&mut store, iter::once(index));
        let gc_store = store.optional_gc_store_mut().ok().and_then(|s| s);
        unsafe {
            match (*table).get(gc_store, index)? {
                runtime::TableElement::FuncRef(f) => {
                    let func = f.map(|f| Func::from_vm_func_ref(&mut store, f));
                    Some(func.into())
                }

                runtime::TableElement::UninitFunc => {
                    unreachable!("lazy init above should have converted UninitFunc")
                }

                runtime::TableElement::GcRef(None) => {
                    Some(Ref::null(self._ty(&store).element().heap_type()))
                }

                #[cfg_attr(not(feature = "gc"), allow(unreachable_code, unused_variables))]
                runtime::TableElement::GcRef(Some(x)) => {
                    match self._ty(&store).element().heap_type().top() {
                        HeapType::Any => {
                            let x = AnyRef::from_cloned_gc_ref(&mut store, x);
                            Some(x.into())
                        }
                        HeapType::Extern => {
                            let x = ExternRef::from_cloned_gc_ref(&mut store, x);
                            Some(x.into())
                        }
                        HeapType::Func => {
                            unreachable!("never have TableElement::GcRef for func tables")
                        }
                        ty => unreachable!("not a top type: {ty:?}"),
                    }
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
    pub fn set(&self, mut store: impl AsContextMut, index: u64, val: Ref) -> Result<()> {
        let store = store.as_context_mut().0;
        let ty = self.ty(&store);
        let val = val.into_table_element(store, ty.element())?;
        let table = self.wasmtime_table(store, iter::empty());
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
    pub fn size(&self, store: impl AsContext) -> u64 {
        self.internal_size(store.as_context().0)
    }

    pub(crate) fn internal_size(&self, store: &StoreOpaque) -> u64 {
        // unwrap here should be ok because the runtime should always guarantee
        // that we can fit the number of elements in a 64-bit integer.
        unsafe { u64::try_from(store[self.0].definition.as_ref().current_elements).unwrap() }
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
    pub fn grow(&self, mut store: impl AsContextMut, delta: u64, init: Ref) -> Result<u64> {
        let store = store.as_context_mut().0;
        let ty = self.ty(&store);
        let init = init.into_table_element(store, ty.element())?;
        let table = self.wasmtime_table(store, iter::empty());
        unsafe {
            match (*table).grow(delta, init, store)? {
                Some(size) => {
                    let vm = (*table).vmtable();
                    store[self.0].definition.write(vm);
                    // unwrap here should be ok because the runtime should always guarantee
                    // that we can fit the table size in a 64-bit integer.
                    Ok(u64::try_from(size).unwrap())
                }
                None => bail!("failed to grow table by `{}`", delta),
            }
        }
    }

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
        delta: u64,
        init: Ref,
    ) -> Result<u64>
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
    /// destination tables, or if the source table's element type does not match
    /// the destination table's element type.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own either `dst_table` or `src_table`.
    pub fn copy(
        mut store: impl AsContextMut,
        dst_table: &Table,
        dst_index: u64,
        src_table: &Table,
        src_index: u64,
        len: u64,
    ) -> Result<()> {
        let store = store.as_context_mut().0;

        let dst_ty = dst_table.ty(&store);
        let src_ty = src_table.ty(&store);
        src_ty
            .element()
            .ensure_matches(store.engine(), dst_ty.element())
            .context(
                "type mismatch: source table's element type does not match \
                 destination table's element type",
            )?;

        let dst_table = dst_table.wasmtime_table(store, iter::empty());
        let src_range = src_index..(src_index.checked_add(len).unwrap_or(u64::MAX));
        let src_table = src_table.wasmtime_table(store, src_range);
        unsafe {
            runtime::Table::copy(
                store.optional_gc_store_mut()?,
                dst_table,
                src_table,
                dst_index,
                src_index,
                len,
            )?;
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
    pub fn fill(&self, mut store: impl AsContextMut, dst: u64, val: Ref, len: u64) -> Result<()> {
        let store = store.as_context_mut().0;
        let ty = self.ty(&store);
        let val = val.into_table_element(store, ty.element())?;

        let table = self.wasmtime_table(store, iter::empty());
        unsafe {
            (*table).fill(store.optional_gc_store_mut()?, dst, val, len)?;
        }

        Ok(())
    }

    #[cfg(feature = "gc")]
    pub(crate) fn trace_roots(
        &self,
        store: &mut StoreOpaque,
        gc_roots_list: &mut crate::runtime::vm::GcRootsList,
    ) {
        if !self
            ._ty(store)
            .element()
            .is_vmgcref_type_and_points_to_object()
        {
            return;
        }

        let table = self.wasmtime_table(store, iter::empty());
        for gc_ref in unsafe { (*table).gc_refs_mut() } {
            if let Some(gc_ref) = gc_ref {
                unsafe {
                    gc_roots_list.add_root(gc_ref.into(), "Wasm table element");
                }
            }
        }
    }

    pub(crate) unsafe fn from_wasmtime_table(
        mut wasmtime_export: crate::runtime::vm::ExportTable,
        store: &mut StoreOpaque,
    ) -> Table {
        // Ensure that the table's type is engine-level canonicalized.
        wasmtime_export
            .table
            .ref_type
            .canonicalize_for_runtime_usage(&mut |module_index| {
                crate::runtime::vm::Instance::from_vmctx(wasmtime_export.vmctx, |instance| {
                    instance.engine_type_index(module_index)
                })
            });

        Table(store.store_data_mut().insert(wasmtime_export))
    }

    pub(crate) fn wasmtime_ty<'a>(&self, data: &'a StoreData) -> &'a wasmtime_environ::Table {
        &data[self.0].table
    }

    pub(crate) fn vmimport(&self, store: &StoreOpaque) -> crate::runtime::vm::VMTableImport {
        let export = &store[self.0];
        crate::runtime::vm::VMTableImport {
            from: export.definition.into(),
            vmctx: export.vmctx.into(),
        }
    }

    /// Get a stable hash key for this table.
    ///
    /// Even if the same underlying table definition is added to the
    /// `StoreData` multiple times and becomes multiple `wasmtime::Table`s,
    /// this hash key will be consistent across all of these tables.
    #[allow(dead_code)] // Not used yet, but added for consistency.
    pub(crate) fn hash_key(&self, store: &StoreOpaque) -> impl core::hash::Hash + Eq + use<'_> {
        store[self.0].definition.as_ptr() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Instance, Module, Store};

    #[test]
    fn hash_key_is_stable_across_duplicate_store_data_entries() -> Result<()> {
        let mut store = Store::<()>::default();
        let module = Module::new(
            store.engine(),
            r#"
                (module
                    (table (export "t") 1 1 externref)
                )
            "#,
        )?;
        let instance = Instance::new(&mut store, &module, &[])?;

        // Each time we `get_table`, we call `Table::from_wasmtime` which adds
        // a new entry to `StoreData`, so `t1` and `t2` will have different
        // indices into `StoreData`.
        let t1 = instance.get_table(&mut store, "t").unwrap();
        let t2 = instance.get_table(&mut store, "t").unwrap();

        // That said, they really point to the same table.
        assert!(t1.get(&mut store, 0).unwrap().unwrap_extern().is_none());
        assert!(t2.get(&mut store, 0).unwrap().unwrap_extern().is_none());
        let e = ExternRef::new(&mut store, 42)?;
        t1.set(&mut store, 0, e.into())?;
        assert!(t1.get(&mut store, 0).unwrap().unwrap_extern().is_some());
        assert!(t2.get(&mut store, 0).unwrap().unwrap_extern().is_some());

        // And therefore their hash keys are the same.
        assert!(t1.hash_key(&store.as_context().0) == t2.hash_key(&store.as_context().0));

        // But the hash keys are different from different tables.
        let instance2 = Instance::new(&mut store, &module, &[])?;
        let t3 = instance2.get_table(&mut store, "t").unwrap();
        assert!(t1.hash_key(&store.as_context().0) != t3.hash_key(&store.as_context().0));

        Ok(())
    }
}
