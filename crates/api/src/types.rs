use wasmtime_environ::{ir, wasm};

// Type Representations

// Type attributes

/// Indicator of whether a global is mutable or not
#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone)]
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

    /// Returs the maximum amount for these limits, if specified.
    pub fn max(&self) -> Option<u32> {
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

    pub(crate) fn get_wasmtime_type(&self) -> ir::Type {
        match self {
            ValType::I32 => ir::types::I32,
            ValType::I64 => ir::types::I64,
            ValType::F32 => ir::types::F32,
            ValType::F64 => ir::types::F64,
            ValType::V128 => ir::types::I8X16,
            _ => unimplemented!("get_wasmtime_type other"),
        }
    }

    pub(crate) fn from_wasmtime_type(ty: ir::Type) -> ValType {
        match ty {
            ir::types::I32 => ValType::I32,
            ir::types::I64 => ValType::I64,
            ir::types::F32 => ValType::F32,
            ir::types::F64 => ValType::F64,
            ir::types::I8X16 => ValType::V128,
            _ => unimplemented!("from_wasmtime_type other"),
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
    Func(FuncType),
    Global(GlobalType),
    Table(TableType),
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
    pub(crate) fn from_wasmtime_export(export: &wasmtime_runtime::Export) -> Self {
        match export {
            wasmtime_runtime::Export::Function { signature, .. } => {
                ExternType::Func(FuncType::from_wasmtime_signature(signature.clone()))
            }
            wasmtime_runtime::Export::Memory { memory, .. } => {
                ExternType::Memory(MemoryType::from_wasmtime_memory(&memory.memory))
            }
            wasmtime_runtime::Export::Global { global, .. } => {
                ExternType::Global(GlobalType::from_wasmtime_global(&global))
            }
            wasmtime_runtime::Export::Table { table, .. } => {
                ExternType::Table(TableType::from_wasmtime_table(&table.table))
            }
        }
    }
}

// Function Types
fn from_wasmtime_abiparam(param: &ir::AbiParam) -> ValType {
    assert_eq!(param.purpose, ir::ArgumentPurpose::Normal);
    ValType::from_wasmtime_type(param.value_type)
}

/// A descriptor for a function in a WebAssembly module.
///
/// WebAssembly functions can have 0 or more parameters and results.
#[derive(Debug, Clone)]
pub struct FuncType {
    params: Box<[ValType]>,
    results: Box<[ValType]>,
    signature: ir::Signature,
}

impl FuncType {
    /// Creates a new function descriptor from the given parameters and results.
    ///
    /// The function descriptor returned will represent a function which takes
    /// `params` as arguments and returns `results` when it is finished.
    pub fn new(params: Box<[ValType]>, results: Box<[ValType]>) -> FuncType {
        use wasmtime_environ::ir::{types, AbiParam, ArgumentPurpose, Signature};
        use wasmtime_jit::native;
        let call_conv = native::call_conv();
        let signature: Signature = {
            let mut params = params
                .iter()
                .map(|p| AbiParam::new(p.get_wasmtime_type()))
                .collect::<Vec<_>>();
            let returns = results
                .iter()
                .map(|p| AbiParam::new(p.get_wasmtime_type()))
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

    /// Returns the list of parameter types for this function.
    pub fn params(&self) -> &[ValType] {
        &self.params
    }

    /// Returns the list of result types for this function.
    pub fn results(&self) -> &[ValType] {
        &self.results
    }

    pub(crate) fn get_wasmtime_signature(&self) -> &ir::Signature {
        &self.signature
    }

    pub(crate) fn from_wasmtime_signature(signature: ir::Signature) -> FuncType {
        let params = signature
            .params
            .iter()
            .filter(|p| p.purpose == ir::ArgumentPurpose::Normal)
            .map(|p| from_wasmtime_abiparam(p))
            .collect::<Vec<_>>();
        let results = signature
            .returns
            .iter()
            .map(|p| from_wasmtime_abiparam(p))
            .collect::<Vec<_>>();
        FuncType {
            params: params.into_boxed_slice(),
            results: results.into_boxed_slice(),
            signature,
        }
    }
}

// Global Types

/// A WebAssembly global descriptor.
///
/// This type describes an instance of a global in a WebAssembly module. Globals
/// are local to an [`Instance`](crate::Instance) and are either immutable or
/// mutable.
#[derive(Debug, Clone)]
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

    pub(crate) fn from_wasmtime_global(global: &wasm::Global) -> GlobalType {
        let ty = ValType::from_wasmtime_type(global.ty);
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
/// an `anyref`. The most common use for tables is a function table through
/// which `call_indirect` can invoke other functions.
#[derive(Debug, Clone)]
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
        assert!(if let wasm::TableElementType::Func = table.ty {
            true
        } else {
            false
        });
        let ty = ValType::FuncRef;
        let limits = Limits::new(table.minimum, table.maximum);
        TableType::new(ty, limits)
    }
}

// Memory Types

/// A descriptor for a WebAssembly memory type.
///
/// Memories are described in units of pages (64KB) and represent contiguous
/// chunks of addressable memory.
#[derive(Debug, Clone)]
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

// Import Types

/// A descriptor for an imported value into a wasm module.
///
/// This type is primarily accessed from the
/// [`Module::imports`](crate::Module::imports) API. Each [`ImportType`]
/// describes an import into the wasm module with the module/name that it's
/// imported from as well as the type of item that's being imported.
#[derive(Debug, Clone)]
pub struct ImportType {
    module: String,
    name: String,
    ty: ExternType,
}

impl ImportType {
    /// Creates a new import descriptor which comes from `module` and `name` and
    /// is of type `ty`.
    pub fn new(module: &str, name: &str, ty: ExternType) -> ImportType {
        ImportType {
            module: module.to_string(),
            name: name.to_string(),
            ty,
        }
    }

    /// Returns the module name that this import is expected to come from.
    pub fn module(&self) -> &str {
        &self.module
    }

    /// Returns the field name of the module that this import is expected to
    /// come from.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the expected type of this import.
    pub fn ty(&self) -> &ExternType {
        &self.ty
    }
}

// Export Types

/// A descriptor for an exported WebAssembly value.
///
/// This type is primarily accessed from the
/// [`Module::exports`](crate::Module::exports) accessor and describes what
/// names are exported from a wasm module and the type of the item that is
/// exported.
#[derive(Debug, Clone)]
pub struct ExportType {
    name: String,
    ty: ExternType,
}

impl ExportType {
    /// Creates a new export which is exported with the given `name` and has the
    /// given `ty`.
    pub fn new(name: &str, ty: ExternType) -> ExportType {
        ExportType {
            name: name.to_string(),
            ty,
        }
    }

    /// Returns the name by which this export is known by.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the type of this export.
    pub fn ty(&self) -> &ExternType {
        &self.ty
    }
}
