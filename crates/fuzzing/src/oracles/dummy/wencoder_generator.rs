use wasm_encoder::{
    CodeSection, CustomSection, ElementSection, Export, ExportSection, Function, FunctionSection,
    GlobalSection, ImportSection, InstanceSection, Instruction, Limits, MemorySection, Module,
    ModuleSection, StartSection, TableSection, TypeSection,
};
use wasmtime::*;
pub struct WencoderGenerator<'a> {
    next: [u32; 6],
    function_section: FunctionSection,
    type_section: TypeSection,
    memory_section: MemorySection,
    table_section: TableSection,
    global_section: GlobalSection,
    import_section: ImportSection,
    instance_section: InstanceSection,
    code_section: CodeSection,
    export_section: ExportSection,
    module_section: ModuleSection,
    custom_section: CustomSection<'a>,
    start_section: StartSection,
    element_section: ElementSection,
}

fn next_index(wg: &mut [u32; 6], ty: &ExternType) -> u32 {
    let index = match ty {
        ExternType::Memory(_) => 0,
        ExternType::Table(_) => 1,
        ExternType::Global(_) => 2,
        ExternType::Func(_) => 3,
        ExternType::Instance(_) => 4,
        ExternType::Module(_) => 5,
    };
    wg[index] += 1;
    wg[index]
}

impl<'a> WencoderGenerator<'a> {
    pub fn new() -> WencoderGenerator<'a> {
        WencoderGenerator {
            next: [0, 0, 0, 0, 0, 0],
            custom_section: CustomSection {
                name: "test",
                data: &[11, 22, 33, 44],
            },
            type_section: TypeSection::new(),
            import_section: ImportSection::new(),
            function_section: FunctionSection::new(),
            table_section: TableSection::new(),
            memory_section: MemorySection::new(),
            global_section: GlobalSection::new(),
            export_section: ExportSection::new(),
            start_section: StartSection { function_index: 0 },
            element_section: ElementSection::new(),
            instance_section: InstanceSection::new(),
            code_section: CodeSection::new(),

            module_section: ModuleSection::new(),
        }
    }
    pub fn finish(self) -> Vec<u8> {
        let mut module = Module::new();

        module.section(&self.custom_section);
        module.section(&self.type_section);
        module.section(&self.import_section);
        module.section(&self.function_section);
        module.section(&self.table_section);
        module.section(&self.memory_section);
        module.section(&self.global_section);
        module.section(&self.export_section);
        module.section(&self.start_section);
        module.section(&self.element_section);
        module.section(&self.code_section);
        module.section(&self.module_section);

        module.finish()
    }
    pub fn import(&mut self, ty: &ImportType<'_>) {
        let imports = &mut self.import_section;
        imports.import(ty.module(), ty.name(), extern_to_entity(&ty.ty()));
    }
    pub fn export(&mut self, ty: &ExportType<'_>) {
        let nth = next_index(&mut self.next, &ty.ty());
        let section_name = format!("item{}", nth);

        let item_ty = ty.ty();
        self.item(&item_ty);

        let export = extern_to_export(&item_ty, |_| nth);
        let exports = &mut self.export_section;
        exports.export(&section_name, export);
    }
    fn item(&mut self, ty: &ExternType) {
        match ty {
            ExternType::Memory(mem) => {
                let memories = &mut self.memory_section;
                memories.memory(wasm_encoder::MemoryType {
                    limits: Limits {
                        min: mem.limits().min(),
                        max: mem.limits().max(),
                    },
                });
            }
            ExternType::Table(table) => {
                let tables = &mut self.table_section;
                tables.table(wasm_encoder::TableType {
                    element_type: wasm_encoder::ValType::FuncRef,
                    limits: Limits {
                        min: table.limits().min(),
                        max: table.limits().max(),
                    },
                });
            }
            ExternType::Global(global) => {
                let globals = &mut self.global_section;
                globals.global(
                    wasm_encoder::GlobalType {
                        val_type: wasm_encoder::ValType::I32,
                        mutable: global.mutability() == Mutability::Var,
                    },
                    value_to_instruction(&global.content()),
                );
            }
            ExternType::Func(ty) => {
                let types = &mut self.type_section;
                types.function(
                    ty.params().into_iter().map(|it| value_to_value(&it)),
                    ty.results().into_iter().map(|it| value_to_value(&it)),
                );
                self.function_section.function(0);

                let locals = vec![];
                let mut func = Function::new(locals);
                for ty in ty.results() {
                    func.instruction(value_to_instruction(&ty));
                }
                let codes = &mut self.code_section;
                codes.function(&func);
            }
            ExternType::Module(ty) => {
                let types = &mut self.type_section;
                types.module(
                    ty.imports()
                        .into_iter()
                        .map(|x| (x.module(), x.name(), extern_to_entity(&x.ty()))),
                    ty.exports()
                        .into_iter()
                        .map(|x| (x.name(), extern_to_entity(&x.ty()))),
                );

                let modules = &mut self.module_section;
                modules.module(&Module::new());
                modules.module(&Module::new());
            }
            ExternType::Instance(ty) => {
                let instances = &mut self.instance_section;
                let next_index_a = &mut self.next;

                instances.instantiate(
                    0,
                    ty.exports()
                        .into_iter()
                        .map(|it| {
                            (
                                it.name(),
                                extern_to_export(&it.ty(), |et| next_index(next_index_a, et)),
                            )
                        })
                        .collect::<Vec<_>>(),
                );
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
where
    F: FnMut(&wasmtime::ExternType) -> u32,
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
