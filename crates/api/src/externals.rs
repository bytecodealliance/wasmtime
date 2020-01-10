use crate::callable::{Callable, NativeCallable, WasmtimeFn, WrappedCallable};
use crate::r#ref::AnyRef;
use crate::runtime::Store;
use crate::trampoline::{generate_global_export, generate_memory_export, generate_table_export};
use crate::trap::Trap;
use crate::types::{ExternType, FuncType, GlobalType, MemoryType, TableType, ValType};
use crate::values::{from_checked_anyfunc, into_checked_anyfunc, Val};
use std::fmt;
use std::rc::Rc;
use std::slice;
use wasmtime_environ::wasm;
use wasmtime_runtime::InstanceHandle;

// Externals

#[derive(Clone)]
pub enum Extern {
    Func(Func),
    Global(Global),
    Table(Table),
    Memory(Memory),
}

impl Extern {
    pub fn func(&self) -> Option<&Func> {
        match self {
            Extern::Func(func) => Some(func),
            _ => None,
        }
    }
    pub fn global(&self) -> Option<&Global> {
        match self {
            Extern::Global(global) => Some(global),
            _ => None,
        }
    }
    pub fn table(&self) -> Option<&Table> {
        match self {
            Extern::Table(table) => Some(table),
            _ => None,
        }
    }
    pub fn memory(&self) -> Option<&Memory> {
        match self {
            Extern::Memory(memory) => Some(memory),
            _ => None,
        }
    }

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

#[derive(Clone)]
pub struct Func {
    _store: Store,
    callable: Rc<dyn WrappedCallable + 'static>,
    ty: FuncType,
}

impl Func {
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

    pub fn ty(&self) -> &FuncType {
        &self.ty
    }

    pub fn param_arity(&self) -> usize {
        self.ty.params().len()
    }

    pub fn result_arity(&self) -> usize {
        self.ty.results().len()
    }

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
        let ty = if let wasmtime_runtime::Export::Function { signature, .. } = &export {
            FuncType::from_wasmtime_signature(signature.clone())
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
    pub fn new(store: &Store, ty: GlobalType, val: Val) -> Global {
        let (wasmtime_export, wasmtime_state) =
            generate_global_export(&ty, val).expect("generated global");
        Global {
            inner: Rc::new(GlobalInner {
                _store: store.clone(),
                ty,
                wasmtime_export,
                wasmtime_state: Some(wasmtime_state),
            }),
        }
    }

    pub fn ty(&self) -> &GlobalType {
        &self.inner.ty
    }

    fn wasmtime_global_definition(&self) -> *mut wasmtime_runtime::VMGlobalDefinition {
        match self.inner.wasmtime_export {
            wasmtime_runtime::Export::Global { definition, .. } => definition,
            _ => panic!("global definition not found"),
        }
    }

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

    pub fn set(&self, val: Val) {
        if val.ty() != *self.ty().content() {
            panic!(
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
    }

    pub(crate) fn wasmtime_export(&self) -> &wasmtime_runtime::Export {
        &self.inner.wasmtime_export
    }

    pub(crate) fn from_wasmtime_global(export: wasmtime_runtime::Export, store: &Store) -> Global {
        let global = if let wasmtime_runtime::Export::Global { ref global, .. } = export {
            global
        } else {
            panic!("wasmtime export is not memory")
        };
        let ty = GlobalType::from_wasmtime_global(&global);
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

#[derive(Clone)]
pub struct Table {
    store: Store,
    ty: TableType,
    wasmtime_handle: InstanceHandle,
    wasmtime_export: wasmtime_runtime::Export,
}

fn get_table_item(
    handle: &InstanceHandle,
    store: &Store,
    table_index: wasm::DefinedTableIndex,
    item_index: u32,
) -> Val {
    if let Some(item) = handle.table_get(table_index, item_index) {
        from_checked_anyfunc(item, store)
    } else {
        AnyRef::null().into()
    }
}

fn set_table_item(
    handle: &mut InstanceHandle,
    store: &Store,
    table_index: wasm::DefinedTableIndex,
    item_index: u32,
    val: Val,
) -> bool {
    let item = into_checked_anyfunc(val, store);
    if let Some(item_ref) = handle.table_get_mut(table_index, item_index) {
        *item_ref = item;
        true
    } else {
        false
    }
}

impl Table {
    pub fn new(store: &Store, ty: TableType, init: Val) -> Table {
        match ty.element() {
            ValType::FuncRef => (),
            _ => panic!("table is not for funcref"),
        }
        let (mut wasmtime_handle, wasmtime_export) =
            generate_table_export(&ty).expect("generated table");

        // Initialize entries with the init value.
        match wasmtime_export {
            wasmtime_runtime::Export::Table { definition, .. } => {
                let index = wasmtime_handle.table_index(unsafe { &*definition });
                let len = unsafe { (*definition).current_elements };
                for i in 0..len {
                    let _success =
                        set_table_item(&mut wasmtime_handle, store, index, i, init.clone());
                    assert!(_success);
                }
            }
            _ => panic!("global definition not found"),
        }

        Table {
            store: store.clone(),
            ty,
            wasmtime_handle,
            wasmtime_export,
        }
    }

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

    pub fn get(&self, index: u32) -> Val {
        let table_index = self.wasmtime_table_index();
        get_table_item(&self.wasmtime_handle, &self.store, table_index, index)
    }

    pub fn set(&self, index: u32, val: Val) -> bool {
        let table_index = self.wasmtime_table_index();
        let mut wasmtime_handle = self.wasmtime_handle.clone();
        set_table_item(&mut wasmtime_handle, &self.store, table_index, index, val)
    }

    pub fn size(&self) -> u32 {
        match self.wasmtime_export {
            wasmtime_runtime::Export::Table { definition, .. } => unsafe {
                (*definition).current_elements
            },
            _ => panic!("global definition not found"),
        }
    }

    pub fn grow(&self, delta: u32, init: Val) -> bool {
        let index = self.wasmtime_table_index();
        if let Some(len) = self.wasmtime_handle.clone().table_grow(index, delta) {
            let mut wasmtime_handle = self.wasmtime_handle.clone();
            for i in 0..delta {
                let i = len - (delta - i);
                let _success =
                    set_table_item(&mut wasmtime_handle, &self.store, index, i, init.clone());
                assert!(_success);
            }
            true
        } else {
            false
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

#[derive(Clone)]
pub struct Memory {
    _store: Store,
    ty: MemoryType,
    wasmtime_handle: InstanceHandle,
    wasmtime_export: wasmtime_runtime::Export,
}

impl Memory {
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

    pub fn ty(&self) -> &MemoryType {
        &self.ty
    }

    fn wasmtime_memory_definition(&self) -> *mut wasmtime_runtime::VMMemoryDefinition {
        match self.wasmtime_export {
            wasmtime_runtime::Export::Memory { definition, .. } => definition,
            _ => panic!("memory definition not found"),
        }
    }

    /// Returns a mutable slice the current memory.
    /// # Safety
    /// Marked unsafe due to posibility that wasmtime can resize internal memory
    /// from other threads.
    pub unsafe fn data(&self) -> &mut [u8] {
        let definition = &*self.wasmtime_memory_definition();
        slice::from_raw_parts_mut(definition.base, definition.current_length)
    }

    pub fn data_ptr(&self) -> *mut u8 {
        unsafe { (*self.wasmtime_memory_definition()).base }
    }

    pub fn data_size(&self) -> usize {
        unsafe { (*self.wasmtime_memory_definition()).current_length }
    }

    pub fn size(&self) -> u32 {
        (self.data_size() / wasmtime_environ::WASM_PAGE_SIZE as usize) as u32
    }

    pub fn grow(&self, delta: u32) -> bool {
        match self.wasmtime_export {
            wasmtime_runtime::Export::Memory { definition, .. } => {
                let definition = unsafe { &(*definition) };
                let index = self.wasmtime_handle.memory_index(definition);
                self.wasmtime_handle
                    .clone()
                    .memory_grow(index, delta)
                    .is_some()
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
