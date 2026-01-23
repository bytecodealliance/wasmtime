use crate::prelude::*;
use crate::runtime::RootedGcRefImpl;
use crate::runtime::vm::{
    self, GcStore, SendSyncPtr, TableElementType, VMFuncRef, VMGcRef, VMStore,
};
use crate::store::{AutoAssertNoGc, StoreInstanceId, StoreOpaque, StoreResourceLimiter};
use crate::trampoline::generate_table_export;
use crate::{
    AnyRef, AsContext, AsContextMut, ExnRef, ExternRef, Func, HeapType, Ref, RefType,
    StoreContextMut, TableType, Trap,
};
use core::iter;
use core::ptr::NonNull;
use wasmtime_environ::DefinedTableIndex;

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
#[repr(C)] // here for the C API
pub struct Table {
    instance: StoreInstanceId,
    index: DefinedTableIndex,
}

// Double-check that the C representation in `extern.h` matches our in-Rust
// representation here in terms of size/alignment/etc.
const _: () = {
    #[repr(C)]
    struct Tmp(u64, u32);
    #[repr(C)]
    struct C(Tmp, u32);
    assert!(core::mem::size_of::<C>() == core::mem::size_of::<Table>());
    assert!(core::mem::align_of::<C>() == core::mem::align_of::<Table>());
    assert!(core::mem::offset_of!(Table, instance) == 0);
};

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
    /// This function will also return an error when used with a
    /// [`Store`](`crate::Store`) which has a
    /// [`ResourceLimiterAsync`](`crate::ResourceLimiterAsync`) (see also:
    /// [`Store::limiter_async`](`crate::Store::limiter_async`).  When using an
    /// async resource limiter, use [`Table::new_async`] instead.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> Result<()> {
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
        let (mut limiter, store) = store
            .as_context_mut()
            .0
            .validate_sync_resource_limiter_and_store_opaque()?;
        vm::assert_ready(Table::_new(store, limiter.as_mut(), ty, init))
    }

    /// Async variant of [`Table::new`].
    ///
    /// You must use this variant with [`Store`](`crate::Store`)s which have a
    /// [`ResourceLimiterAsync`](`crate::ResourceLimiterAsync`).
    #[cfg(feature = "async")]
    pub async fn new_async(
        mut store: impl AsContextMut,
        ty: TableType,
        init: Ref,
    ) -> Result<Table> {
        let (mut limiter, store) = store.as_context_mut().0.resource_limiter_and_store_opaque();
        Table::_new(store, limiter.as_mut(), ty, init).await
    }

    async fn _new(
        store: &mut StoreOpaque,
        limiter: Option<&mut StoreResourceLimiter<'_>>,
        ty: TableType,
        init: Ref,
    ) -> Result<Table> {
        let table = generate_table_export(store, limiter, &ty).await?;
        table._fill(store, 0, init, ty.minimum())?;
        Ok(table)
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
        TableType::from_wasmtime_table(store.engine(), self.wasmtime_ty(store))
    }

    /// Returns the `vm::Table` within `store` as well as the optional
    /// `GcStore` in use within `store`.
    ///
    /// # Panics
    ///
    /// Panics if this table does not belong to `store`.
    fn wasmtime_table<'a>(
        &self,
        store: &'a mut StoreOpaque,
        lazy_init_range: impl IntoIterator<Item = u64>,
    ) -> (&'a mut vm::Table, Option<&'a mut GcStore>) {
        self.instance.assert_belongs_to(store.id());
        let (store, registry, instance) =
            store.optional_gc_store_and_registry_and_instance_mut(self.instance.instance());

        (
            instance.get_defined_table_with_lazy_init(registry, self.index, lazy_init_range),
            store,
        )
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
        let (table, _gc_store) = self.wasmtime_table(&mut store, [index]);
        match table.element_type() {
            TableElementType::Func => {
                let ptr = table.get_func(index).ok()?;
                Some(
                    // SAFETY: `store` owns this table, so therefore it owns all
                    // functions within the table too.
                    ptr.map(|p| unsafe { Func::from_vm_func_ref(store.id(), p) })
                        .into(),
                )
            }
            TableElementType::GcRef => {
                let gc_ref = table
                    .get_gc_ref(index)
                    .ok()?
                    .map(|r| r.unchecked_copy())
                    .map(|r| store.clone_gc_ref(&r));
                Some(match self._ty(&store).element().heap_type().top() {
                    HeapType::Extern => {
                        Ref::Extern(gc_ref.map(|r| ExternRef::from_cloned_gc_ref(&mut store, r)))
                    }
                    HeapType::Any => {
                        Ref::Any(gc_ref.map(|r| AnyRef::from_cloned_gc_ref(&mut store, r)))
                    }
                    HeapType::Exn => {
                        Ref::Exn(gc_ref.map(|r| ExnRef::from_cloned_gc_ref(&mut store, r)))
                    }
                    _ => unreachable!(),
                })
            }
            // TODO(#10248) Required to support stack switching in the embedder
            // API.
            TableElementType::Cont => panic!("unimplemented table for cont"),
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
        self.set_(store.as_context_mut().0, index, val)
    }

    pub(crate) fn set_(&self, store: &mut StoreOpaque, index: u64, val: Ref) -> Result<()> {
        let ty = self._ty(store);
        match element_type(&ty) {
            TableElementType::Func => {
                let element = val.into_table_func(store, ty.element())?;
                let (table, _gc_store) = self.wasmtime_table(store, iter::empty());
                table.set_func(index, element)?;
            }
            TableElementType::GcRef => {
                let mut store = AutoAssertNoGc::new(store);
                let element = val.into_table_gc_ref(&mut store, ty.element())?;
                // Note that `unchecked_copy` should be ok as we're under an
                // `AutoAssertNoGc` which means that despite this not being
                // rooted we don't have to worry about it going away.
                let element = element.map(|r| r.unchecked_copy());
                let (table, gc_store) = self.wasmtime_table(&mut store, iter::empty());
                table.set_gc_ref(gc_store, index, element.as_ref())?;
            }
            // TODO(#10248) Required to support stack switching in the embedder
            // API.
            TableElementType::Cont => bail!("unimplemented table for cont"),
        }
        Ok(())
    }

    /// Returns the current size of this table.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this table.
    pub fn size(&self, store: impl AsContext) -> u64 {
        self._size(store.as_context().0)
    }

    pub(crate) fn _size(&self, store: &StoreOpaque) -> u64 {
        // unwrap here should be ok because the runtime should always guarantee
        // that we can fit the number of elements in a 64-bit integer.
        u64::try_from(store[self.instance].table(self.index).current_elements).unwrap()
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
    /// This function also returns an error when used with a
    /// [`Store`](`crate::Store`) which has a
    /// [`ResourceLimiterAsync`](`crate::ResourceLimiterAsync`) (see also:
    /// [`Store::limiter_async`](`crate::Store::limiter_async`)).  When using an
    /// async resource limiter, use [`Table::grow_async`] instead.
    ///
    /// # Panics
    ///
    /// Panics if `store` does not own this table.
    pub fn grow(&self, mut store: impl AsContextMut, delta: u64, init: Ref) -> Result<u64> {
        let store = store.as_context_mut();
        store.0.validate_sync_resource_limiter_and_store_opaque()?;
        vm::assert_ready(self._grow(store, delta, init))
    }

    async fn _grow<T>(&self, store: StoreContextMut<'_, T>, delta: u64, init: Ref) -> Result<u64> {
        let store = store.0;
        let ty = self.ty(&store);
        let (mut limiter, store) = store.resource_limiter_and_store_opaque();
        let limiter = limiter.as_mut();
        let result = match element_type(&ty) {
            TableElementType::Func => {
                let element = init
                    .into_table_func(store, ty.element())?
                    .map(SendSyncPtr::new);
                self.instance
                    .get_mut(store)
                    .defined_table_grow(self.index, async |table| {
                        // SAFETY: in the context of `defined_table_grow` this
                        // is safe to call as it'll update the internal table
                        // pointer in the instance.
                        unsafe { table.grow_func(limiter, delta, element).await }
                    })
                    .await?
            }
            TableElementType::GcRef => {
                let mut store = AutoAssertNoGc::new(store);
                let element = init
                    .into_table_gc_ref(&mut store, ty.element())?
                    .map(|r| r.unchecked_copy());
                let (gc_store, instance) = self.instance.get_with_gc_store_mut(&mut store);
                instance
                    .defined_table_grow(self.index, async |table| {
                        // SAFETY: in the context of `defined_table_grow` this
                        // is safe to call as it'll update the internal table
                        // pointer in the instance.
                        unsafe {
                            table
                                .grow_gc_ref(limiter, gc_store, delta, element.as_ref())
                                .await
                        }
                    })
                    .await?
            }
            // TODO(#10248) Required to support stack switching in the
            // embedder API.
            TableElementType::Cont => bail!("unimplemented table for cont"),
        };
        match result {
            Some(size) => {
                // unwrap here should be ok because the runtime should always
                // guarantee that we can fit the table size in a 64-bit integer.
                Ok(u64::try_from(size).unwrap())
            }
            None => bail!("failed to grow table by `{delta}`"),
        }
    }

    /// Async variant of [`Table::grow`].
    ///
    /// Required when using a
    /// [`ResourceLimiterAsync`](`crate::ResourceLimiterAsync`).
    ///
    /// # Panics
    ///
    /// This function will panic when if the store doens't own the table.
    #[cfg(feature = "async")]
    pub async fn grow_async(
        &self,
        mut store: impl AsContextMut,
        delta: u64,
        init: Ref,
    ) -> Result<u64> {
        self._grow(store.as_context_mut(), delta, init).await
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

        // SAFETY: the the two tables have the same type, as type-checked above.
        unsafe {
            Self::copy_raw(store, dst_table, dst_index, src_table, src_index, len)?;
        }
        Ok(())
    }

    /// Copies the elements of `src_table` to `dst_table`.
    ///
    /// # Panics
    ///
    /// Panics if the either table doesn't belong to `store`.
    ///
    /// # Safety
    ///
    /// Requires that the two tables have previously been type-checked to have
    /// the same type.
    pub(crate) unsafe fn copy_raw(
        store: &mut StoreOpaque,
        dst_table: &Table,
        dst_index: u64,
        src_table: &Table,
        src_index: u64,
        len: u64,
    ) -> Result<(), Trap> {
        // Handle lazy initialization of the source table first before doing
        // anything else.
        let src_range = src_index..(src_index.checked_add(len).unwrap_or(u64::MAX));
        src_table.wasmtime_table(store, src_range);

        // validate `dst_table` belongs to `store`.
        dst_table.wasmtime_table(store, iter::empty());

        // Figure out which of the three cases we're in:
        //
        // 1. Cross-instance table copy.
        // 2. Intra-instance table copy.
        // 3. Intra-table copy.
        //
        // We handle each of them slightly differently.
        let src_instance = src_table.instance.instance();
        let dst_instance = dst_table.instance.instance();
        match (
            src_instance == dst_instance,
            src_table.index == dst_table.index,
        ) {
            // 1. Cross-instance table copy: split the mutable store borrow into
            // two mutable instance borrows, get each instance's defined table,
            // and do the copy.
            (false, _) => {
                // SAFETY: accessing two instances mutably at the same time
                // requires only accessing defined entities on each instance
                // which is done below with `get_defined_*` methods.
                let (gc_store, [src_instance, dst_instance]) = unsafe {
                    store.optional_gc_store_and_instances_mut([src_instance, dst_instance])
                };
                src_instance.get_defined_table(src_table.index).copy_to(
                    dst_instance.get_defined_table(dst_table.index),
                    gc_store,
                    dst_index,
                    src_index,
                    len,
                )
            }

            // 2. Intra-instance, distinct-tables copy: split the mutable
            // instance borrow into two distinct mutable table borrows and do
            // the copy.
            (true, false) => {
                let (gc_store, instance) = store.optional_gc_store_and_instance_mut(src_instance);
                let [(_, src_table), (_, dst_table)] = instance
                    .tables_mut()
                    .get_disjoint_mut([src_table.index, dst_table.index])
                    .unwrap();
                src_table.copy_to(dst_table, gc_store, dst_index, src_index, len)
            }

            // 3. Intra-table copy: get the table and copy within it!
            (true, true) => {
                let (gc_store, instance) = store.optional_gc_store_and_instance_mut(src_instance);
                instance
                    .get_defined_table(src_table.index)
                    .copy_within(gc_store, dst_index, src_index, len)
            }
        }
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
        self._fill(store.as_context_mut().0, dst, val, len)
    }

    pub(crate) fn _fill(
        &self,
        store: &mut StoreOpaque,
        dst: u64,
        val: Ref,
        len: u64,
    ) -> Result<()> {
        let ty = self._ty(&store);
        match element_type(&ty) {
            TableElementType::Func => {
                let val = val.into_table_func(store, ty.element())?;
                let (table, _) = self.wasmtime_table(store, iter::empty());
                table.fill_func(dst, val, len)?;
            }
            TableElementType::GcRef => {
                // Note that `val` is a `VMGcRef` temporarily read from the
                // store here, and blocking GC with `AutoAssertNoGc` should
                // ensure that it's not collected while being worked on here.
                let mut store = AutoAssertNoGc::new(store);
                let val = val.into_table_gc_ref(&mut store, ty.element())?;
                let val = val.map(|g| g.unchecked_copy());
                let (table, gc_store) = self.wasmtime_table(&mut store, iter::empty());
                table.fill_gc_ref(gc_store, dst, val.as_ref(), len)?;
            }
            // TODO(#10248) Required to support stack switching in the embedder
            // API.
            TableElementType::Cont => bail!("unimplemented table for cont"),
        }

        Ok(())
    }

    #[cfg(feature = "gc")]
    pub(crate) fn trace_roots(&self, store: &mut StoreOpaque, gc_roots_list: &mut vm::GcRootsList) {
        if !self
            ._ty(store)
            .element()
            .is_vmgcref_type_and_points_to_object()
        {
            return;
        }

        let (table, _) = self.wasmtime_table(store, iter::empty());
        for gc_ref in table.gc_refs_mut() {
            if let Some(gc_ref) = gc_ref {
                unsafe {
                    gc_roots_list.add_root(gc_ref.into(), "Wasm table element");
                }
            }
        }
    }

    pub(crate) fn from_raw(instance: StoreInstanceId, index: DefinedTableIndex) -> Table {
        Table { instance, index }
    }

    pub(crate) fn wasmtime_ty<'a>(&self, store: &'a StoreOpaque) -> &'a wasmtime_environ::Table {
        let module = store[self.instance].env_module();
        let index = module.table_index(self.index);
        &module.tables[index]
    }

    pub(crate) fn vmimport(&self, store: &StoreOpaque) -> vm::VMTableImport {
        let instance = &store[self.instance];
        vm::VMTableImport {
            from: instance.table_ptr(self.index).into(),
            vmctx: instance.vmctx().into(),
            index: self.index,
        }
    }

    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        store.id() == self.instance.store_id()
    }

    /// Get a stable hash key for this table.
    ///
    /// Even if the same underlying table definition is added to the
    /// `StoreData` multiple times and becomes multiple `wasmtime::Table`s,
    /// this hash key will be consistent across all of these tables.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "Not used yet, but added for consistency")
    )]
    pub(crate) fn hash_key(&self, store: &StoreOpaque) -> impl core::hash::Hash + Eq + use<'_> {
        store[self.instance].table_ptr(self.index).as_ptr().addr()
    }
}

fn element_type(ty: &TableType) -> TableElementType {
    match ty.element().heap_type().top() {
        HeapType::Func => TableElementType::Func,
        HeapType::Exn | HeapType::Extern | HeapType::Any => TableElementType::GcRef,
        HeapType::Cont => TableElementType::Cont,
        _ => unreachable!(),
    }
}

impl Ref {
    fn into_table_func(
        self,
        store: &mut StoreOpaque,
        ty: &RefType,
    ) -> Result<Option<NonNull<VMFuncRef>>> {
        self.ensure_matches_ty(store, &ty)
            .context("type mismatch: value does not match table element type")?;

        match (self, ty.heap_type().top()) {
            (Ref::Func(None), HeapType::Func) => {
                assert!(ty.is_nullable());
                Ok(None)
            }
            (Ref::Func(Some(f)), HeapType::Func) => {
                debug_assert!(
                    f.comes_from_same_store(store),
                    "checked in `ensure_matches_ty`"
                );
                Ok(Some(f.vm_func_ref(store)))
            }

            _ => unreachable!("checked that the value matches the type above"),
        }
    }

    fn into_table_gc_ref<'a>(
        self,
        store: &'a mut AutoAssertNoGc<'_>,
        ty: &RefType,
    ) -> Result<Option<&'a VMGcRef>> {
        self.ensure_matches_ty(store, &ty)
            .context("type mismatch: value does not match table element type")?;

        match (self, ty.heap_type().top()) {
            (Ref::Extern(e), HeapType::Extern) => match e {
                None => {
                    assert!(ty.is_nullable());
                    Ok(None)
                }
                Some(e) => Ok(Some(e.try_gc_ref(store)?)),
            },

            (Ref::Any(a), HeapType::Any) => match a {
                None => {
                    assert!(ty.is_nullable());
                    Ok(None)
                }
                Some(a) => Ok(Some(a.try_gc_ref(store)?)),
            },

            (Ref::Exn(e), HeapType::Exn) => match e {
                None => {
                    assert!(ty.is_nullable());
                    Ok(None)
                }
                Some(e) => Ok(Some(e.try_gc_ref(store)?)),
            },

            _ => unreachable!("checked that the value matches the type above"),
        }
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

    #[test]
    fn grow_is_send() {
        fn _assert_send<T: Send>(_: T) {}
        fn _grow(table: &Table, store: &mut Store<()>, init: Ref) {
            _assert_send(table.grow(store, 0, init))
        }
    }
}
