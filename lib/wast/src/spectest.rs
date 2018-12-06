use cranelift_codegen::ir::types;
use cranelift_codegen::{ir, isa};
use cranelift_wasm::{Global, GlobalInit, Memory, Table, TableElementType};
use std::ptr;
use target_lexicon::HOST;
use wasmtime_environ::{translate_signature, MemoryPlan, MemoryStyle, TablePlan, TableStyle};
use wasmtime_execute::{ExportValue, Resolver, VMGlobal, VMMemory, VMTable};

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

pub struct SpecTest {
    spectest_global_i32: VMGlobal,
    spectest_global_f32: VMGlobal,
    spectest_global_f64: VMGlobal,
    spectest_table: VMTable,
    spectest_memory: VMMemory,
}

impl SpecTest {
    pub fn new() -> Self {
        Self {
            spectest_global_i32: VMGlobal::definition(&Global {
                ty: types::I32,
                mutability: false,
                initializer: GlobalInit::I32Const(0),
            }),
            spectest_global_f32: VMGlobal::definition(&Global {
                ty: types::I32,
                mutability: false,
                initializer: GlobalInit::F32Const(0),
            }),
            spectest_global_f64: VMGlobal::definition(&Global {
                ty: types::I32,
                mutability: false,
                initializer: GlobalInit::F64Const(0),
            }),
            spectest_table: VMTable::definition(ptr::null_mut(), 0),
            spectest_memory: VMMemory::definition(ptr::null_mut(), 0),
        }
    }
}

impl Resolver for SpecTest {
    fn resolve(&mut self, module: &str, field: &str) -> Option<ExportValue> {
        let call_conv = isa::CallConv::triple_default(&HOST);
        let pointer_type = types::Type::triple_pointer_type(&HOST);
        match module {
            "spectest" => match field {
                "print" => Some(ExportValue::function(
                    spectest_print as usize,
                    translate_signature(
                        ir::Signature {
                            params: vec![],
                            returns: vec![],
                            call_conv,
                        },
                        pointer_type,
                    ),
                )),
                "print_i32" => Some(ExportValue::function(
                    spectest_print_i32 as usize,
                    translate_signature(
                        ir::Signature {
                            params: vec![ir::AbiParam::new(types::I32)],
                            returns: vec![],
                            call_conv,
                        },
                        pointer_type,
                    ),
                )),
                "print_i64" => Some(ExportValue::function(
                    spectest_print_i64 as usize,
                    translate_signature(
                        ir::Signature {
                            params: vec![ir::AbiParam::new(types::I64)],
                            returns: vec![],
                            call_conv,
                        },
                        pointer_type,
                    ),
                )),
                "print_f32" => Some(ExportValue::function(
                    spectest_print_f32 as usize,
                    translate_signature(
                        ir::Signature {
                            params: vec![ir::AbiParam::new(types::F32)],
                            returns: vec![],
                            call_conv,
                        },
                        pointer_type,
                    ),
                )),
                "print_f64" => Some(ExportValue::function(
                    spectest_print_f64 as usize,
                    translate_signature(
                        ir::Signature {
                            params: vec![ir::AbiParam::new(types::F64)],
                            returns: vec![],
                            call_conv,
                        },
                        pointer_type,
                    ),
                )),
                "print_i32_f32" => Some(ExportValue::function(
                    spectest_print_i32_f32 as usize,
                    translate_signature(
                        ir::Signature {
                            params: vec![
                                ir::AbiParam::new(types::I32),
                                ir::AbiParam::new(types::F32),
                            ],
                            returns: vec![],
                            call_conv,
                        },
                        pointer_type,
                    ),
                )),
                "print_f64_f64" => Some(ExportValue::function(
                    spectest_print_f64_f64 as usize,
                    translate_signature(
                        ir::Signature {
                            params: vec![
                                ir::AbiParam::new(types::F64),
                                ir::AbiParam::new(types::F64),
                            ],
                            returns: vec![],
                            call_conv,
                        },
                        pointer_type,
                    ),
                )),
                "global_i32" => Some(ExportValue::global(
                    &mut self.spectest_global_i32,
                    Global {
                        ty: ir::types::I32,
                        mutability: false,
                        initializer: GlobalInit::I32Const(0),
                    },
                )),
                "global_f32" => Some(ExportValue::global(
                    &mut self.spectest_global_f32,
                    Global {
                        ty: ir::types::F32,
                        mutability: false,
                        initializer: GlobalInit::F32Const(0),
                    },
                )),
                "global_f64" => Some(ExportValue::global(
                    &mut self.spectest_global_f64,
                    Global {
                        ty: ir::types::F64,
                        mutability: false,
                        initializer: GlobalInit::F64Const(0),
                    },
                )),
                "table" => Some(ExportValue::table(
                    &mut self.spectest_table,
                    TablePlan {
                        table: Table {
                            ty: TableElementType::Func,
                            minimum: 0,
                            maximum: None,
                        },
                        style: TableStyle::CallerChecksSignature,
                    },
                )),
                "memory" => Some(ExportValue::memory(
                    &mut self.spectest_memory,
                    MemoryPlan {
                        memory: Memory {
                            minimum: 0,
                            maximum: None,
                            shared: false,
                        },
                        style: MemoryStyle::Dynamic,
                        offset_guard_size: 0,
                    },
                )),
                _ => None,
            },
            _ => None,
        }
    }
}
