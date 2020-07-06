use std::fmt;
use wasmtime_environ::{ir, wasm, EntityIndex};

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

/// Limits of tables/memories where the units of the limits are defined by the
/// table/memory types.
///
/// A minimum is always available but the maximum may not be present.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Limits {
    min: u32,
    max: Option<u32>,
}

impl Limits {
    /// Creates a new set of limits with the minimum and maximum both specified.
    pub fn new(min: u32, max: Option<u32>) -> Limits {
        Limits { min, max }
    }

    /// Creates a new `Limits` with the `min` specified and no maximum specified.
    pub fn at_least(min: u32) -> Limits {
        Limits::new(min, None)
    }

    /// Returns the minimum amount for these limits.
    pub fn min(&self) -> u32 {
        self.min
    }

    /// Returns the maximum amount for these limits, if specified.
    pub fn max(&self) -> Option<u32> {
        self.max
    }
}

// Value Types

/// A list of all possible value types in WebAssembly.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum ValType {
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
    /// A reference to opaque data in the Wasm instance.
    ExternRef, /* = 128 */
    /// A reference to a Wasm function.
    FuncRef,
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

    pub(crate) fn get_wasmtime_type(&self) -> ir::Type {
        match self {
            ValType::I32 => ir::types::I32,
            ValType::I64 => ir::types::I64,
            ValType::F32 => ir::types::F32,
            ValType::F64 => ir::types::F64,
            ValType::V128 => ir::types::I8X16,
            ValType::ExternRef => wasmtime_runtime::ref_type(),
            ValType::FuncRef => wasmtime_runtime::pointer_type(),
        }
    }

    pub(crate) fn to_wasm_type(&self) -> wasm::WasmType {
        match self {
            Self::I32 => wasm::WasmType::I32,
            Self::I64 => wasm::WasmType::I64,
            Self::F32 => wasm::WasmType::F32,
            Self::F64 => wasm::WasmType::F64,
            Self::V128 => wasm::WasmType::V128,
            Self::FuncRef => wasm::WasmType::FuncRef,
            Self::ExternRef => wasm::WasmType::ExternRef,
        }
    }

    pub(crate) fn from_wasm_type(ty: &wasm::WasmType) -> Option<Self> {
        match ty {
            wasm::WasmType::I32 => Some(Self::I32),
            wasm::WasmType::I64 => Some(Self::I64),
            wasm::WasmType::F32 => Some(Self::F32),
            wasm::WasmType::F64 => Some(Self::F64),
            wasm::WasmType::V128 => Some(Self::V128),
            wasm::WasmType::FuncRef => Some(Self::FuncRef),
            wasm::WasmType::ExternRef => Some(Self::ExternRef),
            wasm::WasmType::Func | wasm::WasmType::EmptyBlockType => None,
        }
    }
}

// External Types

/// A list of all possible types which can be externally referenced from a
/// WebAssembly module.
///
/// This list can be found in [`ImportType`] or [`ExportType`], so these types
/// can either be imported or exported.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum ExternType {
    /// This external type is the type of a WebAssembly function.
    Func(FuncType),
    /// This external type is the type of a WebAssembly global.
    Global(GlobalType),
    /// This external type is the type of a WebAssembly table.
    Table(TableType),
    /// This external type is the type of a WebAssembly memory.
    Memory(MemoryType),
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

/// A descriptor for a function in a WebAssembly module.
///
/// WebAssembly functions can have 0 or more parameters and results.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct FuncType {
    params: Box<[ValType]>,
    results: Box<[ValType]>,
}

impl FuncType {
    /// Creates a new function descriptor from the given parameters and results.
    ///
    /// The function descriptor returned will represent a function which takes
    /// `params` as arguments and returns `results` when it is finished.
    pub fn new(params: Box<[ValType]>, results: Box<[ValType]>) -> FuncType {
        FuncType { params, results }
    }

    /// Returns the list of parameter types for this function.
    pub fn params(&self) -> &[ValType] {
        &self.params
    }

    /// Returns the list of result types for this function.
    pub fn results(&self) -> &[ValType] {
        &self.results
    }

    pub(crate) fn to_wasm_func_type(&self) -> wasm::WasmFuncType {
        wasm::WasmFuncType {
            params: self.params.iter().map(|p| p.to_wasm_type()).collect(),
            returns: self.results.iter().map(|r| r.to_wasm_type()).collect(),
        }
    }

    /// Get the Cranelift-compatible function signature.
    pub(crate) fn get_wasmtime_signature(&self, pointer_type: ir::Type) -> ir::Signature {
        use wasmtime_environ::ir::{AbiParam, ArgumentPurpose, Signature};
        use wasmtime_jit::native;
        let call_conv = native::call_conv();
        let mut params = self
            .params
            .iter()
            .map(|p| AbiParam::new(p.get_wasmtime_type()))
            .collect::<Vec<_>>();
        let returns = self
            .results
            .iter()
            .map(|p| AbiParam::new(p.get_wasmtime_type()))
            .collect::<Vec<_>>();
        params.insert(
            0,
            AbiParam::special(pointer_type, ArgumentPurpose::VMContext),
        );
        params.insert(1, AbiParam::new(pointer_type));

        Signature {
            params,
            returns,
            call_conv,
        }
    }

    /// Returns `None` if any types in the signature can't be converted to the
    /// types in this crate, but that should very rarely happen and largely only
    /// indicate a bug in our cranelift integration.
    pub(crate) fn from_wasm_func_type(signature: &wasm::WasmFuncType) -> Option<FuncType> {
        let params = signature
            .params
            .iter()
            .map(|p| ValType::from_wasm_type(p))
            .collect::<Option<Vec<_>>>()?;
        let results = signature
            .returns
            .iter()
            .map(|r| ValType::from_wasm_type(r))
            .collect::<Option<Vec<_>>>()?;
        Some(FuncType {
            params: params.into_boxed_slice(),
            results: results.into_boxed_slice(),
        })
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
    pub(crate) fn from_wasmtime_global(global: &wasm::Global) -> Option<GlobalType> {
        let ty = ValType::from_wasm_type(&global.wasm_ty)?;
        let mutability = if global.mutability {
            Mutability::Var
        } else {
            Mutability::Const
        };
        Some(GlobalType::new(ty, mutability))
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
    element: ValType,
    limits: Limits,
}

impl TableType {
    /// Creates a new table descriptor which will contain the specified
    /// `element` and have the `limits` applied to its length.
    pub fn new(element: ValType, limits: Limits) -> TableType {
        TableType { element, limits }
    }

    /// Returns the element value type of this table.
    pub fn element(&self) -> &ValType {
        &self.element
    }

    /// Returns the limits, in units of elements, of this table.
    pub fn limits(&self) -> &Limits {
        &self.limits
    }

    pub(crate) fn from_wasmtime_table(table: &wasm::Table) -> TableType {
        let ty = match table.ty {
            wasm::TableElementType::Func => ValType::FuncRef,
            #[cfg(target_pointer_width = "64")]
            wasm::TableElementType::Val(ir::types::R64) => ValType::ExternRef,
            #[cfg(target_pointer_width = "32")]
            wasm::TableElementType::Val(ir::types::R32) => ValType::ExternRef,
            _ => panic!("only `funcref` and `externref` tables supported"),
        };
        let limits = Limits::new(table.minimum, table.maximum);
        TableType::new(ty, limits)
    }
}

// Memory Types

/// A descriptor for a WebAssembly memory type.
///
/// Memories are described in units of pages (64KB) and represent contiguous
/// chunks of addressable memory.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct MemoryType {
    limits: Limits,
}

impl MemoryType {
    /// Creates a new descriptor for a WebAssembly memory given the specified
    /// limits of the memory.
    pub fn new(limits: Limits) -> MemoryType {
        MemoryType { limits }
    }

    /// Returns the limits (in pages) that are configured for this memory.
    pub fn limits(&self) -> &Limits {
        &self.limits
    }

    pub(crate) fn from_wasmtime_memory(memory: &wasm::Memory) -> MemoryType {
        MemoryType::new(Limits::new(memory.minimum, memory.maximum))
    }
}

// Entity Types

#[derive(Clone, Hash, Eq, PartialEq)]
pub(crate) enum EntityType<'module> {
    Function(&'module wasm::WasmFuncType),
    Table(&'module wasm::Table),
    Memory(&'module wasm::Memory),
    Global(&'module wasm::Global),
}

impl<'module> EntityType<'module> {
    /// Translate from a `EntityIndex` into an `ExternType`.
    pub(crate) fn new(
        entity_index: &EntityIndex,
        module: &'module wasmtime_environ::Module,
    ) -> EntityType<'module> {
        match entity_index {
            EntityIndex::Function(func_index) => {
                let sig = module.local.wasm_func_type(*func_index);
                EntityType::Function(&sig)
            }
            EntityIndex::Table(table_index) => {
                EntityType::Table(&module.local.table_plans[*table_index].table)
            }
            EntityIndex::Memory(memory_index) => {
                EntityType::Memory(&module.local.memory_plans[*memory_index].memory)
            }
            EntityIndex::Global(global_index) => {
                EntityType::Global(&module.local.globals[*global_index])
            }
        }
    }

    /// Convert this `EntityType` to an `ExternType`.
    pub(crate) fn extern_type(&self) -> ExternType {
        match self {
            EntityType::Function(sig) => FuncType::from_wasm_func_type(sig)
                .expect("core wasm function type should be supported")
                .into(),
            EntityType::Table(table) => TableType::from_wasmtime_table(table).into(),
            EntityType::Memory(memory) => MemoryType::from_wasmtime_memory(memory).into(),
            EntityType::Global(global) => GlobalType::from_wasmtime_global(global)
                .expect("core wasm global type should be supported")
                .into(),
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
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct ImportType<'module> {
    /// The module of the import.
    module: &'module str,

    /// The field of the import.
    name: &'module str,

    /// The type of the import.
    ty: EntityType<'module>,
}

impl<'module> ImportType<'module> {
    /// Creates a new import descriptor which comes from `module` and `name` and
    /// is of type `ty`.
    pub(crate) fn new(
        module: &'module str,
        name: &'module str,
        ty: EntityType<'module>,
    ) -> ImportType<'module> {
        ImportType { module, name, ty }
    }

    /// Returns the module name that this import is expected to come from.
    pub fn module(&self) -> &'module str {
        self.module
    }

    /// Returns the field name of the module that this import is expected to
    /// come from.
    pub fn name(&self) -> &'module str {
        self.name
    }

    /// Returns the expected type of this import.
    pub fn ty(&self) -> ExternType {
        self.ty.extern_type()
    }
}

impl<'module> fmt::Debug for ImportType<'module> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImportType")
            .field("module", &self.module().to_owned())
            .field("name", &self.name().to_owned())
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
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct ExportType<'module> {
    /// The name of the export.
    name: &'module str,

    /// The type of the export.
    ty: EntityType<'module>,
}

impl<'module> ExportType<'module> {
    /// Creates a new export which is exported with the given `name` and has the
    /// given `ty`.
    pub(crate) fn new(name: &'module str, ty: EntityType<'module>) -> ExportType<'module> {
        ExportType { name, ty }
    }

    /// Returns the name by which this export is known.
    pub fn name(&self) -> &'module str {
        self.name
    }

    /// Returns the type of this export.
    pub fn ty(&self) -> ExternType {
        self.ty.extern_type()
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
