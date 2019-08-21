use crate::runtime::Store;
use crate::types::{
    ExportType, ExternType, FuncType, GlobalType, ImportType, Limits, MemoryType, Mutability,
    ValType,
};
use failure::Error;
use std::cell::RefCell;
use std::rc::Rc;

use wasmparser::{validate, ExternalKind, ImportSectionEntryType, ModuleReader, SectionCode};

fn into_memory_type(mt: wasmparser::MemoryType) -> MemoryType {
    assert!(!mt.shared);
    MemoryType::new(Limits::new(
        mt.limits.initial,
        mt.limits.maximum.unwrap_or(::std::u32::MAX),
    ))
}

fn into_global_type(gt: &wasmparser::GlobalType) -> GlobalType {
    let mutability = if gt.mutable {
        Mutability::Var
    } else {
        Mutability::Const
    };
    GlobalType::new(into_valtype(&gt.content_type), mutability)
}

fn into_valtype(ty: &wasmparser::Type) -> ValType {
    use wasmparser::Type::*;
    match ty {
        I32 => ValType::I32,
        I64 => ValType::I64,
        F32 => ValType::F32,
        F64 => ValType::F64,
        _ => unimplemented!("types in into_valtype"),
    }
}

fn into_func_type(mt: wasmparser::FuncType) -> FuncType {
    assert!(mt.form == wasmparser::Type::Func);
    let params = mt.params.iter().map(into_valtype).collect::<Vec<_>>();
    let returns = mt.returns.iter().map(into_valtype).collect::<Vec<_>>();
    FuncType::new(params.into_boxed_slice(), returns.into_boxed_slice())
}

fn read_imports_and_exports(
    binary: &[u8],
) -> Result<(Box<[ImportType]>, Box<[ExportType]>), Error> {
    let mut reader = ModuleReader::new(binary)?;
    let mut imports = Vec::new();
    let mut exports = Vec::new();
    let mut memories = Vec::new();
    let mut func_sig = Vec::new();
    let mut sigs = Vec::new();
    let mut globals = Vec::new();
    while !reader.eof() {
        let section = reader.read()?;
        match section.code {
            SectionCode::Memory => {
                let section = section.get_memory_section_reader()?;
                memories.reserve_exact(section.get_count() as usize);
                for entry in section {
                    memories.push(into_memory_type(entry?));
                }
            }
            SectionCode::Type => {
                let section = section.get_type_section_reader()?;
                sigs.reserve_exact(section.get_count() as usize);
                for entry in section {
                    sigs.push(into_func_type(entry?));
                }
            }
            SectionCode::Function => {
                let section = section.get_function_section_reader()?;
                sigs.reserve_exact(section.get_count() as usize);
                for entry in section {
                    func_sig.push(entry?);
                }
            }
            SectionCode::Global => {
                let section = section.get_global_section_reader()?;
                globals.reserve_exact(section.get_count() as usize);
                for entry in section {
                    globals.push(into_global_type(&entry?.ty));
                }
            }
            SectionCode::Import => {
                let section = section.get_import_section_reader()?;
                imports.reserve_exact(section.get_count() as usize);
                for entry in section {
                    let entry = entry?;
                    let module = String::from(entry.module).into();
                    let name = String::from(entry.field).into();
                    let r#type = match entry.ty {
                        ImportSectionEntryType::Function(index) => {
                            func_sig.push(index);
                            let sig = &sigs[index as usize];
                            ExternType::ExternFunc(sig.clone())
                        }
                        ImportSectionEntryType::Table(_tt) => {
                            unimplemented!("ImportSectionEntryType::Table")
                        }
                        ImportSectionEntryType::Memory(mt) => {
                            let memory = into_memory_type(mt);
                            memories.push(memory.clone());
                            ExternType::ExternMemory(memory)
                        }
                        ImportSectionEntryType::Global(gt) => {
                            let global = into_global_type(&gt);
                            globals.push(global.clone());
                            ExternType::ExternGlobal(global)
                        }
                    };
                    imports.push(ImportType::new(module, name, r#type));
                }
            }
            SectionCode::Export => {
                let section = section.get_export_section_reader()?;
                exports.reserve_exact(section.get_count() as usize);
                for entry in section {
                    let entry = entry?;
                    let name = String::from(entry.field).into();
                    let r#type = match entry.kind {
                        ExternalKind::Function => {
                            let sig_index = func_sig[entry.index as usize] as usize;
                            let sig = &sigs[sig_index];
                            ExternType::ExternFunc(sig.clone())
                        }
                        ExternalKind::Table => unimplemented!("ExternalKind::Table"),
                        ExternalKind::Memory => {
                            ExternType::ExternMemory(memories[entry.index as usize].clone())
                        }
                        ExternalKind::Global => {
                            ExternType::ExternGlobal(globals[entry.index as usize].clone())
                        }
                    };
                    exports.push(ExportType::new(name, r#type));
                }
            }
            _ => {
                // skip other sections
            }
        }
    }
    Ok((imports.into_boxed_slice(), exports.into_boxed_slice()))
}

#[derive(Clone)]
pub struct Module {
    store: Rc<RefCell<Store>>,
    binary: Box<[u8]>,
    imports: Box<[ImportType]>,
    exports: Box<[ExportType]>,
}

impl Module {
    pub fn new(store: Rc<RefCell<Store>>, binary: &[u8]) -> Result<Module, Error> {
        let (imports, exports) = read_imports_and_exports(binary)?;
        Ok(Module {
            store,
            binary: binary.into(),
            imports,
            exports,
        })
    }
    pub(crate) fn binary(&self) -> &[u8] {
        &self.binary
    }
    pub fn validate(_store: &Store, binary: &[u8]) -> bool {
        validate(binary, None)
    }
    pub fn imports(&self) -> &[ImportType] {
        &self.imports
    }
    pub fn exports(&self) -> &[ExportType] {
        &self.exports
    }
}
