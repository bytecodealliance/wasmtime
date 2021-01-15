use wasm_encoder::{
    CodeSection, Export, ExportSection, Function, FunctionSection, GlobalSection, ImportSection,
    InstanceSection, Instruction, Limits, MemorySection, Module, ModuleSection, TableSection, TypeSection,
};
use wasmtime::*;
pub struct WencoderGenerator {
    next: [u32; 6],
    module: Module,
}
impl WencoderGenerator {
    pub fn new() -> WencoderGenerator {
        WencoderGenerator {
            next: [0,0,0,0,0,0],
            module: Module::new(),
        }
    }
    fn next(&mut self, ty: &ExternType) -> u32 {
       let index =  match ty {
            ExternType::Memory(_) => 0,
           ExternType::Table(_) => 1,
           ExternType::Global(_) => 2,
            ExternType::Func(_) => 3,
           ExternType::Instance(_) => 4,
           ExternType::Module(_) => 5
       };
       self.next[index] +=1;
       self.next[index]
    }
    pub fn finish(self) -> Vec<u8> {
        self.module.finish()
    }
    pub fn import(&mut self, ty: &ImportType<'_>) {
        let mut imports = ImportSection::new();
        imports.import(ty.module(), ty.name(), extern_to_entity(&ty.ty()));
        self.module.section(&imports);
    }
    pub fn export(&mut self, ty: &ExportType<'_>) {
        let nth = self.next(&ty.ty());
        let section_name = format!("item{}", nth);
        let mut exports = ExportSection::new();
        let item_ty = ty.ty();
        self.item(&item_ty);
        let export = extern_to_export(&item_ty, |_| nth );
        exports.export(&section_name, export);
        self.module.section(&exports);
    }
    fn item(&mut self, ty: &ExternType) {
        match ty {
            ExternType::Memory(mem) => {
                let mut memories = MemorySection::new();
                memories.memory(wasm_encoder::MemoryType {
                    limits: Limits {
                        min: mem.limits().min(),
                        max: mem.limits().max(),
                    },
                });
                self.module.section(&memories);
            }
            ExternType::Table(table) => {
                let mut tables = TableSection::new();
                tables.table(wasm_encoder::TableType {
                    element_type: wasm_encoder::ValType::FuncRef,
                    limits: Limits {
                        min: table.limits().min(),
                        max: table.limits().max(),
                    },
                });
                self.module.section(&tables);
            }
            ExternType::Global(global) => {
                let mut globals = GlobalSection::new();
                globals.global(
                    wasm_encoder::GlobalType {
                        val_type: wasm_encoder::ValType::I32,
                        mutable: global.mutability() == Mutability::Var,
                    },
                    value_to_instruction(&global.content()),
                );
                self.module.section(&globals);
            }
            ExternType::Func(ty) => {
                let mut types = TypeSection::new();
                types.function(
                    ty.params().into_iter().map(|it| value_to_value(&it)),
                    ty.results().into_iter().map(|it| value_to_value(&it)),
                );
                let mut functions = FunctionSection::new();
                functions.function(0);

                let locals = vec![];
                let mut func = Function::new(locals);
                for ty in ty.results() {
                    func.instruction(value_to_instruction(&ty));
                }
                let mut codes = CodeSection::new();
                codes.function(&func);
                self.module.section(&types);
                self.module.section(&functions);
                self.module.section(&codes);
            }
            ExternType::Module(ty) => {
                let mut types = TypeSection::new();
                types.module(
                    ty.imports()
                        .into_iter()
                        .map(|x| (x.module(), x.name(), extern_to_entity(&x.ty()))),
                    ty.exports()
                        .into_iter()
                        .map(|x| (x.name(), extern_to_entity(&x.ty()))),
                );

                let mut modules = ModuleSection::new();
                modules.module(&Module::new());
                modules.module(&Module::new());

//                let mut module = Module::new()
//                let mut module_code = ModuleSection::new();
//                module_code.module(&Module::new());

                self.module.section(&types);
                self.module.section(&modules);
//              self.module.section(&module_code);
            }
            ExternType::Instance(ty) => {
                let mut instances = InstanceSection::new();
                instances.instantiate(
                    0,
                    ty.exports()
                        .into_iter()
                        .map(|it| (it.name(),None,extern_to_export(&it.ty(), |et| self.next(et)))).collect::<Vec<_>>(),
                );
                self.module.section(&instances);
            }
        }
    }
}
fn value_to_instruction(ty: &ValType) -> Instruction {
    match ty {
        ValType::I32 => Instruction::I32Const(0),
        ValType::I64 => Instruction::I64Const(0),
        ValType::F32 => Instruction::F32Const(0.0),
        ValType::F64 => Instruction::F64Const(0.0),
        ValType::V128 => Instruction::V128Const(0i128), 
        ValType::ExternRef => Instruction::RefNull(wasm_encoder::ValType::ExternRef),
        ValType::FuncRef => Instruction::RefNull(wasm_encoder::ValType::FuncRef),
    }
}
fn extern_to_entity(val: &wasmtime::ExternType) -> wasm_encoder::EntityType {
    match val {
        wasmtime::ExternType::Func(_) => wasm_encoder::EntityType::Function(0),
        wasmtime::ExternType::Global(x) => {
            wasm_encoder::EntityType::Global(wasm_encoder::GlobalType {
                val_type: value_to_value(x.content()),
                mutable: x.mutability() == Mutability::Var,
            })
        }
        wasmtime::ExternType::Table(x) => {
            wasm_encoder::EntityType::Table(wasm_encoder::TableType {
                element_type: value_to_value(&x.element()),
                limits: wasm_encoder::Limits {
                    min: x.limits().min(),
                    max: x.limits().max(),
                },
            })
        }
        wasmtime::ExternType::Memory(x) => {
            wasm_encoder::EntityType::Memory(wasm_encoder::MemoryType {
                limits: wasm_encoder::Limits {
                    min: x.limits().min(),
                    max: x.limits().max(),
                },
            })
        }
        wasmtime::ExternType::Instance(_) => wasm_encoder::EntityType::Instance(0),
        wasmtime::ExternType::Module(_) => wasm_encoder::EntityType::Module(0),
    }
}
fn value_to_value(from: &ValType) -> wasm_encoder::ValType {
    match from {
        ValType::I32 => wasm_encoder::ValType::I32,
        ValType::I64 => wasm_encoder::ValType::I64,
        ValType::F32 => wasm_encoder::ValType::F32,
        ValType::F64 => wasm_encoder::ValType::F64,
        ValType::V128 => wasm_encoder::ValType::V128, 
        ValType::ExternRef => wasm_encoder::ValType::ExternRef,
        ValType::FuncRef => wasm_encoder::ValType::FuncRef,
    }
}
fn extern_to_export<F>(val: &wasmtime::ExternType, mut fn_next: F) -> wasm_encoder::Export
          where F: FnMut(&wasmtime::ExternType) -> u32  
{
    match val {
        wasmtime::ExternType::Func(_) => Export::Function(fn_next(val)),
        wasmtime::ExternType::Global(_) => Export::Global(fn_next(val)),
        wasmtime::ExternType::Table(_) => Export::Table(fn_next(val)),
        wasmtime::ExternType::Memory(_) => Export::Memory(fn_next(val)),
        wasmtime::ExternType::Instance(_) => Export::Instance(fn_next(val)),
        wasmtime::ExternType::Module(_) => Export::Module(fn_next(val)),
    }
}
