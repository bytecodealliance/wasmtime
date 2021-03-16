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

impl FuncId {
    /// Get the `FuncId` for the function named by `name`.
    pub fn from_name(name: &ir::ExternalName) -> FuncId {
        if let ir::ExternalName::User { namespace, index } = *name {
            debug_assert_eq!(namespace, 0);
            FuncId::from_u32(index)
        } else {
            panic!("unexpected ExternalName kind {}", name)
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

impl DataId {
    /// Get the `DataId` for the data object named by `name`.
    pub fn from_name(name: &ir::ExternalName) -> DataId {
        if let ir::ExternalName::User { namespace, index } = *name {
            debug_assert_eq!(namespace, 1);
            DataId::from_u32(index)
        } else {
            panic!("unexpected ExternalName kind {}", name)
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
#[derive(Debug)]
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
#[derive(Debug)]
pub enum ModuleError {
    /// Indicates an identifier was used before it was declared
    Undeclared(String),

    /// Indicates an identifier was used as data/function first, but then used as the other
    IncompatibleDeclaration(String),

    /// Indicates a function identifier was declared with a
    /// different signature than declared previously
    IncompatibleSignature(String, ir::Signature, ir::Signature),

    /// Indicates an identifier was defined more than once
    DuplicateDefinition(String),

    /// Indicates an identifier was defined, but was declared as an import
    InvalidImportDefinition(String),

    /// Wraps a `cranelift-codegen` error
    Compilation(CodegenError),

    /// Wraps a generic error from a backend
    Backend(anyhow::Error),
}

impl std::error::Error for ModuleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Undeclared { .. }
            | Self::IncompatibleDeclaration { .. }
            | Self::IncompatibleSignature { .. }
            | Self::DuplicateDefinition { .. }
            | Self::InvalidImportDefinition { .. } => None,
            Self::Compilation(source) => Some(source),
            Self::Backend(source) => Some(&**source),
        }
    }
}

impl std::fmt::Display for ModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Undeclared(name) => {
                write!(f, "Undeclared identifier: {}", name)
            }
            Self::IncompatibleDeclaration(name) => {
                write!(f, "Incompatible declaration of identifier: {}", name,)
            }
            Self::IncompatibleSignature(name, prev_sig, new_sig) => {
                write!(
                    f,
                    "Function {} signature {:?} is incompatible with previous declaration {:?}",
                    name, new_sig, prev_sig,
                )
            }
            Self::DuplicateDefinition(name) => {
                write!(f, "Duplicate definition of identifier: {}", name)
            }
            Self::InvalidImportDefinition(name) => {
                write!(
                    f,
                    "Invalid to define identifier declared as an import: {}",
                    name,
                )
            }
            Self::Compilation(err) => {
                write!(f, "Compilation error: {}", err)
            }
            Self::Backend(err) => write!(f, "Backend error: {}", err),
        }
    }
}

impl std::convert::From<CodegenError> for ModuleError {
    fn from(source: CodegenError) -> Self {
        Self::Compilation { 0: source }
    }
}

/// A convenient alias for a `Result` that uses `ModuleError` as the error type.
pub type ModuleResult<T> = Result<T, ModuleError>;

/// Information about a data object which can be accessed.
#[derive(Debug)]
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
#[derive(Debug, Default)]
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

    /// Return whether `name` names a function, rather than a data object.
    pub fn is_function(name: &ir::ExternalName) -> bool {
        if let ir::ExternalName::User { namespace, .. } = *name {
            namespace == 0
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

    /// Declare a function in this module.
    pub fn declare_function(
        &mut self,
        name: &str,
        linkage: Linkage,
        signature: &ir::Signature,
    ) -> ModuleResult<(FuncId, Linkage)> {
        // TODO: Can we avoid allocating names so often?
        use super::hash_map::Entry::*;
        match self.names.entry(name.to_owned()) {
            Occupied(entry) => match *entry.get() {
                FuncOrDataId::Func(id) => {
                    let existing = &mut self.functions[id];
                    existing.merge(linkage, signature)?;
                    Ok((id, existing.linkage))
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
                Ok((id, self.functions[id].linkage))
            }
        }
    }

    /// Declare an anonymous function in this module.
    pub fn declare_anonymous_function(
        &mut self,
        signature: &ir::Signature,
    ) -> ModuleResult<FuncId> {
        let id = self.functions.push(FunctionDeclaration {
            name: String::new(),
            linkage: Linkage::Local,
            signature: signature.clone(),
        });
        self.functions[id].name = format!(".L{:?}", id);
        Ok(id)
    }

    /// Declare a data object in this module.
    pub fn declare_data(
        &mut self,
        name: &str,
        linkage: Linkage,
        writable: bool,
        tls: bool,
    ) -> ModuleResult<(DataId, Linkage)> {
        // TODO: Can we avoid allocating names so often?
        use super::hash_map::Entry::*;
        match self.names.entry(name.to_owned()) {
            Occupied(entry) => match *entry.get() {
                FuncOrDataId::Data(id) => {
                    let existing = &mut self.data_objects[id];
                    existing.merge(linkage, writable, tls);
                    Ok((id, existing.linkage))
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
                Ok((id, self.data_objects[id].linkage))
            }
        }
    }

    /// Declare an anonymous data object in this module.
    pub fn declare_anonymous_data(&mut self, writable: bool, tls: bool) -> ModuleResult<DataId> {
        let id = self.data_objects.push(DataDeclaration {
            name: String::new(),
            linkage: Linkage::Local,
            writable,
            tls,
        });
        self.data_objects[id].name = format!(".L{:?}", id);
        Ok(id)
    }
}

/// Information about the compiled function.
pub struct ModuleCompiledFunction {
    /// The size of the compiled function.
    pub size: binemit::CodeOffset,
}

/// A record of a relocation to perform.
#[derive(Clone)]
pub struct RelocRecord {
    /// Where in the generated code this relocation is to be applied.
    pub offset: binemit::CodeOffset,
    /// The kind of relocation this represents.
    pub reloc: binemit::Reloc,
    /// What symbol we're relocating against.
    pub name: ir::ExternalName,
    /// The offset to add to the relocation.
    pub addend: binemit::Addend,
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

    /// Declare an anonymous function in this module.
    fn declare_anonymous_function(&mut self, signature: &ir::Signature) -> ModuleResult<FuncId>;

    /// Declare a data object in this module.
    fn declare_data(
        &mut self,
        name: &str,
        linkage: Linkage,
        writable: bool,
        tls: bool,
    ) -> ModuleResult<DataId>;

    /// Declare an anonymous data object in this module.
    fn declare_anonymous_data(&mut self, writable: bool, tls: bool) -> ModuleResult<DataId>;

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
    fn define_function(
        &mut self,
        func: FuncId,
        ctx: &mut Context,
        trap_sink: &mut dyn binemit::TrapSink,
        stack_map_sink: &mut dyn binemit::StackMapSink,
    ) -> ModuleResult<ModuleCompiledFunction>;

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
        relocs: &[RelocRecord],
    ) -> ModuleResult<ModuleCompiledFunction>;

    /// Define a data object, producing the data contents from the given `DataContext`.
    fn define_data(&mut self, data: DataId, data_ctx: &DataContext) -> ModuleResult<()>;
}

impl<M: Module> Module for &mut M {
    fn isa(&self) -> &dyn isa::TargetIsa {
        (**self).isa()
    }

    fn declarations(&self) -> &ModuleDeclarations {
        (**self).declarations()
    }

    fn get_name(&self, name: &str) -> Option<FuncOrDataId> {
        (**self).get_name(name)
    }

    fn target_config(&self) -> isa::TargetFrontendConfig {
        (**self).target_config()
    }

    fn make_context(&self) -> Context {
        (**self).make_context()
    }

    fn clear_context(&self, ctx: &mut Context) {
        (**self).clear_context(ctx)
    }

    fn make_signature(&self) -> ir::Signature {
        (**self).make_signature()
    }

    fn clear_signature(&self, sig: &mut ir::Signature) {
        (**self).clear_signature(sig)
    }

    fn declare_function(
        &mut self,
        name: &str,
        linkage: Linkage,
        signature: &ir::Signature,
    ) -> ModuleResult<FuncId> {
        (**self).declare_function(name, linkage, signature)
    }

    fn declare_anonymous_function(&mut self, signature: &ir::Signature) -> ModuleResult<FuncId> {
        (**self).declare_anonymous_function(signature)
    }

    fn declare_data(
        &mut self,
        name: &str,
        linkage: Linkage,
        writable: bool,
        tls: bool,
    ) -> ModuleResult<DataId> {
        (**self).declare_data(name, linkage, writable, tls)
    }

    fn declare_anonymous_data(&mut self, writable: bool, tls: bool) -> ModuleResult<DataId> {
        (**self).declare_anonymous_data(writable, tls)
    }

    fn declare_func_in_func(&self, func: FuncId, in_func: &mut ir::Function) -> ir::FuncRef {
        (**self).declare_func_in_func(func, in_func)
    }

    fn declare_data_in_func(&self, data: DataId, func: &mut ir::Function) -> ir::GlobalValue {
        (**self).declare_data_in_func(data, func)
    }

    fn declare_func_in_data(&self, func: FuncId, ctx: &mut DataContext) -> ir::FuncRef {
        (**self).declare_func_in_data(func, ctx)
    }

    fn declare_data_in_data(&self, data: DataId, ctx: &mut DataContext) -> ir::GlobalValue {
        (**self).declare_data_in_data(data, ctx)
    }

    fn define_function(
        &mut self,
        func: FuncId,
        ctx: &mut Context,
        trap_sink: &mut dyn binemit::TrapSink,
        stack_map_sink: &mut dyn binemit::StackMapSink,
    ) -> ModuleResult<ModuleCompiledFunction> {
        (**self).define_function(func, ctx, trap_sink, stack_map_sink)
    }

    fn define_function_bytes(
        &mut self,
        func: FuncId,
        bytes: &[u8],
        relocs: &[RelocRecord],
    ) -> ModuleResult<ModuleCompiledFunction> {
        (**self).define_function_bytes(func, bytes, relocs)
    }

    fn define_data(&mut self, data: DataId, data_ctx: &DataContext) -> ModuleResult<()> {
        (**self).define_data(data, data_ctx)
    }
}
