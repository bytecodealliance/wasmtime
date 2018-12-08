use cranelift_codegen::ir::types;
use cranelift_codegen::{ir, isa};
use cranelift_entity::PrimaryMap;
use cranelift_wasm::{Global, GlobalInit, Memory, Table, TableElementType};
use std::ptr;
use target_lexicon::HOST;
use wasmtime_environ::{
    translate_signature, MemoryPlan, MemoryStyle, Module, TablePlan, TableStyle,
};
use wasmtime_execute::{Export, Resolver};
use wasmtime_runtime::{
    Imports, Instance, VMFunctionBody, VMGlobalDefinition, VMMemoryDefinition, VMTableDefinition,
};

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
    instance: Instance,
    spectest_global_i32: VMGlobalDefinition,
    spectest_global_f32: VMGlobalDefinition,
    spectest_global_f64: VMGlobalDefinition,
    spectest_table: VMTableDefinition,
    spectest_memory: VMMemoryDefinition,
}

impl SpecTest {
    pub fn new() -> Result<Self, String> {
        let finished_functions = PrimaryMap::new();
        let imports = Imports::none();
        let data_initializers = Vec::new();
        Ok(Self {
            instance: Instance::new(
                &Module::new(),
                &finished_functions.into_boxed_slice(),
                imports,
                &data_initializers,
            )?,
            spectest_global_i32: VMGlobalDefinition::new(&Global {
                ty: types::I32,
                mutability: true,
                initializer: GlobalInit::I32Const(0),
            }),
            spectest_global_f32: VMGlobalDefinition::new(&Global {
                ty: types::I32,
                mutability: true,
                initializer: GlobalInit::F32Const(0),
            }),
            spectest_global_f64: VMGlobalDefinition::new(&Global {
                ty: types::I32,
                mutability: true,
                initializer: GlobalInit::F64Const(0),
            }),
            spectest_table: VMTableDefinition {
                base: ptr::null_mut(),
                current_elements: 0,
            },
            spectest_memory: VMMemoryDefinition {
                base: ptr::null_mut(),
                current_length: 0,
            },
        })
    }
}

impl Resolver for SpecTest {
    fn resolve(&mut self, module: &str, field: &str) -> Option<Export> {
        let call_conv = isa::CallConv::triple_default(&HOST);
        let pointer_type = types::Type::triple_pointer_type(&HOST);
        match module {
            "spectest" => match field {
                "print" => Some(Export::function(
                    spectest_print as *const VMFunctionBody,
                    translate_signature(
                        ir::Signature {
                            params: vec![],
                            returns: vec![],
                            call_conv,
                        },
                        pointer_type,
                    ),
                )),
                "print_i32" => Some(Export::function(
                    spectest_print_i32 as *const VMFunctionBody,
                    translate_signature(
                        ir::Signature {
                            params: vec![ir::AbiParam::new(types::I32)],
                            returns: vec![],
                            call_conv,
                        },
                        pointer_type,
                    ),
                )),
                "print_i64" => Some(Export::function(
                    spectest_print_i64 as *const VMFunctionBody,
                    translate_signature(
                        ir::Signature {
                            params: vec![ir::AbiParam::new(types::I64)],
                            returns: vec![],
                            call_conv,
                        },
                        pointer_type,
                    ),
                )),
                "print_f32" => Some(Export::function(
                    spectest_print_f32 as *const VMFunctionBody,
                    translate_signature(
                        ir::Signature {
                            params: vec![ir::AbiParam::new(types::F32)],
                            returns: vec![],
                            call_conv,
                        },
                        pointer_type,
                    ),
                )),
                "print_f64" => Some(Export::function(
                    spectest_print_f64 as *const VMFunctionBody,
                    translate_signature(
                        ir::Signature {
                            params: vec![ir::AbiParam::new(types::F64)],
                            returns: vec![],
                            call_conv,
                        },
                        pointer_type,
                    ),
                )),
                "print_i32_f32" => Some(Export::function(
                    spectest_print_i32_f32 as *const VMFunctionBody,
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
                "print_f64_f64" => Some(Export::function(
                    spectest_print_f64_f64 as *const VMFunctionBody,
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
                "global_i32" => Some(Export::global(
                    &mut self.spectest_global_i32,
                    Global {
                        ty: ir::types::I32,
                        mutability: false,
                        initializer: GlobalInit::I32Const(0),
                    },
                )),
                "global_f32" => Some(Export::global(
                    &mut self.spectest_global_f32,
                    Global {
                        ty: ir::types::F32,
                        mutability: false,
                        initializer: GlobalInit::F32Const(0),
                    },
                )),
                "global_f64" => Some(Export::global(
                    &mut self.spectest_global_f64,
                    Global {
                        ty: ir::types::F64,
                        mutability: false,
                        initializer: GlobalInit::F64Const(0),
                    },
                )),
                "table" => Some(Export::table(
                    &mut self.spectest_table,
                    self.instance.vmctx_mut(),
                    TablePlan {
                        table: Table {
                            ty: TableElementType::Func,
                            minimum: 0,
                            maximum: None,
                        },
                        style: TableStyle::CallerChecksSignature,
                    },
                )),
                "memory" => Some(Export::memory(
                    &mut self.spectest_memory,
                    self.instance.vmctx_mut(),
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
