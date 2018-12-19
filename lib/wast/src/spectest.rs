use cranelift_codegen::ir::types;
use cranelift_codegen::{ir, isa};
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{DefinedFuncIndex, Global, GlobalInit, Memory, Table, TableElementType};
use std::rc::Rc;
use target_lexicon::HOST;
use wasmtime_environ::{
    translate_signature, Export, MemoryPlan, MemoryStyle, Module, TablePlan, TableStyle,
};
use wasmtime_execute::{target_tunables, ActionError, InstancePlus};
use wasmtime_runtime::{Imports, VMFunctionBody};

extern "C" fn spectest_print() {}

#[allow(clippy::print_stdout)]
extern "C" fn spectest_print_i32(x: i32) {
    println!("{}: i32", x);
}

#[allow(clippy::print_stdout)]
extern "C" fn spectest_print_i64(x: i64) {
    println!("{}: i64", x);
}

#[allow(clippy::print_stdout)]
extern "C" fn spectest_print_f32(x: f32) {
    println!("{}: f32", x);
}

#[allow(clippy::print_stdout)]
extern "C" fn spectest_print_f64(x: f64) {
    println!("{}: f64", x);
}

#[allow(clippy::print_stdout)]
extern "C" fn spectest_print_i32_f32(x: i32, y: f32) {
    println!("{}: i32", x);
    println!("{}: f32", y);
}

#[allow(clippy::print_stdout)]
extern "C" fn spectest_print_f64_f64(x: f64, y: f64) {
    println!("{}: f64", x);
    println!("{}: f64", y);
}

/// Return an instance implementing the "spectest" interface used in the
/// spec testsuite.
pub fn instantiate_spectest() -> Result<InstancePlus, ActionError> {
    let call_conv = isa::CallConv::triple_default(&HOST);
    let pointer_type = types::Type::triple_pointer_type(&HOST);
    let mut module = Module::new();
    let mut finished_functions: PrimaryMap<DefinedFuncIndex, *const VMFunctionBody> =
        PrimaryMap::new();

    let sig = module.signatures.push(translate_signature(
        ir::Signature {
            params: vec![],
            returns: vec![],
            call_conv,
        },
        pointer_type,
    ));
    let func = module.functions.push(sig);
    module
        .exports
        .insert("print".to_owned(), Export::Function(func));
    finished_functions.push(spectest_print as *const VMFunctionBody);

    let sig = module.signatures.push(translate_signature(
        ir::Signature {
            params: vec![ir::AbiParam::new(types::I32)],
            returns: vec![],
            call_conv,
        },
        pointer_type,
    ));
    let func = module.functions.push(sig);
    module
        .exports
        .insert("print_i32".to_owned(), Export::Function(func));
    finished_functions.push(spectest_print_i32 as *const VMFunctionBody);

    let sig = module.signatures.push(translate_signature(
        ir::Signature {
            params: vec![ir::AbiParam::new(types::I64)],
            returns: vec![],
            call_conv,
        },
        pointer_type,
    ));
    let func = module.functions.push(sig);
    module
        .exports
        .insert("print_i64".to_owned(), Export::Function(func));
    finished_functions.push(spectest_print_i64 as *const VMFunctionBody);

    let sig = module.signatures.push(translate_signature(
        ir::Signature {
            params: vec![ir::AbiParam::new(types::F32)],
            returns: vec![],
            call_conv,
        },
        pointer_type,
    ));
    let func = module.functions.push(sig);
    module
        .exports
        .insert("print_f32".to_owned(), Export::Function(func));
    finished_functions.push(spectest_print_f32 as *const VMFunctionBody);

    let sig = module.signatures.push(translate_signature(
        ir::Signature {
            params: vec![ir::AbiParam::new(types::F64)],
            returns: vec![],
            call_conv,
        },
        pointer_type,
    ));
    let func = module.functions.push(sig);
    module
        .exports
        .insert("print_f64".to_owned(), Export::Function(func));
    finished_functions.push(spectest_print_f64 as *const VMFunctionBody);

    let sig = module.signatures.push(translate_signature(
        ir::Signature {
            params: vec![ir::AbiParam::new(types::I32), ir::AbiParam::new(types::F32)],
            returns: vec![],
            call_conv,
        },
        pointer_type,
    ));
    let func = module.functions.push(sig);
    module
        .exports
        .insert("print_i32_f32".to_owned(), Export::Function(func));
    finished_functions.push(spectest_print_i32_f32 as *const VMFunctionBody);

    let sig = module.signatures.push(translate_signature(
        ir::Signature {
            params: vec![ir::AbiParam::new(types::F64), ir::AbiParam::new(types::F64)],
            returns: vec![],
            call_conv,
        },
        pointer_type,
    ));
    let func = module.functions.push(sig);
    module
        .exports
        .insert("print_f64_f64".to_owned(), Export::Function(func));
    finished_functions.push(spectest_print_f64_f64 as *const VMFunctionBody);

    let global = module.globals.push(Global {
        ty: types::I32,
        mutability: false,
        initializer: GlobalInit::I32Const(666),
    });
    module
        .exports
        .insert("global_i32".to_owned(), Export::Global(global));

    let global = module.globals.push(Global {
        ty: types::I64,
        mutability: false,
        initializer: GlobalInit::I64Const(666),
    });
    module
        .exports
        .insert("global_i64".to_owned(), Export::Global(global));

    let global = module.globals.push(Global {
        ty: types::F32,
        mutability: false,
        initializer: GlobalInit::F32Const(0x44268000),
    });
    module
        .exports
        .insert("global_f32".to_owned(), Export::Global(global));

    let global = module.globals.push(Global {
        ty: types::F64,
        mutability: false,
        initializer: GlobalInit::F64Const(0x4084d00000000000),
    });
    module
        .exports
        .insert("global_f64".to_owned(), Export::Global(global));

    let table = module.table_plans.push(TablePlan {
        table: Table {
            ty: TableElementType::Func,
            minimum: 10,
            maximum: Some(20),
        },
        style: TableStyle::CallerChecksSignature,
    });
    module
        .exports
        .insert("table".to_owned(), Export::Table(table));

    let tunables = target_tunables(&HOST);
    let memory = module.memory_plans.push(MemoryPlan {
        memory: Memory {
            minimum: 1,
            maximum: Some(2),
            shared: false,
        },
        style: MemoryStyle::Static {
            bound: tunables.static_memory_bound,
        },
        offset_guard_size: tunables.static_memory_offset_guard_size,
    });
    module
        .exports
        .insert("memory".to_owned(), Export::Memory(memory));

    let imports = Imports::none();
    let data_initializers = Vec::new();

    InstancePlus::with_parts(
        Rc::new(module),
        finished_functions.into_boxed_slice(),
        imports,
        data_initializers,
    )
}
