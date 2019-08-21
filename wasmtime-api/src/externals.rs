use crate::callable::{Callable, WasmtimeFn};
use crate::runtime::Store;
use crate::trap::Trap;
use crate::types::{ExternType, FuncType, GlobalType, MemoryType};
use crate::values::Val;
use std::cell::RefCell;
use std::rc::Rc;
use std::result::Result;

use crate::trampoline::generate_func_export;
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
            _ => unimplemented!("ExternType::type"),
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
            _ => unimplemented!("get_wasmtime_export"),
        }
    }

    pub(crate) fn from_wasmtime_export(
        store: Rc<RefCell<Store>>,
        instance_handle: InstanceHandle,
        export: wasmtime_runtime::Export,
    ) -> Extern {
        use cranelift_wasm::GlobalInit;
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
            wasmtime_runtime::Export::Memory {
                definition: _,
                vmctx: _,
                memory,
            } => {
                let ty = MemoryType::from_cranelift_memory(memory.memory.clone());
                let m = Memory::new(store, ty);
                Extern::Memory(Rc::new(RefCell::new(m)))
            }
            wasmtime_runtime::Export::Global {
                definition: _,
                vmctx: _,
                global,
            } => {
                let ty = GlobalType::from_cranelift_global(global.clone());
                let val = match global.initializer {
                    GlobalInit::I32Const(i) => Val::from(i),
                    GlobalInit::I64Const(i) => Val::from(i),
                    GlobalInit::F32Const(f) => Val::from_f32_bits(f),
                    GlobalInit::F64Const(f) => Val::from_f64_bits(f),
                    _ => unimplemented!("from_wasmtime_export initializer"),
                };
                Extern::Global(Rc::new(RefCell::new(Global::new(store, ty, val))))
            }
            wasmtime_runtime::Export::Table { .. } => {
                // TODO Extern::Table(Rc::new(RefCell::new(Table::new(store, ty, val))))
                Extern::Table(Rc::new(RefCell::new(Table)))
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
    val: Val,
}

impl Global {
    pub fn new(store: Rc<RefCell<Store>>, r#type: GlobalType, val: Val) -> Global {
        Global {
            _store: store,
            r#type,
            val,
        }
    }

    pub fn r#type(&self) -> &GlobalType {
        &self.r#type
    }

    pub fn get(&self) -> &Val {
        &self.val
    }

    pub fn set(&mut self, val: Val) {
        self.val = val;
    }
}

pub struct Table;

pub struct Memory {
    _store: Rc<RefCell<Store>>,
    r#type: MemoryType,
}

impl Memory {
    pub fn new(store: Rc<RefCell<Store>>, r#type: MemoryType) -> Memory {
        Memory {
            _store: store,
            r#type,
        }
    }

    pub fn r#type(&self) -> &MemoryType {
        &self.r#type
    }

    pub fn data(&self) -> *const u8 {
        unimplemented!("Memory::data")
    }

    pub fn data_size(&self) -> usize {
        unimplemented!("Memory::data_size")
    }

    pub fn size(&self) -> u32 {
        unimplemented!("Memory::size")
    }

    pub fn grow(&mut self, _delta: u32) -> bool {
        unimplemented!("Memory::grow")
    }
}
