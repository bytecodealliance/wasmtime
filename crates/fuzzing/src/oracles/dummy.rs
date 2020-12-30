//! Dummy implementations of things that a Wasm module can import.
use wasmtime::*;
mod wat_generator;
use wat_generator::WatGenerator;
mod wencoder_generator;
use wencoder_generator::WencoderGenerator;

/// Create a set of dummy functions/globals/etc for the given imports.
pub fn dummy_imports<'module>(
    store: &Store,
    import_tys: impl Iterator<Item = ImportType<'module>>,
) -> Vec<Extern> {
    import_tys
        .map(|imp| match imp.ty() {
            ExternType::Func(func_ty) => Extern::Func(dummy_func(&store, func_ty)),
            ExternType::Global(global_ty) => Extern::Global(dummy_global(&store, global_ty)),
            ExternType::Table(table_ty) => Extern::Table(dummy_table(&store, table_ty)),
            ExternType::Memory(mem_ty) => Extern::Memory(dummy_memory(&store, mem_ty)),
            ExternType::Instance(instance_ty) => {
                Extern::Instance(dummy_instance(&store, instance_ty))
            }
            ExternType::Module(module_ty) => Extern::Module(dummy_module(&store, module_ty)),
        })
        .collect()
}
/// Construct a dummy function for the given function type
pub fn dummy_func(store: &Store, ty: FuncType) -> Func {
    Func::new(store, ty.clone(), move |_, _, results| {
        for (ret_ty, result) in ty.results().zip(results) {
            *result = dummy_value(ret_ty);
        }
        Ok(())
    })
}

/// Construct a dummy value for the given value type.
pub fn dummy_value(val_ty: ValType) -> Val {
    match val_ty {
        ValType::I32 => Val::I32(0),
        ValType::I64 => Val::I64(0),
        ValType::F32 => Val::F32(0),
        ValType::F64 => Val::F64(0),
        ValType::V128 => Val::V128(0),
        ValType::ExternRef => Val::ExternRef(None),
        ValType::FuncRef => Val::FuncRef(None),
    }
}

/// Construct a sequence of dummy values for the given types.
pub fn dummy_values(val_tys: impl IntoIterator<Item = ValType>) -> Vec<Val> {
    val_tys.into_iter().map(dummy_value).collect()
}

/// Construct a dummy global for the given global type.
pub fn dummy_global(store: &Store, ty: GlobalType) -> Global {
    let val = dummy_value(ty.content().clone());
    Global::new(store, ty, val).unwrap()
}

/// Construct a dummy table for the given table type.
pub fn dummy_table(store: &Store, ty: TableType) -> Table {
    let init_val = dummy_value(ty.element().clone());
    Table::new(store, ty, init_val).unwrap()
}

/// Construct a dummy memory for the given memory type.
pub fn dummy_memory(store: &Store, ty: MemoryType) -> Memory {
    Memory::new(store, ty)
}

/// Construct a dummy instance for the given instance type.
///
/// This is done by using the expected type to generate a module on-the-fly
/// which we the instantiate.
pub fn dummy_instance(store: &Store, ty: InstanceType) -> Instance {
    dummy_instance_wenc(store, ty)
}
/// Construct a dummy module for the given module type.
///
/// This is done by using the expected type to generate a module on-the-fly.
pub fn dummy_module(store: &Store, ty: ModuleType) -> Module {
    dummy_module_wenc(store, ty)
}
/// Wat Generator
/// Construct instance by wat
pub fn dummy_instance_wat(store: &Store, ty: InstanceType) -> Instance {
    let mut wat = WatGenerator::new();
    for ty in ty.exports() {
        wat.export(&ty);
    }
    let module = Module::new(store.engine(), &wat.finish()).unwrap();
    Instance::new(store, &module, &[]).unwrap()
}
/// Construct module by wat
pub fn dummy_module_wat(store: &Store, ty: ModuleType) -> Module {
    let mut wat = WatGenerator::new();
    for ty in ty.imports() {
        wat.import(&ty);
    }
    for ty in ty.exports() {
        wat.export(&ty);
    }
    Module::new(store.engine(), &wat.finish()).unwrap()
}
/// wencoder Generator
/// Construct instance by wencoder
pub fn dummy_instance_wenc(store: &Store, ty: InstanceType) -> Instance {
    let mut wgen = WencoderGenerator::new();
    for ty in ty.exports() {
        wgen.export(&ty);
    }
    let module = Module::new(store.engine(), &wgen.finish()).unwrap();
    Instance::new(store, &module, &[]).unwrap()
}
/// Construct module by wencoder
pub fn dummy_module_wenc(store: &Store, ty: ModuleType) -> Module {
    let mut wgen = WencoderGenerator::new();
    for ty in ty.imports() {
        wgen.import(&ty);
    }
    for ty in ty.exports() {
        wgen.export(&ty)
    }
    Module::new(store.engine(), &wgen.finish()).unwrap()
}
