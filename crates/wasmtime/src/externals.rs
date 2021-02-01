use crate::memory::Memory;
use crate::trampoline::{generate_global_export, generate_table_export, StoreInstanceHandle};
use crate::values::{from_checked_anyfunc, into_checked_anyfunc, Val};
use crate::{
    ExternRef, ExternType, Func, GlobalType, Instance, Module, Mutability, Store, TableType, Trap,
    ValType,
};
use anyhow::{anyhow, bail, Result};
use std::mem;
use std::ptr;
use wasmtime_environ::wasm;
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
    /// A WebAssembly `table` which is an array of `Val` types.
    Table(Table),
    /// A WebAssembly linear memory.
    Memory(Memory),
    /// A WebAssembly instance.
    Instance(Instance),
    /// A WebAssembly module.
    Module(Module),
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

    /// Returns the underlying `Instance`, if this external is a instance.
    ///
    /// Returns `None` if this is not a instance.
    pub fn into_instance(self) -> Option<Instance> {
        match self {
            Extern::Instance(instance) => Some(instance),
            _ => None,
        }
    }

    /// Returns the underlying `Module`, if this external is a module.
    ///
    /// Returns `None` if this is not a module.
    pub fn into_module(self) -> Option<Module> {
        match self {
            Extern::Module(module) => Some(module),
            _ => None,
        }
    }

    /// Returns the type associated with this `Extern`.
    pub fn ty(&self) -> ExternType {
        match self {
            Extern::Func(ft) => ExternType::Func(ft.ty()),
            Extern::Memory(ft) => ExternType::Memory(ft.ty()),
            Extern::Table(tt) => ExternType::Table(tt.ty()),
            Extern::Global(gt) => ExternType::Global(gt.ty()),
            Extern::Instance(i) => ExternType::Instance(i.ty()),
            Extern::Module(m) => ExternType::Module(m.ty()),
        }
    }

    pub(crate) unsafe fn from_wasmtime_export(
        wasmtime_export: &wasmtime_runtime::Export,
        store: &Store,
    ) -> Extern {
        match wasmtime_export {
            wasmtime_runtime::Export::Function(f) => {
                Extern::Func(Func::from_wasmtime_function(f, store))
            }
            wasmtime_runtime::Export::Memory(m) => {
                Extern::Memory(Memory::from_wasmtime_memory(m, store))
            }
            wasmtime_runtime::Export::Global(g) => {
                Extern::Global(Global::from_wasmtime_global(g, store))
            }
            wasmtime_runtime::Export::Table(t) => {
                Extern::Table(Table::from_wasmtime_table(t, store))
            }
            wasmtime_runtime::Export::Instance(i) => {
                Extern::Instance(Instance::from_wasmtime(i, store))
            }
            wasmtime_runtime::Export::Module(m) => {
                Extern::Module(m.downcast_ref::<Module>().unwrap().clone())
            }
        }
    }

    pub(crate) fn comes_from_same_store(&self, store: &Store) -> bool {
        let my_store = match self {
            Extern::Func(f) => f.store(),
            Extern::Global(g) => &g.instance.store,
            Extern::Memory(m) => &m.instance.store,
            Extern::Table(t) => &t.instance.store,
            Extern::Instance(i) => i.store(),
            // Modules don't live in stores right now, so they're compatible
            // with all stores.
            Extern::Module(_) => return true,
        };
        Store::same(my_store, store)
    }

    pub(crate) fn desc(&self) -> &'static str {
        match self {
            Extern::Func(_) => "function",
            Extern::Table(_) => "table",
            Extern::Memory(_) => "memory",
            Extern::Global(_) => "global",
            Extern::Instance(_) => "instance",
            Extern::Module(_) => "module",
        }
    }

    pub(crate) fn wasmtime_export(&self) -> wasmtime_runtime::Export {
        match self {
            Extern::Func(f) => f.wasmtime_export().clone().into(),
            Extern::Global(f) => f.wasmtime_export().clone().into(),
            Extern::Table(f) => f.wasmtime_export().clone().into(),
            Extern::Memory(f) => f.wasmtime_export().clone().into(),
            Extern::Instance(f) => wasmtime_runtime::Export::Instance(f.wasmtime_export().clone()),
            Extern::Module(f) => wasmtime_runtime::Export::Module(Box::new(f.clone())),
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

impl From<Table> for Extern {
    fn from(r: Table) -> Self {
        Extern::Table(r)
    }
}

impl From<Instance> for Extern {
    fn from(r: Instance) -> Self {
        Extern::Instance(r)
    }
}

impl From<Module> for Extern {
    fn from(r: Module) -> Self {
        Extern::Module(r)
    }
}

/// A WebAssembly `global` value which can be read and written to.
///
/// A `global` in WebAssembly is sort of like a global variable within an
/// [`Instance`](crate::Instance). The `global.get` and `global.set`
/// instructions will modify and read global values in a wasm module. Globals
/// can either be imported or exported from wasm modules.
///
/// If you're familiar with Rust already you can think of a `Global` as a sort
/// of `Rc<Cell<Val>>`, more or less.
///
/// # `Global` and `Clone`
///
/// Globals are internally reference counted so you can `clone` a `Global`. The
/// cloning process only performs a shallow clone, so two cloned `Global`
/// instances are equivalent in their functionality.
#[derive(Clone)]
pub struct Global {
    instance: StoreInstanceHandle,
    wasmtime_export: wasmtime_runtime::ExportGlobal,
}

impl Global {
    /// Creates a new WebAssembly `global` value with the provide type `ty` and
    /// initial value `val`.
    ///
    /// The `store` argument provided is used as a general global cache for
    /// information, and otherwise the `ty` and `val` arguments are used to
    /// initialize the global.
    ///
    /// # Errors
    ///
    /// Returns an error if the `ty` provided does not match the type of the
    /// value `val`.
    pub fn new(store: &Store, ty: GlobalType, val: Val) -> Result<Global> {
        if !val.comes_from_same_store(store) {
            bail!("cross-`Store` globals are not supported");
        }
        if val.ty() != *ty.content() {
            bail!("value provided does not match the type of this global");
        }
        let (instance, wasmtime_export) = generate_global_export(store, &ty, val)?;
        Ok(Global {
            instance,
            wasmtime_export,
        })
    }

    /// Returns the underlying type of this `global`.
    pub fn ty(&self) -> GlobalType {
        // The original export is coming from wasmtime_runtime itself we should
        // support all the types coming out of it, so assert such here.
        GlobalType::from_wasmtime_global(&self.wasmtime_export.global)
    }

    /// Returns the value type of this `global`.
    pub fn val_type(&self) -> ValType {
        ValType::from_wasm_type(&self.wasmtime_export.global.wasm_ty)
    }

    /// Returns the underlying mutability of this `global`.
    pub fn mutability(&self) -> Mutability {
        if self.wasmtime_export.global.mutability {
            Mutability::Var
        } else {
            Mutability::Const
        }
    }

    /// Returns the current [`Val`] of this global.
    pub fn get(&self) -> Val {
        unsafe {
            let definition = &mut *self.wasmtime_export.definition;
            match self.val_type() {
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
                    from_checked_anyfunc(definition.as_anyfunc() as *mut _, &self.instance.store)
                }
                ty => unimplemented!("Global::get for {:?}", ty),
            }
        }
    }

    /// Attempts to set the current value of this global to [`Val`].
    ///
    /// # Errors
    ///
    /// Returns an error if this global has a different type than `Val`, or if
    /// it's not a mutable global.
    pub fn set(&self, val: Val) -> Result<()> {
        if self.mutability() != Mutability::Var {
            bail!("immutable global cannot be set");
        }
        let ty = self.val_type();
        if val.ty() != ty {
            bail!("global of type {:?} cannot be set to {:?}", ty, val.ty());
        }
        if !val.comes_from_same_store(&self.instance.store) {
            bail!("cross-`Store` values are not supported");
        }
        unsafe {
            let definition = &mut *self.wasmtime_export.definition;
            match val {
                Val::I32(i) => *definition.as_i32_mut() = i,
                Val::I64(i) => *definition.as_i64_mut() = i,
                Val::F32(f) => *definition.as_u32_mut() = f,
                Val::F64(f) => *definition.as_u64_mut() = f,
                Val::FuncRef(f) => {
                    *definition.as_anyfunc_mut() = f.map_or(ptr::null(), |f| {
                        f.caller_checked_anyfunc().as_ptr() as *const _
                    });
                }
                Val::ExternRef(x) => {
                    // In case the old value's `Drop` implementation is
                    // re-entrant and tries to touch this global again, do a
                    // replace, and then drop. This way no one can observe a
                    // halfway-deinitialized value.
                    let old = mem::replace(definition.as_externref_mut(), x.map(|x| x.inner));
                    drop(old);
                }
                _ => unimplemented!("Global::set for {:?}", val.ty()),
            }
        }
        Ok(())
    }

    pub(crate) unsafe fn from_wasmtime_global(
        wasmtime_export: &wasmtime_runtime::ExportGlobal,
        store: &Store,
    ) -> Global {
        Global {
            instance: store.existing_vmctx(wasmtime_export.vmctx),
            wasmtime_export: wasmtime_export.clone(),
        }
    }

    pub(crate) fn wasmtime_ty(&self) -> &wasmtime_environ::wasm::Global {
        &self.wasmtime_export.global
    }

    pub(crate) fn vmimport(&self) -> wasmtime_runtime::VMGlobalImport {
        wasmtime_runtime::VMGlobalImport {
            from: self.wasmtime_export.definition,
        }
    }

    pub(crate) fn wasmtime_export(&self) -> &wasmtime_runtime::ExportGlobal {
        &self.wasmtime_export
    }
}

/// A WebAssembly `table`, or an array of values.
///
/// Like [`Memory`] a table is an indexed array of values, but unlike [`Memory`]
/// it's an array of WebAssembly values rather than bytes. One of the most
/// common usages of a table is a function table for wasm modules, where each
/// element has the `Func` type.
///
/// Tables, like globals, are not threadsafe and can only be used on one thread.
/// Tables can be grown in size and each element can be read/written.
///
/// # `Table` and `Clone`
///
/// Tables are internally reference counted so you can `clone` a `Table`. The
/// cloning process only performs a shallow clone, so two cloned `Table`
/// instances are equivalent in their functionality.
#[derive(Clone)]
pub struct Table {
    instance: StoreInstanceHandle,
    wasmtime_export: wasmtime_runtime::ExportTable,
}

fn set_table_item(
    instance: &InstanceHandle,
    table_index: wasm::DefinedTableIndex,
    item_index: u32,
    item: runtime::TableElement,
) -> Result<()> {
    instance
        .table_set(table_index, item_index, item)
        .map_err(|()| anyhow!("table element index out of bounds"))
}

impl Table {
    /// Creates a new `Table` with the given parameters.
    ///
    /// * `store` - a global cache to store information in
    /// * `ty` - the type of this table, containing both the element type as
    ///   well as the initial size and maximum size, if any.
    /// * `init` - the initial value to fill all table entries with, if the
    ///   table starts with an initial size.
    ///
    /// # Errors
    ///
    /// Returns an error if `init` does not match the element type of the table.
    pub fn new(store: &Store, ty: TableType, init: Val) -> Result<Table> {
        let (instance, wasmtime_export) = generate_table_export(store, &ty)?;

        let init: runtime::TableElement = match ty.element() {
            ValType::FuncRef => into_checked_anyfunc(init, store)?.into(),
            ValType::ExternRef => init
                .externref()
                .ok_or_else(|| {
                    anyhow!("table initialization value does not have expected type `externref`")
                })?
                .map(|x| x.inner)
                .into(),
            ty => bail!("unsupported table element type: {:?}", ty),
        };

        // Initialize entries with the init value.
        let definition = unsafe { &*wasmtime_export.definition };
        let index = instance.table_index(definition);
        for i in 0..definition.current_elements {
            set_table_item(&instance, index, i, init.clone())?;
        }

        Ok(Table {
            instance,
            wasmtime_export,
        })
    }

    /// Returns the underlying type of this table, including its element type as
    /// well as the maximum/minimum lower bounds.
    pub fn ty(&self) -> TableType {
        TableType::from_wasmtime_table(&self.wasmtime_export.table.table)
    }

    fn wasmtime_table_index(&self) -> wasm::DefinedTableIndex {
        unsafe { self.instance.table_index(&*self.wasmtime_export.definition) }
    }

    /// Returns the table element value at `index`.
    ///
    /// Returns `None` if `index` is out of bounds.
    pub fn get(&self, index: u32) -> Option<Val> {
        let table_index = self.wasmtime_table_index();
        let item = self.instance.table_get(table_index, index)?;
        match item {
            runtime::TableElement::FuncRef(f) => {
                Some(unsafe { from_checked_anyfunc(f, &self.instance.store) })
            }
            runtime::TableElement::ExternRef(None) => Some(Val::ExternRef(None)),
            runtime::TableElement::ExternRef(Some(x)) => {
                Some(Val::ExternRef(Some(ExternRef { inner: x })))
            }
        }
    }

    /// Writes the `val` provided into `index` within this table.
    ///
    /// # Errors
    ///
    /// Returns an error if `index` is out of bounds or if `val` does not have
    /// the right type to be stored in this table.
    pub fn set(&self, index: u32, val: Val) -> Result<()> {
        if !val.comes_from_same_store(&self.instance.store) {
            bail!("cross-`Store` values are not supported in tables");
        }
        let table_index = self.wasmtime_table_index();
        set_table_item(
            &self.instance,
            table_index,
            index,
            val.into_table_element()?,
        )
    }

    /// Returns the current size of this table.
    pub fn size(&self) -> u32 {
        unsafe { (*self.wasmtime_export.definition).current_elements }
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
    /// error if `init` is not of the right type.
    pub fn grow(&self, delta: u32, init: Val) -> Result<u32> {
        let index = self.wasmtime_table_index();
        let orig_size = match self.ty().element() {
            ValType::FuncRef => {
                let init = into_checked_anyfunc(init, &self.instance.store)?;
                self.instance.defined_table_grow(index, delta, init.into())
            }
            ValType::ExternRef => {
                let init = match init {
                    Val::ExternRef(Some(x)) => Some(x.inner),
                    Val::ExternRef(None) => None,
                    _ => bail!("incorrect init value for growing table"),
                };
                self.instance.defined_table_grow(
                    index,
                    delta,
                    runtime::TableElement::ExternRef(init),
                )
            }
            _ => unreachable!("only `funcref` and `externref` tables are supported"),
        };
        if let Some(size) = orig_size {
            Ok(size)
        } else {
            bail!("failed to grow table by `{}`", delta)
        }
    }

    /// Copy `len` elements from `src_table[src_index..]` into
    /// `dst_table[dst_index..]`.
    ///
    /// # Errors
    ///
    /// Returns an error if the range is out of bounds of either the source or
    /// destination tables.
    pub fn copy(
        dst_table: &Table,
        dst_index: u32,
        src_table: &Table,
        src_index: u32,
        len: u32,
    ) -> Result<()> {
        if !Store::same(&dst_table.instance.store, &src_table.instance.store) {
            bail!("cross-`Store` table copies are not supported");
        }

        // NB: We must use the `dst_table`'s `wasmtime_handle` for the
        // `dst_table_index` and vice versa for `src_table` since each table can
        // come from different modules.

        let dst_table_index = dst_table.wasmtime_table_index();
        let dst_table_index = dst_table.instance.get_defined_table(dst_table_index);

        let src_table_index = src_table.wasmtime_table_index();
        let src_table_index = src_table.instance.get_defined_table(src_table_index);

        runtime::Table::copy(dst_table_index, src_table_index, dst_index, src_index, len)
            .map_err(|e| Trap::from_runtime(&dst_table.instance.store, e))?;
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
    pub fn fill(&self, dst: u32, val: Val, len: u32) -> Result<()> {
        if !val.comes_from_same_store(&self.instance.store) {
            bail!("cross-`Store` table fills are not supported");
        }

        let table_index = self.wasmtime_table_index();
        self.instance
            .handle
            .defined_table_fill(table_index, dst, val.into_table_element()?, len)
            .map_err(|e| Trap::from_runtime(&self.instance.store, e))?;

        Ok(())
    }

    pub(crate) unsafe fn from_wasmtime_table(
        wasmtime_export: &wasmtime_runtime::ExportTable,
        store: &Store,
    ) -> Table {
        Table {
            instance: store.existing_vmctx(wasmtime_export.vmctx),
            wasmtime_export: wasmtime_export.clone(),
        }
    }

    pub(crate) fn wasmtime_ty(&self) -> &wasmtime_environ::wasm::Table {
        &self.wasmtime_export.table.table
    }

    pub(crate) fn vmimport(&self) -> wasmtime_runtime::VMTableImport {
        wasmtime_runtime::VMTableImport {
            from: self.wasmtime_export.definition,
            vmctx: self.wasmtime_export.vmctx,
        }
    }

    pub(crate) fn wasmtime_export(&self) -> &wasmtime_runtime::ExportTable {
        &self.wasmtime_export
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
    pub fn ty(&self) -> ExternType {
        self.definition.ty()
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
