use crate::callable::{Callable, WasmtimeFn};
use crate::runtime::Store;
use crate::trap::Trap;
use crate::types::{ExternType, FuncType, GlobalType, MemoryType, TableType, ValType};
use crate::values::Val;
use std::cell::RefCell;
use std::rc::Rc;
use std::result::Result;

use crate::trampoline::{generate_func_export, generate_global_export, generate_memory_export};
use wasmtime_runtime::InstanceHandle;
// Externals

pub enum Extern {
    Func(Rc<RefCell<Func>>),
    Global(Rc<RefCell<Global>>),
    Table(Rc<RefCell<Table>>),
    Memory(Rc<RefCell<Memory>>),
}

impl Extern {
    pub fn func(&self) -> &Rc<RefCell<Func>> {
        match self {
            Extern::Func(func) => func,
            _ => panic!("Extern::Func expected"),
        }
    }
    pub fn global(&self) -> &Rc<RefCell<Global>> {
        match self {
            Extern::Global(global) => global,
            _ => panic!("Extern::Global expected"),
        }
    }
    pub fn table(&self) -> &Rc<RefCell<Table>> {
        match self {
            Extern::Table(table) => table,
            _ => panic!("Extern::Table expected"),
        }
    }
    pub fn memory(&self) -> &Rc<RefCell<Memory>> {
        match self {
            Extern::Memory(memory) => memory,
            _ => panic!("Extern::Memory expected"),
        }
    }

    pub fn r#type(&self) -> ExternType {
        match self {
            Extern::Func(ft) => ExternType::ExternFunc(ft.borrow().r#type().clone()),
            Extern::Memory(ft) => ExternType::ExternMemory(ft.borrow().r#type().clone()),
            Extern::Table(tt) => ExternType::ExternTable(tt.borrow().r#type().clone()),
            Extern::Global(gt) => ExternType::ExternGlobal(gt.borrow().r#type().clone()),
        }
    }

    pub(crate) fn get_wasmtime_export(&mut self) -> wasmtime_runtime::Export {
        match self {
            Extern::Func(f) => {
                if f.borrow().anchor.is_none() {
                    generate_func_export(&f).expect("generate_func_export");
                }
                f.borrow().anchor.as_ref().unwrap().1.clone()
            }
            Extern::Global(g) => g.borrow().wasmtime_export().clone(),
            Extern::Memory(m) => m.borrow().wasmtime_export().clone(),
            _ => unimplemented!("get_wasmtime_export"),
        }
    }

    pub(crate) fn from_wasmtime_export(
        store: Rc<RefCell<Store>>,
        instance_handle: InstanceHandle,
        export: wasmtime_runtime::Export,
    ) -> Extern {
        match export {
            wasmtime_runtime::Export::Function {
                address,
                vmctx,
                ref signature,
            } => {
                let ty = FuncType::from_cranelift_signature(signature.clone());
                let callable = WasmtimeFn::new(store.clone(), signature.clone(), address, vmctx);
                let mut f = Func::new(store, ty, Rc::new(callable));
                f.anchor = Some((instance_handle, export.clone()));
                Extern::Func(Rc::new(RefCell::new(f)))
            }
            wasmtime_runtime::Export::Memory { .. } => Extern::Memory(Rc::new(RefCell::new(
                Memory::from_wasmtime_memory(export, store, instance_handle),
            ))),
            wasmtime_runtime::Export::Global { .. } => Extern::Global(Rc::new(RefCell::new(
                Global::from_wasmtime_global(export, store),
            ))),
            wasmtime_runtime::Export::Table {
                definition: _,
                vmctx: _,
                table,
            } => {
                let ty = TableType::from_cranelift_table(table.table.clone());
                Extern::Table(Rc::new(RefCell::new(Table::new(store, ty))))
            }
        }
    }
}

pub struct Func {
    _store: Rc<RefCell<Store>>,
    callable: Rc<dyn Callable + 'static>,
    r#type: FuncType,
    pub(crate) anchor: Option<(InstanceHandle, wasmtime_runtime::Export)>,
}

impl Func {
    pub fn new(
        store: Rc<RefCell<Store>>,
        r#type: FuncType,
        callable: Rc<dyn Callable + 'static>,
    ) -> Func {
        Func {
            _store: store,
            callable,
            r#type,
            anchor: None,
        }
    }

    pub fn r#type(&self) -> &FuncType {
        &self.r#type
    }

    pub fn param_arity(&self) -> usize {
        self.r#type.params().len()
    }

    pub fn result_arity(&self) -> usize {
        self.r#type.results().len()
    }

    pub fn callable(&self) -> &(dyn Callable + 'static) {
        self.callable.as_ref()
    }

    pub fn call(&self, params: &[Val]) -> Result<Box<[Val]>, Rc<RefCell<Trap>>> {
        let mut results = vec![Val::default(); self.result_arity()];
        self.callable.call(params, &mut results)?;
        Ok(results.into_boxed_slice())
    }
}

pub struct Global {
    _store: Rc<RefCell<Store>>,
    r#type: GlobalType,
    wasmtime_export: wasmtime_runtime::Export,
    #[allow(dead_code)]
    wasmtime_state: Option<crate::trampoline::GlobalState>,
}

impl Global {
    pub fn new(store: Rc<RefCell<Store>>, r#type: GlobalType, val: Val) -> Global {
        let (wasmtime_export, wasmtime_state) =
            generate_global_export(&r#type, val).expect("generated global");
        Global {
            _store: store,
            r#type,
            wasmtime_export,
            wasmtime_state: Some(wasmtime_state),
        }
    }

    pub fn r#type(&self) -> &GlobalType {
        &self.r#type
    }

    fn wasmtime_global_definition(&self) -> *mut wasmtime_runtime::VMGlobalDefinition {
        match self.wasmtime_export {
            wasmtime_runtime::Export::Global { definition, .. } => definition,
            _ => panic!("global definition not found"),
        }
    }

    pub fn get(&self) -> Val {
        let definition = unsafe { &mut *self.wasmtime_global_definition() };
        unsafe {
            match self.r#type().content() {
                ValType::I32 => Val::from(*definition.as_i32()),
                ValType::I64 => Val::from(*definition.as_i64()),
                ValType::F32 => Val::from_f32_bits(*definition.as_u32()),
                ValType::F64 => Val::from_f64_bits(*definition.as_u64()),
                _ => unimplemented!("Global::get for {:?}", self.r#type().content()),
            }
        }
    }

    pub fn set(&mut self, val: Val) {
        if val.r#type() != *self.r#type().content() {
            panic!(
                "global of type {:?} cannot be set to {:?}",
                self.r#type().content(),
                val.r#type()
            );
        }
        let definition = unsafe { &mut *self.wasmtime_global_definition() };
        unsafe {
            match val {
                Val::I32(i) => *definition.as_i32_mut() = i,
                Val::I64(i) => *definition.as_i64_mut() = i,
                Val::F32(f) => *definition.as_u32_mut() = f,
                Val::F64(f) => *definition.as_u64_mut() = f,
                _ => unimplemented!("Global::set for {:?}", val.r#type()),
            }
        }
    }

    pub(crate) fn wasmtime_export(&self) -> &wasmtime_runtime::Export {
        &self.wasmtime_export
    }

    pub(crate) fn from_wasmtime_global(
        export: wasmtime_runtime::Export,
        store: Rc<RefCell<Store>>,
    ) -> Global {
        let global = if let wasmtime_runtime::Export::Global { ref global, .. } = export {
            global
        } else {
            panic!("wasmtime export is not memory")
        };
        let ty = GlobalType::from_cranelift_global(global.clone());
        Global {
            _store: store,
            r#type: ty,
            wasmtime_export: export,
            wasmtime_state: None,
        }
    }
}

pub struct Table {
    _store: Rc<RefCell<Store>>,
    r#type: TableType,
}

impl Table {
    pub fn new(store: Rc<RefCell<Store>>, r#type: TableType) -> Table {
        Table {
            _store: store,
            r#type,
        }
    }

    pub fn r#type(&self) -> &TableType {
        &self.r#type
    }

    pub fn get(&self, _index: u32) -> Val {
        unimplemented!("Table::get")
    }

    pub fn set(&self, _index: u32, _val: &Val) -> usize {
        unimplemented!("Table::set")
    }

    pub fn size(&self) -> u32 {
        unimplemented!("Table::size")
    }

    pub fn grow(&mut self, _delta: u32) -> bool {
        unimplemented!("Table::grow")
    }
}

pub struct Memory {
    _store: Rc<RefCell<Store>>,
    r#type: MemoryType,
    wasmtime_handle: InstanceHandle,
    wasmtime_export: wasmtime_runtime::Export,
}

impl Memory {
    pub fn new(store: Rc<RefCell<Store>>, r#type: MemoryType) -> Memory {
        let (wasmtime_handle, wasmtime_export) =
            generate_memory_export(&r#type).expect("generated memory");
        Memory {
            _store: store,
            r#type,
            wasmtime_handle,
            wasmtime_export,
        }
    }

    pub fn r#type(&self) -> &MemoryType {
        &self.r#type
    }

    fn wasmtime_memory_definition(&self) -> *mut wasmtime_runtime::VMMemoryDefinition {
        match self.wasmtime_export {
            wasmtime_runtime::Export::Memory { definition, .. } => definition,
            _ => panic!("memory definition not found"),
        }
    }

    pub fn data(&self) -> *mut u8 {
        unsafe { (*self.wasmtime_memory_definition()).base }
    }

    pub fn data_size(&self) -> usize {
        unsafe { (*self.wasmtime_memory_definition()).current_length }
    }

    pub fn size(&self) -> u32 {
        (self.data_size() / wasmtime_environ::WASM_PAGE_SIZE as usize) as u32
    }

    pub fn grow(&mut self, delta: u32) -> bool {
        match self.wasmtime_export {
            wasmtime_runtime::Export::Memory { definition, .. } => {
                let definition = unsafe { &(*definition) };
                let index = self.wasmtime_handle.memory_index(definition);
                self.wasmtime_handle.memory_grow(index, delta).is_some()
            }
            _ => panic!("memory definition not found"),
        }
    }

    pub(crate) fn wasmtime_export(&self) -> &wasmtime_runtime::Export {
        &self.wasmtime_export
    }

    pub(crate) fn from_wasmtime_memory(
        export: wasmtime_runtime::Export,
        store: Rc<RefCell<Store>>,
        instance_handle: wasmtime_runtime::InstanceHandle,
    ) -> Memory {
        let memory = if let wasmtime_runtime::Export::Memory { ref memory, .. } = export {
            memory
        } else {
            panic!("wasmtime export is not memory")
        };
        let ty = MemoryType::from_cranelift_memory(memory.memory.clone());
        Memory {
            _store: store,
            r#type: ty,
            wasmtime_handle: instance_handle,
            wasmtime_export: export,
        }
    }
}
