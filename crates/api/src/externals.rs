use crate::callable::{Callable, NativeCallable, WasmtimeFn, WrappedCallable};
use crate::runtime::Store;
use crate::trampoline::{generate_global_export, generate_memory_export, generate_table_export};
use crate::trap::Trap;
use crate::types::{ExternType, FuncType, GlobalType, MemoryType, TableType, ValType};
use crate::values::{from_checked_anyfunc, into_checked_anyfunc, Val};
use crate::Mutability;
use anyhow::{anyhow, bail, Result};
use std::fmt;
use std::rc::Rc;
use std::slice;
use wasmtime_environ::wasm;
use wasmtime_runtime::InstanceHandle;

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
}

impl Extern {
    /// Returns the underlying `Func`, if this external is a function.
    ///
    /// Returns `None` if this is not a function.
    pub fn func(&self) -> Option<&Func> {
        match self {
            Extern::Func(func) => Some(func),
            _ => None,
        }
    }

    /// Returns the underlying `Global`, if this external is a global.
    ///
    /// Returns `None` if this is not a global.
    pub fn global(&self) -> Option<&Global> {
        match self {
            Extern::Global(global) => Some(global),
            _ => None,
        }
    }

    /// Returns the underlying `Table`, if this external is a table.
    ///
    /// Returns `None` if this is not a table.
    pub fn table(&self) -> Option<&Table> {
        match self {
            Extern::Table(table) => Some(table),
            _ => None,
        }
    }

    /// Returns the underlying `Memory`, if this external is a memory.
    ///
    /// Returns `None` if this is not a memory.
    pub fn memory(&self) -> Option<&Memory> {
        match self {
            Extern::Memory(memory) => Some(memory),
            _ => None,
        }
    }

    /// Returns the type associated with this `Extern`.
    pub fn ty(&self) -> ExternType {
        match self {
            Extern::Func(ft) => ExternType::Func(ft.ty().clone()),
            Extern::Memory(ft) => ExternType::Memory(ft.ty().clone()),
            Extern::Table(tt) => ExternType::Table(tt.ty().clone()),
            Extern::Global(gt) => ExternType::Global(gt.ty().clone()),
        }
    }

    pub(crate) fn get_wasmtime_export(&self) -> wasmtime_runtime::Export {
        match self {
            Extern::Func(f) => f.wasmtime_export().clone(),
            Extern::Global(g) => g.wasmtime_export().clone(),
            Extern::Memory(m) => m.wasmtime_export().clone(),
            Extern::Table(t) => t.wasmtime_export().clone(),
        }
    }

    pub(crate) fn from_wasmtime_export(
        store: &Store,
        instance_handle: InstanceHandle,
        export: wasmtime_runtime::Export,
    ) -> Extern {
        match export {
            wasmtime_runtime::Export::Function { .. } => {
                Extern::Func(Func::from_wasmtime_function(export, store, instance_handle))
            }
            wasmtime_runtime::Export::Memory { .. } => {
                Extern::Memory(Memory::from_wasmtime_memory(export, store, instance_handle))
            }
            wasmtime_runtime::Export::Global { .. } => {
                Extern::Global(Global::from_wasmtime_global(export, store))
            }
            wasmtime_runtime::Export::Table { .. } => {
                Extern::Table(Table::from_wasmtime_table(export, store, instance_handle))
            }
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

/// A WebAssembly function which can be called.
///
/// This type can represent a number of callable items, such as:
///
/// * An exported function from a WebAssembly module.
/// * A user-defined function used to satisfy an import.
///
/// These types of callable items are all wrapped up in this `Func` and can be
/// used to both instantiate an [`Instance`](crate::Instance) as well as be
/// extracted from an [`Instance`](crate::Instance).
///
/// # `Func` and `Clone`
///
/// Functions are internally reference counted so you can `clone` a `Func`. The
/// cloning process only performs a shallow clone, so two cloned `Func`
/// instances are equivalent in their functionality.
#[derive(Clone)]
pub struct Func {
    _store: Store,
    callable: Rc<dyn WrappedCallable + 'static>,
    ty: FuncType,
}

impl Func {
    /// Creates a new `Func` with the given arguments, typically to create a
    /// user-defined function to pass as an import to a module.
    ///
    /// * `store` - a cache of data where information is stored, typically
    ///   shared with a [`Module`](crate::Module).
    ///
    /// * `ty` - the signature of this function, used to indicate what the
    ///   inputs and outputs are, which must be WebAssembly types.
    ///
    /// * `callable` - a type implementing the [`Callable`] trait which
    ///   is the implementation of this `Func` value.
    ///
    /// Note that the implementation of `callable` must adhere to the `ty`
    /// signature given, error or traps may occur if it does not respect the
    /// `ty` signature.
    pub fn new(store: &Store, ty: FuncType, callable: Rc<dyn Callable + 'static>) -> Self {
        let callable = Rc::new(NativeCallable::new(callable, &ty, &store));
        Func::from_wrapped(store, ty, callable)
    }

    fn from_wrapped(
        store: &Store,
        ty: FuncType,
        callable: Rc<dyn WrappedCallable + 'static>,
    ) -> Func {
        Func {
            _store: store.clone(),
            callable,
            ty,
        }
    }

    /// Returns the underlying wasm type that this `Func` has.
    pub fn ty(&self) -> &FuncType {
        &self.ty
    }

    /// Returns the number of parameters that this function takes.
    pub fn param_arity(&self) -> usize {
        self.ty.params().len()
    }

    /// Returns the number of results this function produces.
    pub fn result_arity(&self) -> usize {
        self.ty.results().len()
    }

    /// Invokes this function with the `params` given, returning the results and
    /// any trap, if one occurs.
    ///
    /// The `params` here must match the type signature of this `Func`, or a
    /// trap will occur. If a trap occurs while executing this function, then a
    /// trap will also be returned.
    ///
    /// This function should not panic unless the underlying function itself
    /// initiates a panic.
    pub fn call(&self, params: &[Val]) -> Result<Box<[Val]>, Trap> {
        let mut results = vec![Val::null(); self.result_arity()];
        self.callable.call(params, &mut results)?;
        Ok(results.into_boxed_slice())
    }

    pub(crate) fn wasmtime_export(&self) -> &wasmtime_runtime::Export {
        self.callable.wasmtime_export()
    }

    pub(crate) fn from_wasmtime_function(
        export: wasmtime_runtime::Export,
        store: &Store,
        instance_handle: InstanceHandle,
    ) -> Self {
        // This is only called with `Export::Function`, and since it's coming
        // from wasmtime_runtime itself we should support all the types coming
        // out of it, so assert such here.
        let ty = if let wasmtime_runtime::Export::Function { signature, .. } = &export {
            FuncType::from_wasmtime_signature(signature.clone())
                .expect("core wasm signature should be supported")
        } else {
            panic!("expected function export")
        };
        let callable = WasmtimeFn::new(store, instance_handle, export);
        Func::from_wrapped(store, ty, Rc::new(callable))
    }
}

impl fmt::Debug for Func {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Func")
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
    inner: Rc<GlobalInner>,
}

struct GlobalInner {
    _store: Store,
    ty: GlobalType,
    wasmtime_export: wasmtime_runtime::Export,
    #[allow(dead_code)]
    wasmtime_state: Option<crate::trampoline::GlobalState>,
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
        if val.ty() != *ty.content() {
            bail!("value provided does not match the type of this global");
        }
        let (wasmtime_export, wasmtime_state) = generate_global_export(&ty, val)?;
        Ok(Global {
            inner: Rc::new(GlobalInner {
                _store: store.clone(),
                ty,
                wasmtime_export,
                wasmtime_state: Some(wasmtime_state),
            }),
        })
    }

    /// Returns the underlying type of this `global`.
    pub fn ty(&self) -> &GlobalType {
        &self.inner.ty
    }

    fn wasmtime_global_definition(&self) -> *mut wasmtime_runtime::VMGlobalDefinition {
        match self.inner.wasmtime_export {
            wasmtime_runtime::Export::Global { definition, .. } => definition,
            _ => panic!("global definition not found"),
        }
    }

    /// Returns the current [`Val`] of this global.
    pub fn get(&self) -> Val {
        let definition = unsafe { &mut *self.wasmtime_global_definition() };
        unsafe {
            match self.ty().content() {
                ValType::I32 => Val::from(*definition.as_i32()),
                ValType::I64 => Val::from(*definition.as_i64()),
                ValType::F32 => Val::F32(*definition.as_u32()),
                ValType::F64 => Val::F64(*definition.as_u64()),
                _ => unimplemented!("Global::get for {:?}", self.ty().content()),
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
        if self.ty().mutability() != Mutability::Var {
            bail!("immutable global cannot be set");
        }
        if val.ty() != *self.ty().content() {
            bail!(
                "global of type {:?} cannot be set to {:?}",
                self.ty().content(),
                val.ty()
            );
        }
        let definition = unsafe { &mut *self.wasmtime_global_definition() };
        unsafe {
            match val {
                Val::I32(i) => *definition.as_i32_mut() = i,
                Val::I64(i) => *definition.as_i64_mut() = i,
                Val::F32(f) => *definition.as_u32_mut() = f,
                Val::F64(f) => *definition.as_u64_mut() = f,
                _ => unimplemented!("Global::set for {:?}", val.ty()),
            }
        }
        Ok(())
    }

    pub(crate) fn wasmtime_export(&self) -> &wasmtime_runtime::Export {
        &self.inner.wasmtime_export
    }

    pub(crate) fn from_wasmtime_global(export: wasmtime_runtime::Export, store: &Store) -> Global {
        let global = if let wasmtime_runtime::Export::Global { ref global, .. } = export {
            global
        } else {
            panic!("wasmtime export is not global")
        };
        // The original export is coming from wasmtime_runtime itself we should
        // support all the types coming out of it, so assert such here.
        let ty = GlobalType::from_wasmtime_global(&global)
            .expect("core wasm global type should be supported");
        Global {
            inner: Rc::new(GlobalInner {
                _store: store.clone(),
                ty: ty,
                wasmtime_export: export,
                wasmtime_state: None,
            }),
        }
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
    store: Store,
    ty: TableType,
    wasmtime_handle: InstanceHandle,
    wasmtime_export: wasmtime_runtime::Export,
}

fn set_table_item(
    handle: &mut InstanceHandle,
    table_index: wasm::DefinedTableIndex,
    item_index: u32,
    item: wasmtime_runtime::VMCallerCheckedAnyfunc,
) -> Result<()> {
    handle
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
        let item = into_checked_anyfunc(init, store)?;
        let (mut wasmtime_handle, wasmtime_export) = generate_table_export(&ty)?;

        // Initialize entries with the init value.
        match wasmtime_export {
            wasmtime_runtime::Export::Table { definition, .. } => {
                let index = wasmtime_handle.table_index(unsafe { &*definition });
                let len = unsafe { (*definition).current_elements };
                for i in 0..len {
                    set_table_item(&mut wasmtime_handle, index, i, item.clone())?;
                }
            }
            _ => unreachable!("export should be a table"),
        }

        Ok(Table {
            store: store.clone(),
            ty,
            wasmtime_handle,
            wasmtime_export,
        })
    }

    /// Returns the underlying type of this table, including its element type as
    /// well as the maximum/minimum lower bounds.
    pub fn ty(&self) -> &TableType {
        &self.ty
    }

    fn wasmtime_table_index(&self) -> wasm::DefinedTableIndex {
        match self.wasmtime_export {
            wasmtime_runtime::Export::Table { definition, .. } => {
                self.wasmtime_handle.table_index(unsafe { &*definition })
            }
            _ => panic!("global definition not found"),
        }
    }

    /// Returns the table element value at `index`.
    ///
    /// Returns `None` if `index` is out of bounds.
    pub fn get(&self, index: u32) -> Option<Val> {
        let table_index = self.wasmtime_table_index();
        let item = self.wasmtime_handle.table_get(table_index, index)?;
        Some(from_checked_anyfunc(item, &self.store))
    }

    /// Writes the `val` provided into `index` within this table.
    ///
    /// # Errors
    ///
    /// Returns an error if `index` is out of bounds or if `val` does not have
    /// the right type to be stored in this table.
    pub fn set(&self, index: u32, val: Val) -> Result<()> {
        let table_index = self.wasmtime_table_index();
        let mut wasmtime_handle = self.wasmtime_handle.clone();
        let item = into_checked_anyfunc(val, &self.store)?;
        set_table_item(&mut wasmtime_handle, table_index, index, item)
    }

    /// Returns the current size of this table.
    pub fn size(&self) -> u32 {
        match self.wasmtime_export {
            wasmtime_runtime::Export::Table { definition, .. } => unsafe {
                (*definition).current_elements
            },
            _ => panic!("global definition not found"),
        }
    }

    /// Grows the size of this table by `delta` more elements, initialization
    /// all new elements to `init`.
    ///
    /// # Errors
    ///
    /// Returns an error if the table cannot be grown by `delta`, for example
    /// if it would cause the table to exceed its maximum size. Also returns an
    /// error if `init` is not of the right type.
    pub fn grow(&self, delta: u32, init: Val) -> Result<u32> {
        let index = self.wasmtime_table_index();
        let item = into_checked_anyfunc(init, &self.store)?;
        if let Some(len) = self.wasmtime_handle.clone().table_grow(index, delta) {
            let mut wasmtime_handle = self.wasmtime_handle.clone();
            for i in 0..delta {
                let i = len - (delta - i);
                set_table_item(&mut wasmtime_handle, index, i, item.clone())?;
            }
            Ok(len)
        } else {
            bail!("failed to grow table by `{}`", delta)
        }
    }

    pub(crate) fn wasmtime_export(&self) -> &wasmtime_runtime::Export {
        &self.wasmtime_export
    }

    pub(crate) fn from_wasmtime_table(
        export: wasmtime_runtime::Export,
        store: &Store,
        instance_handle: wasmtime_runtime::InstanceHandle,
    ) -> Table {
        let table = if let wasmtime_runtime::Export::Table { ref table, .. } = export {
            table
        } else {
            panic!("wasmtime export is not table")
        };
        let ty = TableType::from_wasmtime_table(&table.table);
        Table {
            store: store.clone(),
            ty: ty,
            wasmtime_handle: instance_handle,
            wasmtime_export: export,
        }
    }
}

/// A WebAssembly linear memory.
///
/// WebAssembly memories represent a contiguous array of bytes that have a size
/// that is always a multiple of the WebAssembly page size, currently 64
/// kilobytes.
///
/// WebAssembly memory is used for global data, statics in C/C++/Rust, shadow
/// stack memory, etc. Accessing wasm memory is generally quite fast!
///
/// # `Memory` and `Clone`
///
/// Memories are internally reference counted so you can `clone` a `Memory`. The
/// cloning process only performs a shallow clone, so two cloned `Memory`
/// instances are equivalent in their functionality.
///
/// # `Memory` and threads
///
/// It is intended that `Memory` is safe to share between threads. At this time
/// this is not implemented in `wasmtime`, however. This is planned to be
/// implemented though!
#[derive(Clone)]
pub struct Memory {
    _store: Store,
    ty: MemoryType,
    wasmtime_handle: InstanceHandle,
    wasmtime_export: wasmtime_runtime::Export,
}

impl Memory {
    /// Creates a new WebAssembly memory given the configuration of `ty`.
    ///
    /// The `store` argument is a general location for cache information, and
    /// otherwise the memory will immediately be allocated according to the
    /// type's configuration. All WebAssembly memory is initialized to zero.
    pub fn new(store: &Store, ty: MemoryType) -> Memory {
        let (wasmtime_handle, wasmtime_export) =
            generate_memory_export(&ty).expect("generated memory");
        Memory {
            _store: store.clone(),
            ty,
            wasmtime_handle,
            wasmtime_export,
        }
    }

    /// Returns the underlying type of this memory.
    pub fn ty(&self) -> &MemoryType {
        &self.ty
    }

    fn wasmtime_memory_definition(&self) -> *mut wasmtime_runtime::VMMemoryDefinition {
        match self.wasmtime_export {
            wasmtime_runtime::Export::Memory { definition, .. } => definition,
            _ => panic!("memory definition not found"),
        }
    }

    /// Returns this memory as a slice view that can be read natively in Rust.
    ///
    /// # Safety
    ///
    /// This is an unsafe operation because there is no guarantee that the
    /// following operations do not happen concurrently while the slice is in
    /// use:
    ///
    /// * Data could be modified by calling into a wasm module.
    /// * Memory could be relocated through growth by calling into a wasm
    ///   module.
    /// * When threads are supported, non-atomic reads will race with other
    ///   writes.
    ///
    /// Extreme care need be taken when the data of a `Memory` is read. The
    /// above invariants all need to be upheld at a bare minimum, and in
    /// general you'll need to ensure that while you're looking at slice you're
    /// the only one who can possibly look at the slice and read/write it.
    ///
    /// Be sure to keep in mind that `Memory` is reference counted, meaning
    /// that there may be other users of this `Memory` instance elsewhere in
    /// your program. Additionally `Memory` can be shared and used in any number
    /// of wasm instances, so calling any wasm code should be considered
    /// dangerous while you're holding a slice of memory.
    pub unsafe fn data_unchecked(&self) -> &[u8] {
        self.data_unchecked_mut()
    }

    /// Returns this memory as a slice view that can be read and written
    /// natively in Rust.
    ///
    /// # Safety
    ///
    /// All of the same safety caveats of [`Memory::data_unchecked`] apply
    /// here, doubly so because this is returning a mutable slice! As a
    /// double-extra reminder, remember that `Memory` is reference counted, so
    /// you can very easily acquire two mutable slices by simply calling this
    /// function twice. Extreme caution should be used when using this method,
    /// and in general you probably want to result to unsafe accessors and the
    /// `data` methods below.
    pub unsafe fn data_unchecked_mut(&self) -> &mut [u8] {
        let definition = &*self.wasmtime_memory_definition();
        slice::from_raw_parts_mut(definition.base, definition.current_length)
    }

    /// Returns the base pointer, in the host's address space, that the memory
    /// is located at.
    ///
    /// When reading and manipulating memory be sure to read up on the caveats
    /// of [`Memory::data_unchecked`] to make sure that you can safely
    /// read/write the memory.
    pub fn data_ptr(&self) -> *mut u8 {
        unsafe { (*self.wasmtime_memory_definition()).base }
    }

    /// Returns the byte length of this memory.
    ///
    /// The returned value will be a multiple of the wasm page size, 64k.
    pub fn data_size(&self) -> usize {
        unsafe { (*self.wasmtime_memory_definition()).current_length }
    }

    /// Returns the size, in pages, of this wasm memory.
    pub fn size(&self) -> u32 {
        (self.data_size() / wasmtime_environ::WASM_PAGE_SIZE as usize) as u32
    }

    /// Grows this WebAssembly memory by `delta` pages.
    ///
    /// This will attempt to add `delta` more pages of memory on to the end of
    /// this `Memory` instance. If successful this may relocate the memory and
    /// cause [`Memory::data_ptr`] to return a new value. Additionally previous
    /// slices into this memory may no longer be valid.
    ///
    /// On success returns the number of pages this memory previously had
    /// before the growth succeeded.
    ///
    /// # Errors
    ///
    /// Returns an error if memory could not be grown, for example if it exceeds
    /// the maximum limits of this memory.
    pub fn grow(&self, delta: u32) -> Result<u32> {
        match self.wasmtime_export {
            wasmtime_runtime::Export::Memory { definition, .. } => {
                let definition = unsafe { &(*definition) };
                let index = self.wasmtime_handle.memory_index(definition);
                self.wasmtime_handle
                    .clone()
                    .memory_grow(index, delta)
                    .ok_or_else(|| anyhow!("failed to grow memory"))
            }
            _ => panic!("memory definition not found"),
        }
    }

    pub(crate) fn wasmtime_export(&self) -> &wasmtime_runtime::Export {
        &self.wasmtime_export
    }

    pub(crate) fn from_wasmtime_memory(
        export: wasmtime_runtime::Export,
        store: &Store,
        instance_handle: wasmtime_runtime::InstanceHandle,
    ) -> Memory {
        let memory = if let wasmtime_runtime::Export::Memory { ref memory, .. } = export {
            memory
        } else {
            panic!("wasmtime export is not memory")
        };
        let ty = MemoryType::from_wasmtime_memory(&memory.memory);
        Memory {
            _store: store.clone(),
            ty: ty,
            wasmtime_handle: instance_handle,
            wasmtime_export: export,
        }
    }
}
