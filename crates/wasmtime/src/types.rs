use std::fmt;
use wasmtime_environ::{EntityType, Global, Memory, Table, WasmFuncType, WasmType};
use wasmtime_jit::TypeTables;

pub(crate) mod matching;

// Type Representations

// Type attributes

/// Indicator of whether a global is mutable or not
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum Mutability {
    /// The global is constant and its value does not change
    Const,
    /// The value of the global can change over time
    Var,
}

// Value Types

/// A list of all possible value types in WebAssembly.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum ValType {
    // NB: the ordering here is intended to match the ordering in
    // `wasmtime_types::WasmType` to help improve codegen when converting.
    /// Signed 32 bit integer.
    I32,
    /// Signed 64 bit integer.
    I64,
    /// Floating point 32 bit integer.
    F32,
    /// Floating point 64 bit integer.
    F64,
    /// A 128 bit number.
    V128,
    /// A reference to a Wasm function.
    FuncRef,
    /// A reference to opaque data in the Wasm instance.
    ExternRef,
}

impl fmt::Display for ValType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ValType::I32 => write!(f, "i32"),
            ValType::I64 => write!(f, "i64"),
            ValType::F32 => write!(f, "f32"),
            ValType::F64 => write!(f, "f64"),
            ValType::V128 => write!(f, "v128"),
            ValType::ExternRef => write!(f, "externref"),
            ValType::FuncRef => write!(f, "funcref"),
        }
    }
}

impl ValType {
    /// Returns true if `ValType` matches any of the numeric types. (e.g. `I32`,
    /// `I64`, `F32`, `F64`).
    pub fn is_num(&self) -> bool {
        match self {
            ValType::I32 | ValType::I64 | ValType::F32 | ValType::F64 => true,
            _ => false,
        }
    }

    /// Returns true if `ValType` matches either of the reference types.
    pub fn is_ref(&self) -> bool {
        match self {
            ValType::ExternRef | ValType::FuncRef => true,
            _ => false,
        }
    }

    pub(crate) fn to_wasm_type(&self) -> WasmType {
        match self {
            Self::I32 => WasmType::I32,
            Self::I64 => WasmType::I64,
            Self::F32 => WasmType::F32,
            Self::F64 => WasmType::F64,
            Self::V128 => WasmType::V128,
            Self::FuncRef => WasmType::FuncRef,
            Self::ExternRef => WasmType::ExternRef,
        }
    }

    pub(crate) fn from_wasm_type(ty: &WasmType) -> Self {
        match ty {
            WasmType::I32 => Self::I32,
            WasmType::I64 => Self::I64,
            WasmType::F32 => Self::F32,
            WasmType::F64 => Self::F64,
            WasmType::V128 => Self::V128,
            WasmType::FuncRef => Self::FuncRef,
            WasmType::ExternRef => Self::ExternRef,
            WasmType::ExnRef => unimplemented!(),
        }
    }
}

// External Types

/// A list of all possible types which can be externally referenced from a
/// WebAssembly module.
///
/// This list can be found in [`ImportType`] or [`ExportType`], so these types
/// can either be imported or exported.
#[derive(Debug, Clone)]
pub enum ExternType {
    /// This external type is the type of a WebAssembly function.
    Func(FuncType),
    /// This external type is the type of a WebAssembly global.
    Global(GlobalType),
    /// This external type is the type of a WebAssembly table.
    Table(TableType),
    /// This external type is the type of a WebAssembly memory.
    Memory(MemoryType),
    /// This external type is the type of a WebAssembly instance.
    Instance(InstanceType),
    /// This external type is the type of a WebAssembly module.
    Module(ModuleType),
}

macro_rules! accessors {
    ($(($variant:ident($ty:ty) $get:ident $unwrap:ident))*) => ($(
        /// Attempt to return the underlying type of this external type,
        /// returning `None` if it is a different type.
        pub fn $get(&self) -> Option<&$ty> {
            if let ExternType::$variant(e) = self {
                Some(e)
            } else {
                None
            }
        }

        /// Returns the underlying descriptor of this [`ExternType`], panicking
        /// if it is a different type.
        ///
        /// # Panics
        ///
        /// Panics if `self` is not of the right type.
        pub fn $unwrap(&self) -> &$ty {
            self.$get().expect(concat!("expected ", stringify!($ty)))
        }
    )*)
}

impl ExternType {
    accessors! {
        (Func(FuncType) func unwrap_func)
        (Global(GlobalType) global unwrap_global)
        (Table(TableType) table unwrap_table)
        (Memory(MemoryType) memory unwrap_memory)
        (Module(ModuleType) module unwrap_module)
        (Instance(InstanceType) instance unwrap_instance)
    }

    pub(crate) fn from_wasmtime(types: &TypeTables, ty: &EntityType) -> ExternType {
        match ty {
            EntityType::Function(idx) => {
                FuncType::from_wasm_func_type(types.wasm_signatures[*idx].clone()).into()
            }
            EntityType::Global(ty) => GlobalType::from_wasmtime_global(ty).into(),
            EntityType::Memory(ty) => MemoryType::from_wasmtime_memory(ty).into(),
            EntityType::Table(ty) => TableType::from_wasmtime_table(ty).into(),
            EntityType::Module(ty) => {
                let ty = &types.module_signatures[*ty];
                ModuleType::from_wasmtime(types, ty).into()
            }
            EntityType::Instance(ty) => {
                let ty = &types.instance_signatures[*ty];
                InstanceType::from_wasmtime(types, ty).into()
            }
            EntityType::Tag(_) => unimplemented!("wasm tag support"),
        }
    }
}

impl From<FuncType> for ExternType {
    fn from(ty: FuncType) -> ExternType {
        ExternType::Func(ty)
    }
}

impl From<GlobalType> for ExternType {
    fn from(ty: GlobalType) -> ExternType {
        ExternType::Global(ty)
    }
}

impl From<MemoryType> for ExternType {
    fn from(ty: MemoryType) -> ExternType {
        ExternType::Memory(ty)
    }
}

impl From<TableType> for ExternType {
    fn from(ty: TableType) -> ExternType {
        ExternType::Table(ty)
    }
}

impl From<ModuleType> for ExternType {
    fn from(ty: ModuleType) -> ExternType {
        ExternType::Module(ty)
    }
}

impl From<InstanceType> for ExternType {
    fn from(ty: InstanceType) -> ExternType {
        ExternType::Instance(ty)
    }
}

/// A descriptor for a function in a WebAssembly module.
///
/// WebAssembly functions can have 0 or more parameters and results.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct FuncType {
    sig: WasmFuncType,
}

impl FuncType {
    /// Creates a new function descriptor from the given parameters and results.
    ///
    /// The function descriptor returned will represent a function which takes
    /// `params` as arguments and returns `results` when it is finished.
    pub fn new(
        params: impl IntoIterator<Item = ValType>,
        results: impl IntoIterator<Item = ValType>,
    ) -> FuncType {
        FuncType {
            sig: WasmFuncType::new(
                params.into_iter().map(|t| t.to_wasm_type()).collect(),
                results.into_iter().map(|t| t.to_wasm_type()).collect(),
            ),
        }
    }

    /// Returns the list of parameter types for this function.
    pub fn params(&self) -> impl ExactSizeIterator<Item = ValType> + '_ {
        self.sig.params().iter().map(ValType::from_wasm_type)
    }

    /// Returns the list of result types for this function.
    pub fn results(&self) -> impl ExactSizeIterator<Item = ValType> + '_ {
        self.sig.returns().iter().map(ValType::from_wasm_type)
    }

    pub(crate) fn as_wasm_func_type(&self) -> &WasmFuncType {
        &self.sig
    }

    pub(crate) fn from_wasm_func_type(sig: WasmFuncType) -> FuncType {
        Self { sig }
    }
}

// Global Types

/// A WebAssembly global descriptor.
///
/// This type describes an instance of a global in a WebAssembly module. Globals
/// are local to an [`Instance`](crate::Instance) and are either immutable or
/// mutable.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct GlobalType {
    content: ValType,
    mutability: Mutability,
}

impl GlobalType {
    /// Creates a new global descriptor of the specified `content` type and
    /// whether or not it's mutable.
    pub fn new(content: ValType, mutability: Mutability) -> GlobalType {
        GlobalType {
            content,
            mutability,
        }
    }

    /// Returns the value type of this global descriptor.
    pub fn content(&self) -> &ValType {
        &self.content
    }

    /// Returns whether or not this global is mutable.
    pub fn mutability(&self) -> Mutability {
        self.mutability
    }

    /// Returns `None` if the wasmtime global has a type that we can't
    /// represent, but that should only very rarely happen and indicate a bug.
    pub(crate) fn from_wasmtime_global(global: &Global) -> GlobalType {
        let ty = ValType::from_wasm_type(&global.wasm_ty);
        let mutability = if global.mutability {
            Mutability::Var
        } else {
            Mutability::Const
        };
        GlobalType::new(ty, mutability)
    }
}

// Table Types

/// A descriptor for a table in a WebAssembly module.
///
/// Tables are contiguous chunks of a specific element, typically a `funcref` or
/// an `externref`. The most common use for tables is a function table through
/// which `call_indirect` can invoke other functions.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct TableType {
    ty: Table,
}

impl TableType {
    /// Creates a new table descriptor which will contain the specified
    /// `element` and have the `limits` applied to its length.
    pub fn new(element: ValType, min: u32, max: Option<u32>) -> TableType {
        TableType {
            ty: Table {
                wasm_ty: element.to_wasm_type(),
                minimum: min,
                maximum: max,
            },
        }
    }

    /// Returns the element value type of this table.
    pub fn element(&self) -> ValType {
        ValType::from_wasm_type(&self.ty.wasm_ty)
    }

    /// Returns minimum number of elements this table must have
    pub fn minimum(&self) -> u32 {
        self.ty.minimum
    }

    /// Returns the optionally-specified maximum number of elements this table
    /// can have.
    ///
    /// If this returns `None` then the table is not limited in size.
    pub fn maximum(&self) -> Option<u32> {
        self.ty.maximum
    }

    pub(crate) fn from_wasmtime_table(table: &Table) -> TableType {
        TableType { ty: table.clone() }
    }

    pub(crate) fn wasmtime_table(&self) -> &Table {
        &self.ty
    }
}

// Memory Types

/// A descriptor for a WebAssembly memory type.
///
/// Memories are described in units of pages (64KB) and represent contiguous
/// chunks of addressable memory.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct MemoryType {
    ty: Memory,
}

impl MemoryType {
    /// Creates a new descriptor for a 32-bit WebAssembly memory given the
    /// specified limits of the memory.
    ///
    /// The `minimum` and `maximum`  values here are specified in units of
    /// WebAssembly pages, which are 64k.
    pub fn new(minimum: u32, maximum: Option<u32>) -> MemoryType {
        MemoryType {
            ty: Memory {
                memory64: false,
                shared: false,
                minimum: minimum.into(),
                maximum: maximum.map(|i| i.into()),
            },
        }
    }

    /// Creates a new descriptor for a 64-bit WebAssembly memory given the
    /// specified limits of the memory.
    ///
    /// The `minimum` and `maximum`  values here are specified in units of
    /// WebAssembly pages, which are 64k.
    ///
    /// Note that 64-bit memories are part of the memory64 proposal for
    /// WebAssembly which is not standardized yet.
    pub fn new64(minimum: u64, maximum: Option<u64>) -> MemoryType {
        MemoryType {
            ty: Memory {
                memory64: true,
                shared: false,
                minimum,
                maximum,
            },
        }
    }

    /// Returns whether this is a 64-bit memory or not.
    ///
    /// Note that 64-bit memories are part of the memory64 proposal for
    /// WebAssembly which is not standardized yet.
    pub fn is_64(&self) -> bool {
        self.ty.memory64
    }

    /// Returns minimum number of WebAssembly pages this memory must have.
    ///
    /// Note that the return value, while a `u64`, will always fit into a `u32`
    /// for 32-bit memories.
    pub fn minimum(&self) -> u64 {
        self.ty.minimum
    }

    /// Returns the optionally-specified maximum number of pages this memory
    /// can have.
    ///
    /// If this returns `None` then the memory is not limited in size.
    ///
    /// Note that the return value, while a `u64`, will always fit into a `u32`
    /// for 32-bit memories.
    pub fn maximum(&self) -> Option<u64> {
        self.ty.maximum
    }

    pub(crate) fn from_wasmtime_memory(memory: &Memory) -> MemoryType {
        MemoryType { ty: memory.clone() }
    }

    pub(crate) fn wasmtime_memory(&self) -> &Memory {
        &self.ty
    }
}

// Module Types

/// A descriptor for a WebAssembly module type.
///
/// This is a part of the [WebAssembly module-linking proposal][proposal].
///
/// [proposal]: https://github.com/webassembly/module-linking
#[derive(Debug, Clone)]
pub struct ModuleType {
    imports: Vec<(String, Option<String>, ExternType)>,
    exports: Vec<(String, ExternType)>,
}

impl ModuleType {
    /// Creates a new empty module type.
    pub fn new() -> ModuleType {
        ModuleType {
            imports: Vec::new(),
            exports: Vec::new(),
        }
    }

    /// Adds a new export to this `ModuleType`.
    pub fn add_named_export(&mut self, name: &str, ty: ExternType) {
        self.exports.push((name.to_string(), ty));
    }

    /// Adds a new import to this `ModuleType`.
    pub fn add_named_import(&mut self, module: &str, field: Option<&str>, ty: ExternType) {
        self.imports
            .push((module.to_string(), field.map(|f| f.to_string()), ty));
    }

    /// Returns the list of imports associated with this module type.
    pub fn imports(&self) -> impl ExactSizeIterator<Item = ImportType<'_>> {
        self.imports.iter().map(|(name, field, ty)| ImportType {
            module: name,
            name: field.as_deref(),
            ty: EntityOrExtern::Extern(ty),
        })
    }

    /// Returns the list of exports associated with this module type.
    pub fn exports(&self) -> impl ExactSizeIterator<Item = ExportType<'_>> {
        self.exports.iter().map(|(name, ty)| ExportType {
            name,
            ty: EntityOrExtern::Extern(ty),
        })
    }

    pub(crate) fn from_wasmtime(
        types: &TypeTables,
        ty: &wasmtime_environ::ModuleSignature,
    ) -> ModuleType {
        let exports = &types.instance_signatures[ty.exports].exports;
        ModuleType {
            exports: exports
                .iter()
                .map(|(name, ty)| (name.to_string(), ExternType::from_wasmtime(types, ty)))
                .collect(),
            imports: ty
                .imports
                .iter()
                .map(|(m, ty)| (m.to_string(), None, ExternType::from_wasmtime(types, ty)))
                .collect(),
        }
    }
}

// Instance Types

/// A descriptor for a WebAssembly instance type.
///
/// This is a part of the [WebAssembly module-linking proposal][proposal].
///
/// [proposal]: https://github.com/webassembly/module-linking
#[derive(Debug, Clone)]
pub struct InstanceType {
    exports: Vec<(String, ExternType)>,
}

impl InstanceType {
    /// Creates a new empty instance type.
    pub fn new() -> InstanceType {
        InstanceType {
            exports: Vec::new(),
        }
    }

    /// Adds a new export to this `ModuleType`.
    pub fn add_named_export(&mut self, name: &str, ty: ExternType) {
        self.exports.push((name.to_string(), ty));
    }

    /// Returns the list of exports associated with this module type.
    pub fn exports(&self) -> impl ExactSizeIterator<Item = ExportType<'_>> {
        self.exports.iter().map(|(name, ty)| ExportType {
            name,
            ty: EntityOrExtern::Extern(ty),
        })
    }

    pub(crate) fn from_wasmtime(
        types: &TypeTables,
        ty: &wasmtime_environ::InstanceSignature,
    ) -> InstanceType {
        InstanceType {
            exports: ty
                .exports
                .iter()
                .map(|(name, ty)| (name.to_string(), ExternType::from_wasmtime(types, ty)))
                .collect(),
        }
    }
}

// Import Types

/// A descriptor for an imported value into a wasm module.
///
/// This type is primarily accessed from the
/// [`Module::imports`](crate::Module::imports) API. Each [`ImportType`]
/// describes an import into the wasm module with the module/name that it's
/// imported from as well as the type of item that's being imported.
#[derive(Clone)]
pub struct ImportType<'module> {
    /// The module of the import.
    module: &'module str,

    /// The field of the import.
    name: Option<&'module str>,

    /// The type of the import.
    ty: EntityOrExtern<'module>,
}

#[derive(Clone)]
enum EntityOrExtern<'a> {
    Entity(EntityType, &'a TypeTables),
    Extern(&'a ExternType),
}

impl<'module> ImportType<'module> {
    /// Creates a new import descriptor which comes from `module` and `name` and
    /// is of type `ty`.
    pub(crate) fn new(
        module: &'module str,
        name: Option<&'module str>,
        ty: EntityType,
        types: &'module TypeTables,
    ) -> ImportType<'module> {
        ImportType {
            module,
            name,
            ty: EntityOrExtern::Entity(ty, types),
        }
    }

    /// Returns the module name that this import is expected to come from.
    pub fn module(&self) -> &'module str {
        self.module
    }

    /// Returns the field name of the module that this import is expected to
    /// come from.
    ///
    /// Note that this is optional due to the module linking proposal. If the
    /// module linking proposal is enabled this is always `None`, otherwise this
    /// is always `Some`.
    pub fn name(&self) -> Option<&'module str> {
        self.name
    }

    /// Returns the expected type of this import.
    pub fn ty(&self) -> ExternType {
        match &self.ty {
            EntityOrExtern::Entity(e, types) => ExternType::from_wasmtime(types, e),
            EntityOrExtern::Extern(e) => (*e).clone(),
        }
    }
}

impl<'module> fmt::Debug for ImportType<'module> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImportType")
            .field("module", &self.module())
            .field("name", &self.name())
            .field("ty", &self.ty())
            .finish()
    }
}

// Export Types

/// A descriptor for an exported WebAssembly value.
///
/// This type is primarily accessed from the
/// [`Module::exports`](crate::Module::exports) accessor and describes what
/// names are exported from a wasm module and the type of the item that is
/// exported.
#[derive(Clone)]
pub struct ExportType<'module> {
    /// The name of the export.
    name: &'module str,

    /// The type of the export.
    ty: EntityOrExtern<'module>,
}

impl<'module> ExportType<'module> {
    /// Creates a new export which is exported with the given `name` and has the
    /// given `ty`.
    pub(crate) fn new(
        name: &'module str,
        ty: EntityType,
        types: &'module TypeTables,
    ) -> ExportType<'module> {
        ExportType {
            name,
            ty: EntityOrExtern::Entity(ty, types),
        }
    }

    /// Returns the name by which this export is known.
    pub fn name(&self) -> &'module str {
        self.name
    }

    /// Returns the type of this export.
    pub fn ty(&self) -> ExternType {
        match &self.ty {
            EntityOrExtern::Entity(e, types) => ExternType::from_wasmtime(types, e),
            EntityOrExtern::Extern(e) => (*e).clone(),
        }
    }
}

impl<'module> fmt::Debug for ExportType<'module> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExportType")
            .field("name", &self.name().to_owned())
            .field("ty", &self.ty())
            .finish()
    }
}
