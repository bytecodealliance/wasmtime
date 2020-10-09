//! Defines `Module` and related types.

// TODO: Should `ir::Function` really have a `name`?

// TODO: Factor out `ir::Function`'s `ext_funcs` and `global_values` into a struct
// shared with `DataContext`?

use super::HashMap;
use crate::data_context::DataContext;
use cranelift_codegen::binemit;
use cranelift_codegen::entity::{entity_impl, PrimaryMap};
use cranelift_codegen::{ir, isa, CodegenError, Context};
use std::borrow::ToOwned;
use std::string::String;
use thiserror::Error;

/// A function identifier for use in the `Module` interface.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FuncId(u32);
entity_impl!(FuncId, "funcid");

/// Function identifiers are namespace 0 in `ir::ExternalName`
impl From<FuncId> for ir::ExternalName {
    fn from(id: FuncId) -> Self {
        Self::User {
            namespace: 0,
            index: id.0,
        }
    }
}

/// A data object identifier for use in the `Module` interface.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DataId(u32);
entity_impl!(DataId, "dataid");

/// Data identifiers are namespace 1 in `ir::ExternalName`
impl From<DataId> for ir::ExternalName {
    fn from(id: DataId) -> Self {
        Self::User {
            namespace: 1,
            index: id.0,
        }
    }
}

/// Linkage refers to where an entity is defined and who can see it.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Linkage {
    /// Defined outside of a module.
    Import,
    /// Defined inside the module, but not visible outside it.
    Local,
    /// Defined inside the module, visible outside it, and may be preempted.
    Preemptible,
    /// Defined inside the module, visible inside the current static linkage unit, but not outside.
    ///
    /// A static linkage unit is the combination of all object files passed to a linker to create
    /// an executable or dynamic library.
    Hidden,
    /// Defined inside the module, and visible outside it.
    Export,
}

impl Linkage {
    fn merge(a: Self, b: Self) -> Self {
        match a {
            Self::Export => Self::Export,
            Self::Hidden => match b {
                Self::Export => Self::Export,
                Self::Preemptible => Self::Preemptible,
                _ => Self::Hidden,
            },
            Self::Preemptible => match b {
                Self::Export => Self::Export,
                _ => Self::Preemptible,
            },
            Self::Local => match b {
                Self::Export => Self::Export,
                Self::Hidden => Self::Hidden,
                Self::Preemptible => Self::Preemptible,
                Self::Local | Self::Import => Self::Local,
            },
            Self::Import => b,
        }
    }

    /// Test whether this linkage can have a definition.
    pub fn is_definable(self) -> bool {
        match self {
            Self::Import => false,
            Self::Local | Self::Preemptible | Self::Hidden | Self::Export => true,
        }
    }

    /// Test whether this linkage will have a definition that cannot be preempted.
    pub fn is_final(self) -> bool {
        match self {
            Self::Import | Self::Preemptible => false,
            Self::Local | Self::Hidden | Self::Export => true,
        }
    }
}

/// A declared name may refer to either a function or data declaration
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum FuncOrDataId {
    /// When it's a FuncId
    Func(FuncId),
    /// When it's a DataId
    Data(DataId),
}

/// Mapping to `ir::ExternalName` is trivial based on the `FuncId` and `DataId` mapping.
impl From<FuncOrDataId> for ir::ExternalName {
    fn from(id: FuncOrDataId) -> Self {
        match id {
            FuncOrDataId::Func(funcid) => Self::from(funcid),
            FuncOrDataId::Data(dataid) => Self::from(dataid),
        }
    }
}

/// Information about a function which can be called.
pub struct FunctionDeclaration {
    pub name: String,
    pub linkage: Linkage,
    pub signature: ir::Signature,
}

impl FunctionDeclaration {
    fn merge(&mut self, linkage: Linkage, sig: &ir::Signature) -> Result<(), ModuleError> {
        self.linkage = Linkage::merge(self.linkage, linkage);
        if &self.signature != sig {
            return Err(ModuleError::IncompatibleSignature(
                self.name.clone(),
                self.signature.clone(),
                sig.clone(),
            ));
        }
        Ok(())
    }
}

/// Error messages for all `Module` methods
#[derive(Error, Debug)]
pub enum ModuleError {
    /// Indicates an identifier was used before it was declared
    #[error("Undeclared identifier: {0}")]
    Undeclared(String),
    /// Indicates an identifier was used as data/function first, but then used as the other
    #[error("Incompatible declaration of identifier: {0}")]
    IncompatibleDeclaration(String),
    /// Indicates a function identifier was declared with a
    /// different signature than declared previously
    #[error("Function {0} signature {2:?} is incompatible with previous declaration {1:?}")]
    IncompatibleSignature(String, ir::Signature, ir::Signature),
    /// Indicates an identifier was defined more than once
    #[error("Duplicate definition of identifier: {0}")]
    DuplicateDefinition(String),
    /// Indicates an identifier was defined, but was declared as an import
    #[error("Invalid to define identifier declared as an import: {0}")]
    InvalidImportDefinition(String),
    /// Indicates a too-long function was defined
    #[error("Function {0} exceeds the maximum function size")]
    FunctionTooLarge(String),
    /// Wraps a `cranelift-codegen` error
    #[error("Compilation error: {0}")]
    Compilation(#[from] CodegenError),
    /// Wraps a generic error from a backend
    #[error("Backend error: {0}")]
    Backend(#[source] anyhow::Error),
}

/// A convenient alias for a `Result` that uses `ModuleError` as the error type.
pub type ModuleResult<T> = Result<T, ModuleError>;

/// Information about a data object which can be accessed.
pub struct DataDeclaration {
    pub name: String,
    pub linkage: Linkage,
    pub writable: bool,
    pub tls: bool,
}

impl DataDeclaration {
    fn merge(&mut self, linkage: Linkage, writable: bool, tls: bool) {
        self.linkage = Linkage::merge(self.linkage, linkage);
        self.writable = self.writable || writable;
        assert_eq!(
            self.tls, tls,
            "Can't change TLS data object to normal or in the opposite way",
        );
    }
}

/// This provides a view to the state of a module which allows `ir::ExternalName`s to be translated
/// into `FunctionDeclaration`s and `DataDeclaration`s.
#[derive(Default)]
pub struct ModuleDeclarations {
    names: HashMap<String, FuncOrDataId>,
    functions: PrimaryMap<FuncId, FunctionDeclaration>,
    data_objects: PrimaryMap<DataId, DataDeclaration>,
}

impl ModuleDeclarations {
    /// Get the module identifier for a given name, if that name
    /// has been declared.
    pub fn get_name(&self, name: &str) -> Option<FuncOrDataId> {
        self.names.get(name).copied()
    }

    /// Get an iterator of all function declarations
    pub fn get_functions(&self) -> impl Iterator<Item = (FuncId, &FunctionDeclaration)> {
        self.functions.iter()
    }

    /// Get the `FuncId` for the function named by `name`.
    pub fn get_function_id(&self, name: &ir::ExternalName) -> FuncId {
        if let ir::ExternalName::User { namespace, index } = *name {
            debug_assert_eq!(namespace, 0);
            FuncId::from_u32(index)
        } else {
            panic!("unexpected ExternalName kind {}", name)
        }
    }

    /// Get the `DataId` for the data object named by `name`.
    pub fn get_data_id(&self, name: &ir::ExternalName) -> DataId {
        if let ir::ExternalName::User { namespace, index } = *name {
            debug_assert_eq!(namespace, 1);
            DataId::from_u32(index)
        } else {
            panic!("unexpected ExternalName kind {}", name)
        }
    }

    /// Get the `FunctionDeclaration` for the function named by `name`.
    pub fn get_function_decl(&self, func_id: FuncId) -> &FunctionDeclaration {
        &self.functions[func_id]
    }

    /// Get an iterator of all data declarations
    pub fn get_data_objects(&self) -> impl Iterator<Item = (DataId, &DataDeclaration)> {
        self.data_objects.iter()
    }

    /// Get the `DataDeclaration` for the data object named by `name`.
    pub fn get_data_decl(&self, data_id: DataId) -> &DataDeclaration {
        &self.data_objects[data_id]
    }

    /// Return whether `name` names a function, rather than a data object.
    pub fn is_function(&self, name: &ir::ExternalName) -> bool {
        if let ir::ExternalName::User { namespace, .. } = *name {
            namespace == 0
        } else {
            panic!("unexpected ExternalName kind {}", name)
        }
    }

    /// Declare a function in this module.
    pub fn declare_function(
        &mut self,
        name: &str,
        linkage: Linkage,
        signature: &ir::Signature,
    ) -> ModuleResult<(FuncId, &FunctionDeclaration)> {
        // TODO: Can we avoid allocating names so often?
        use super::hash_map::Entry::*;
        match self.names.entry(name.to_owned()) {
            Occupied(entry) => match *entry.get() {
                FuncOrDataId::Func(id) => {
                    let existing = &mut self.functions[id];
                    existing.merge(linkage, signature)?;
                    Ok((id, existing))
                }
                FuncOrDataId::Data(..) => {
                    Err(ModuleError::IncompatibleDeclaration(name.to_owned()))
                }
            },
            Vacant(entry) => {
                let id = self.functions.push(FunctionDeclaration {
                    name: name.to_owned(),
                    linkage,
                    signature: signature.clone(),
                });
                entry.insert(FuncOrDataId::Func(id));
                Ok((id, &self.functions[id]))
            }
        }
    }

    /// Declare a data object in this module.
    pub fn declare_data(
        &mut self,
        name: &str,
        linkage: Linkage,
        writable: bool,
        tls: bool,
    ) -> ModuleResult<(DataId, &DataDeclaration)> {
        // TODO: Can we avoid allocating names so often?
        use super::hash_map::Entry::*;
        match self.names.entry(name.to_owned()) {
            Occupied(entry) => match *entry.get() {
                FuncOrDataId::Data(id) => {
                    let existing = &mut self.data_objects[id];
                    existing.merge(linkage, writable, tls);
                    Ok((id, existing))
                }

                FuncOrDataId::Func(..) => {
                    Err(ModuleError::IncompatibleDeclaration(name.to_owned()))
                }
            },
            Vacant(entry) => {
                let id = self.data_objects.push(DataDeclaration {
                    name: name.to_owned(),
                    linkage,
                    writable,
                    tls,
                });
                entry.insert(FuncOrDataId::Data(id));
                Ok((id, &self.data_objects[id]))
            }
        }
    }
}

/// Information about the compiled function.
pub struct ModuleCompiledFunction {
    /// The size of the compiled function.
    pub size: binemit::CodeOffset,
}

/// A `Module` is a utility for collecting functions and data objects, and linking them together.
pub trait Module {
    /// Return the `TargetIsa` to compile for.
    fn isa(&self) -> &dyn isa::TargetIsa;

    /// Get all declarations in this module.
    fn declarations(&self) -> &ModuleDeclarations;

    /// Get the module identifier for a given name, if that name
    /// has been declared.
    fn get_name(&self, name: &str) -> Option<FuncOrDataId> {
        self.declarations().get_name(name)
    }

    /// Return the target information needed by frontends to produce Cranelift IR
    /// for the current target.
    fn target_config(&self) -> isa::TargetFrontendConfig {
        self.isa().frontend_config()
    }

    /// Create a new `Context` initialized for use with this `Module`.
    ///
    /// This ensures that the `Context` is initialized with the default calling
    /// convention for the `TargetIsa`.
    fn make_context(&self) -> Context {
        let mut ctx = Context::new();
        ctx.func.signature.call_conv = self.isa().default_call_conv();
        ctx
    }

    /// Clear the given `Context` and reset it for use with a new function.
    ///
    /// This ensures that the `Context` is initialized with the default calling
    /// convention for the `TargetIsa`.
    fn clear_context(&self, ctx: &mut Context) {
        ctx.clear();
        ctx.func.signature.call_conv = self.isa().default_call_conv();
    }

    /// Create a new empty `Signature` with the default calling convention for
    /// the `TargetIsa`, to which parameter and return types can be added for
    /// declaring a function to be called by this `Module`.
    fn make_signature(&self) -> ir::Signature {
        ir::Signature::new(self.isa().default_call_conv())
    }

    /// Clear the given `Signature` and reset for use with a new function.
    ///
    /// This ensures that the `Signature` is initialized with the default
    /// calling convention for the `TargetIsa`.
    fn clear_signature(&self, sig: &mut ir::Signature) {
        sig.clear(self.isa().default_call_conv());
    }

    /// Declare a function in this module.
    fn declare_function(
        &mut self,
        name: &str,
        linkage: Linkage,
        signature: &ir::Signature,
    ) -> ModuleResult<FuncId>;

    /// Declare a data object in this module.
    fn declare_data(
        &mut self,
        name: &str,
        linkage: Linkage,
        writable: bool,
        tls: bool,
    ) -> ModuleResult<DataId>;

    /// Use this when you're building the IR of a function to reference a function.
    ///
    /// TODO: Coalesce redundant decls and signatures.
    /// TODO: Look into ways to reduce the risk of using a FuncRef in the wrong function.
    fn declare_func_in_func(&self, func: FuncId, in_func: &mut ir::Function) -> ir::FuncRef {
        let decl = &self.declarations().functions[func];
        let signature = in_func.import_signature(decl.signature.clone());
        let colocated = decl.linkage.is_final();
        in_func.import_function(ir::ExtFuncData {
            name: ir::ExternalName::user(0, func.as_u32()),
            signature,
            colocated,
        })
    }

    /// Use this when you're building the IR of a function to reference a data object.
    ///
    /// TODO: Same as above.
    fn declare_data_in_func(&self, data: DataId, func: &mut ir::Function) -> ir::GlobalValue {
        let decl = &self.declarations().data_objects[data];
        let colocated = decl.linkage.is_final();
        func.create_global_value(ir::GlobalValueData::Symbol {
            name: ir::ExternalName::user(1, data.as_u32()),
            offset: ir::immediates::Imm64::new(0),
            colocated,
            tls: decl.tls,
        })
    }

    /// TODO: Same as above.
    fn declare_func_in_data(&self, func: FuncId, ctx: &mut DataContext) -> ir::FuncRef {
        ctx.import_function(ir::ExternalName::user(0, func.as_u32()))
    }

    /// TODO: Same as above.
    fn declare_data_in_data(&self, data: DataId, ctx: &mut DataContext) -> ir::GlobalValue {
        ctx.import_global_value(ir::ExternalName::user(1, data.as_u32()))
    }

    /// Define a function, producing the function body from the given `Context`.
    ///
    /// Returns the size of the function's code and constant data.
    ///
    /// Note: After calling this function the given `Context` will contain the compiled function.
    fn define_function<TS>(
        &mut self,
        func: FuncId,
        ctx: &mut Context,
        trap_sink: &mut TS,
    ) -> ModuleResult<ModuleCompiledFunction>
    where
        TS: binemit::TrapSink;

    /// Define a function, taking the function body from the given `bytes`.
    ///
    /// This function is generally only useful if you need to precisely specify
    /// the emitted instructions for some reason; otherwise, you should use
    /// `define_function`.
    ///
    /// Returns the size of the function's code.
    fn define_function_bytes(
        &mut self,
        func: FuncId,
        bytes: &[u8],
    ) -> ModuleResult<ModuleCompiledFunction>;

    /// Define a data object, producing the data contents from the given `DataContext`.
    fn define_data(&mut self, data: DataId, data_ctx: &DataContext) -> ModuleResult<()>;
}
