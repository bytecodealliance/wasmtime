use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use cranelift_codegen::ir;

// Type Representations

// Type attributes

#[derive(Debug, Clone, Copy)]
pub enum Mutability {
    Const,
    Var,
}

#[derive(Debug, Clone)]
pub struct Limits {
    min: u32,
    max: u32,
}

impl Limits {
    pub fn new(min: u32, max: u32) -> Limits {
        Limits { min, max }
    }

    pub fn at_least(min: u32) -> Limits {
        Limits {
            min,
            max: ::core::u32::MAX,
        }
    }

    pub fn min(&self) -> u32 {
        self.min
    }

    pub fn max(&self) -> u32 {
        self.max
    }
}

// Value Types

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ValType {
    I32,
    I64,
    F32,
    F64,
    V128,
    AnyRef, /* = 128 */
    FuncRef,
}

impl ValType {
    pub fn is_num(&self) -> bool {
        match self {
            ValType::I32 | ValType::I64 | ValType::F32 | ValType::F64 => true,
            _ => false,
        }
    }

    pub fn is_ref(&self) -> bool {
        match self {
            ValType::AnyRef | ValType::FuncRef => true,
            _ => false,
        }
    }

    pub(crate) fn get_cranelift_type(&self) -> ir::Type {
        match self {
            ValType::I32 => ir::types::I32,
            ValType::I64 => ir::types::I64,
            ValType::F32 => ir::types::F32,
            ValType::F64 => ir::types::F64,
            ValType::V128 => ir::types::I8X16,
            _ => unimplemented!("get_cranelift_type other"),
        }
    }

    pub(crate) fn from_cranelift_type(ty: ir::Type) -> ValType {
        match ty {
            ir::types::I32 => ValType::I32,
            ir::types::I64 => ValType::I64,
            ir::types::F32 => ValType::F32,
            ir::types::F64 => ValType::F64,
            ir::types::I8X16 => ValType::V128,
            _ => unimplemented!("from_cranelift_type other"),
        }
    }
}

// External Types

#[derive(Debug, Clone)]
pub enum ExternType {
    ExternFunc(FuncType),
    ExternGlobal(GlobalType),
    ExternTable(TableType),
    ExternMemory(MemoryType),
}

impl ExternType {
    pub fn func(&self) -> &FuncType {
        match self {
            ExternType::ExternFunc(func) => func,
            _ => panic!("ExternType::ExternFunc expected"),
        }
    }
    pub fn global(&self) -> &GlobalType {
        match self {
            ExternType::ExternGlobal(func) => func,
            _ => panic!("ExternType::ExternGlobal expected"),
        }
    }
    pub fn table(&self) -> &TableType {
        match self {
            ExternType::ExternTable(table) => table,
            _ => panic!("ExternType::ExternTable expected"),
        }
    }
    pub fn memory(&self) -> &MemoryType {
        match self {
            ExternType::ExternMemory(memory) => memory,
            _ => panic!("ExternType::ExternMemory expected"),
        }
    }
    pub(crate) fn from_wasmtime_export(export: &wasmtime_runtime::Export) -> Self {
        match export {
            wasmtime_runtime::Export::Function { signature, .. } => {
                ExternType::ExternFunc(FuncType::from_cranelift_signature(signature.clone()))
            }
            wasmtime_runtime::Export::Memory { memory, .. } => {
                ExternType::ExternMemory(MemoryType::from_cranelift_memory(&memory.memory))
            }
            wasmtime_runtime::Export::Global { global, .. } => {
                ExternType::ExternGlobal(GlobalType::from_cranelift_global(&global))
            }
            wasmtime_runtime::Export::Table { table, .. } => {
                ExternType::ExternTable(TableType::from_cranelift_table(&table.table))
            }
        }
    }
}

// Function Types
fn from_cranelift_abiparam(param: &ir::AbiParam) -> ValType {
    assert_eq!(param.purpose, ir::ArgumentPurpose::Normal);
    ValType::from_cranelift_type(param.value_type)
}

#[derive(Debug, Clone)]
pub struct FuncType {
    params: Box<[ValType]>,
    results: Box<[ValType]>,
    signature: ir::Signature,
}

impl FuncType {
    pub fn new(params: Box<[ValType]>, results: Box<[ValType]>) -> FuncType {
        use cranelift_codegen::ir::*;
        use cranelift_codegen::isa::CallConv;
        use target_lexicon::HOST;
        let call_conv = CallConv::triple_default(&HOST);
        let signature: Signature = {
            let mut params = params
                .iter()
                .map(|p| AbiParam::new(p.get_cranelift_type()))
                .collect::<Vec<_>>();
            let returns = results
                .iter()
                .map(|p| AbiParam::new(p.get_cranelift_type()))
                .collect::<Vec<_>>();
            params.insert(0, AbiParam::special(types::I64, ArgumentPurpose::VMContext));

            Signature {
                params,
                returns,
                call_conv,
            }
        };
        FuncType {
            params,
            results,
            signature,
        }
    }
    pub fn params(&self) -> &[ValType] {
        &self.params
    }
    pub fn results(&self) -> &[ValType] {
        &self.results
    }

    pub(crate) fn get_cranelift_signature(&self) -> &ir::Signature {
        &self.signature
    }

    pub(crate) fn from_cranelift_signature(signature: ir::Signature) -> FuncType {
        let params = signature
            .params
            .iter()
            .filter(|p| p.purpose == ir::ArgumentPurpose::Normal)
            .map(|p| from_cranelift_abiparam(p))
            .collect::<Vec<_>>();
        let results = signature
            .returns
            .iter()
            .map(|p| from_cranelift_abiparam(p))
            .collect::<Vec<_>>();
        FuncType {
            params: params.into_boxed_slice(),
            results: results.into_boxed_slice(),
            signature,
        }
    }
}

// Global Types

#[derive(Debug, Clone)]
pub struct GlobalType {
    content: ValType,
    mutability: Mutability,
}

impl GlobalType {
    pub fn new(content: ValType, mutability: Mutability) -> GlobalType {
        GlobalType {
            content,
            mutability,
        }
    }
    pub fn content(&self) -> &ValType {
        &self.content
    }
    pub fn mutability(&self) -> Mutability {
        self.mutability
    }

    pub(crate) fn from_cranelift_global(global: &cranelift_wasm::Global) -> GlobalType {
        let ty = ValType::from_cranelift_type(global.ty);
        let mutability = if global.mutability {
            Mutability::Var
        } else {
            Mutability::Const
        };
        GlobalType::new(ty, mutability)
    }
}

// Table Types

#[derive(Debug, Clone)]
pub struct TableType {
    element: ValType,
    limits: Limits,
}

impl TableType {
    pub fn new(element: ValType, limits: Limits) -> TableType {
        TableType { element, limits }
    }
    pub fn element(&self) -> &ValType {
        &self.element
    }
    pub fn limits(&self) -> &Limits {
        &self.limits
    }

    pub(crate) fn from_cranelift_table(table: &cranelift_wasm::Table) -> TableType {
        assert!(if let cranelift_wasm::TableElementType::Func = table.ty {
            true
        } else {
            false
        });
        let ty = ValType::FuncRef;
        let limits = Limits::new(table.minimum, table.maximum.unwrap_or(::core::u32::MAX));
        TableType::new(ty, limits)
    }
}

// Memory Types

#[derive(Debug, Clone)]
pub struct MemoryType {
    limits: Limits,
}

impl MemoryType {
    pub fn new(limits: Limits) -> MemoryType {
        MemoryType { limits }
    }
    pub fn limits(&self) -> &Limits {
        &self.limits
    }

    pub(crate) fn from_cranelift_memory(memory: &cranelift_wasm::Memory) -> MemoryType {
        MemoryType::new(Limits::new(
            memory.minimum,
            memory.maximum.unwrap_or(::core::u32::MAX),
        ))
    }
}

// Import Types

#[derive(Debug, Clone)]
pub struct Name(String);

impl Name {
    pub fn new(value: &str) -> Self {
        Name(value.to_owned())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for Name {
    fn from(s: String) -> Name {
        Name(s)
    }
}

impl ::alloc::string::ToString for Name {
    fn to_string(&self) -> String {
        self.0.to_owned()
    }
}

#[derive(Debug, Clone)]
pub struct ImportType {
    module: Name,
    name: Name,
    r#type: ExternType,
}

impl ImportType {
    pub fn new(module: Name, name: Name, r#type: ExternType) -> ImportType {
        ImportType {
            module,
            name,
            r#type,
        }
    }
    pub fn module(&self) -> &Name {
        &self.module
    }
    pub fn name(&self) -> &Name {
        &self.name
    }
    pub fn r#type(&self) -> &ExternType {
        &self.r#type
    }
}

// Export Types

#[derive(Debug, Clone)]
pub struct ExportType {
    name: Name,
    r#type: ExternType,
}

impl ExportType {
    pub fn new(name: Name, r#type: ExternType) -> ExportType {
        ExportType { name, r#type }
    }
    pub fn name(&self) -> &Name {
        &self.name
    }
    pub fn r#type(&self) -> &ExternType {
        &self.r#type
    }
}
