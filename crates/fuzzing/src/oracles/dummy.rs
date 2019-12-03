//! Dummy implementations of things that a Wasm module can import.

use std::rc::Rc;
use wasmtime::{
    Callable, Extern, ExternType, Func, FuncType, Global, GlobalType, HostRef, ImportType, Memory,
    MemoryType, Store, Table, TableType, Trap, Val, ValType,
};

/// Create a set of dummy functions/globals/etc for the given imports.
pub fn dummy_imports(
    store: &HostRef<Store>,
    import_tys: &[ImportType],
) -> Result<Vec<Extern>, HostRef<Trap>> {
    let mut imports = Vec::with_capacity(import_tys.len());
    for imp in import_tys {
        imports.push(match imp.r#type() {
            ExternType::ExternFunc(func_ty) => {
                Extern::Func(HostRef::new(DummyFunc::new(&store, func_ty.clone())))
            }
            ExternType::ExternGlobal(global_ty) => {
                Extern::Global(HostRef::new(dummy_global(&store, global_ty.clone())?))
            }
            ExternType::ExternTable(table_ty) => {
                Extern::Table(HostRef::new(dummy_table(&store, table_ty.clone())?))
            }
            ExternType::ExternMemory(mem_ty) => {
                Extern::Memory(HostRef::new(dummy_memory(&store, mem_ty.clone())))
            }
        });
    }
    Ok(imports)
}

/// A function that doesn't do anything but return the default (zero) value for
/// the function's type.
#[derive(Debug)]
pub struct DummyFunc(FuncType);

impl DummyFunc {
    /// Construct a new dummy `Func`.
    pub fn new(store: &HostRef<Store>, ty: FuncType) -> Func {
        let callable = DummyFunc(ty.clone());
        Func::new(store, ty, Rc::new(callable) as _)
    }
}

impl Callable for DummyFunc {
    fn call(&self, _params: &[Val], results: &mut [Val]) -> Result<(), HostRef<Trap>> {
        for (ret_ty, result) in self.0.results().iter().zip(results) {
            *result = dummy_value(ret_ty)?;
        }

        Ok(())
    }
}

/// Construct a dummy value for the given value type.
pub fn dummy_value(val_ty: &ValType) -> Result<Val, HostRef<Trap>> {
    Ok(match val_ty {
        ValType::I32 => Val::I32(0),
        ValType::I64 => Val::I64(0),
        ValType::F32 => Val::F32(0),
        ValType::F64 => Val::F64(0),
        ValType::V128 => {
            return Err(HostRef::new(Trap::new(
                "dummy_value: unsupported function return type: v128".to_string(),
            )))
        }
        ValType::AnyRef => {
            return Err(HostRef::new(Trap::new(
                "dummy_value: unsupported function return type: anyref".to_string(),
            )))
        }
        ValType::FuncRef => {
            return Err(HostRef::new(Trap::new(
                "dummy_value: unsupported function return type: funcref".to_string(),
            )))
        }
    })
}

/// Construct a dummy global for the given global type.
pub fn dummy_global(store: &HostRef<Store>, ty: GlobalType) -> Result<Global, HostRef<Trap>> {
    let val = dummy_value(ty.content())?;
    Ok(Global::new(store, ty, val))
}

/// Construct a dummy table for the given table type.
pub fn dummy_table(store: &HostRef<Store>, ty: TableType) -> Result<Table, HostRef<Trap>> {
    let init_val = dummy_value(&ty.element())?;
    Ok(Table::new(store, ty, init_val))
}

/// Construct a dummy memory for the given memory type.
pub fn dummy_memory(store: &HostRef<Store>, ty: MemoryType) -> Memory {
    Memory::new(store, ty)
}
