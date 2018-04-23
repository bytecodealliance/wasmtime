//! Defines `Module` and related types.

// TODO: Should `ir::Function` really have a `name`?

// TODO: Factor out `ir::Function`'s `ext_funcs` and `global_vars` into a struct
// shared with `DataContext`?

use Backend;
use cretonne_codegen::entity::{EntityRef, PrimaryMap};
use cretonne_codegen::result::{CtonError, CtonResult};
use cretonne_codegen::{binemit, ir, Context};
use data_context::DataContext;
use std::collections::HashMap;

/// A function identifier for use in the `Module` interface.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FuncId(u32);
entity_impl!(FuncId, "funcid");

/// A data object identifier for use in the `Module` interface.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DataId(u32);
entity_impl!(DataId, "dataid");

/// Linkage refers to where an entity is defined and who can see it.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Linkage {
    /// Defined outside of a module.
    Import,
    /// Defined inside the module, but not visible outside it.
    Local,
    /// Defined inside the module, visible outside it, and may be preempted.
    Preemptible,
    /// Defined inside the module, and visible outside it.
    Export,
}

impl Linkage {
    fn merge(a: Self, b: Self) -> Self {
        match a {
            Linkage::Export => Linkage::Export,
            Linkage::Preemptible => {
                match b {
                    Linkage::Export => Linkage::Export,
                    _ => Linkage::Preemptible,
                }
            }
            Linkage::Local => {
                match b {
                    Linkage::Export => Linkage::Export,
                    Linkage::Preemptible => Linkage::Preemptible,
                    _ => Linkage::Local,
                }
            }
            Linkage::Import => b,
        }
    }

    /// Test whether this linkage can have a definition.
    pub fn is_definable(&self) -> bool {
        match *self {
            Linkage::Import => false,
            Linkage::Local | Linkage::Preemptible | Linkage::Export => true,
        }
    }

    /// Test whether this linkage will have a definition that cannot be preempted.
    pub fn is_final(&self) -> bool {
        match *self {
            Linkage::Import | Linkage::Preemptible => false,
            Linkage::Local | Linkage::Export => true,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum FuncOrDataId {
    Func(FuncId),
    Data(DataId),
}

/// Information about a function which can be called.
pub struct FunctionDeclaration {
    pub name: String,
    pub linkage: Linkage,
    pub signature: ir::Signature,
}

/// A function belonging to a `Module`.
struct ModuleFunction<B>
where
    B: Backend,
{
    /// The function declaration.
    decl: FunctionDeclaration,
    /// The compiled artifact, once it's available.
    compiled: Option<B::CompiledFunction>,
    /// A flag indicating whether the function has been finalized.
    finalized: bool,
}

impl<B> ModuleFunction<B>
where
    B: Backend,
{
    fn merge(&mut self, linkage: Linkage) {
        self.decl.linkage = Linkage::merge(self.decl.linkage, linkage);
    }
}

/// Information about a data object which can be accessed.
pub struct DataDeclaration {
    pub name: String,
    pub linkage: Linkage,
    pub writable: bool,
}

/// A data object belonging to a `Module`.
struct ModuleData<B>
where
    B: Backend,
{
    /// The data object declaration.
    decl: DataDeclaration,
    /// The "compiled" artifact, once it's available.
    compiled: Option<B::CompiledData>,
    /// A flag indicating whether the data object has been finalized.
    finalized: bool,
}

impl<B> ModuleData<B>
where
    B: Backend,
{
    fn merge(&mut self, linkage: Linkage, writable: bool) {
        self.decl.linkage = Linkage::merge(self.decl.linkage, linkage);
        self.decl.writable = self.decl.writable || writable;
    }
}

/// The functions and data objects belonging to a module.
struct ModuleContents<B>
where
    B: Backend,
{
    functions: PrimaryMap<FuncId, ModuleFunction<B>>,
    data_objects: PrimaryMap<DataId, ModuleData<B>>,
}

impl<B> ModuleContents<B>
where
    B: Backend,
{
    fn get_function_info(&self, name: &ir::ExternalName) -> &ModuleFunction<B> {
        if let ir::ExternalName::User { namespace, index } = *name {
            debug_assert_eq!(namespace, 0);
            let func = FuncId::new(index as usize);
            &self.functions[func]
        } else {
            panic!("unexpected ExternalName kind")
        }
    }

    /// Get the `DataDeclaration` for the function named by `name`.
    fn get_data_info(&self, name: &ir::ExternalName) -> &ModuleData<B> {
        if let ir::ExternalName::User { namespace, index } = *name {
            debug_assert_eq!(namespace, 1);
            let data = DataId::new(index as usize);
            &self.data_objects[data]
        } else {
            panic!("unexpected ExternalName kind")
        }
    }
}

/// This provides a view to the state of a module which allows `ir::ExternalName`s to be translated
/// into `FunctionDeclaration`s and `DataDeclaration`s.
pub struct ModuleNamespace<'a, B: 'a>
where
    B: Backend,
{
    contents: &'a ModuleContents<B>,
}

impl<'a, B> ModuleNamespace<'a, B>
where
    B: Backend,
{
    /// Get the `FunctionDeclaration` for the function named by `name`.
    pub fn get_function_decl(&self, name: &ir::ExternalName) -> &FunctionDeclaration {
        &self.contents.get_function_info(name).decl
    }

    /// Get the `DataDeclaration` for the function named by `name`.
    pub fn get_data_decl(&self, name: &ir::ExternalName) -> &DataDeclaration {
        &self.contents.get_data_info(name).decl
    }

    /// Get the definition for the function named by `name`, along with its name
    /// and signature.
    pub fn get_function_definition(
        &self,
        name: &ir::ExternalName,
    ) -> (Option<&B::CompiledFunction>, &str, &ir::Signature) {
        let info = self.contents.get_function_info(name);
        debug_assert_eq!(info.decl.linkage.is_definable(), info.compiled.is_some());
        (
            info.compiled.as_ref(),
            &info.decl.name,
            &info.decl.signature,
        )
    }

    /// Get the definition for the data object named by `name`, along with its name
    /// and writable flag
    pub fn get_data_definition(
        &self,
        name: &ir::ExternalName,
    ) -> (Option<&B::CompiledData>, &str, bool) {
        let info = self.contents.get_data_info(name);
        debug_assert_eq!(info.decl.linkage.is_definable(), info.compiled.is_some());
        (info.compiled.as_ref(), &info.decl.name, info.decl.writable)
    }

    /// Return whether `name` names a function, rather than a data object.
    pub fn is_function(&self, name: &ir::ExternalName) -> bool {
        if let ir::ExternalName::User { namespace, .. } = *name {
            namespace == 0
        } else {
            panic!("unexpected ExternalName kind")
        }
    }
}

/// A `Module` is a utility for collecting functions and data objects, and linking them together.
pub struct Module<B>
where
    B: Backend,
{
    names: HashMap<String, FuncOrDataId>,
    contents: ModuleContents<B>,
    backend: B,
}

impl<B> Module<B>
where
    B: Backend,
{
    /// Create a new `Module`.
    pub fn new(backend_builder: B::Builder) -> Self {
        Self {
            names: HashMap::new(),
            contents: ModuleContents {
                functions: PrimaryMap::new(),
                data_objects: PrimaryMap::new(),
            },
            backend: B::new(backend_builder),
        }
    }

    /// Return then pointer type for the current target.
    pub fn pointer_type(&self) -> ir::types::Type {
        if self.backend.isa().flags().is_64bit() {
            ir::types::I64
        } else {
            ir::types::I32
        }
    }

    /// Declare a function in this module.
    pub fn declare_function(
        &mut self,
        name: &str,
        linkage: Linkage,
        signature: &ir::Signature,
    ) -> Result<FuncId, CtonError> {
        // TODO: Can we avoid allocating names so often?
        use std::collections::hash_map::Entry::*;
        match self.names.entry(name.to_owned()) {
            Occupied(entry) => {
                match *entry.get() {
                    FuncOrDataId::Func(id) => {
                        let existing = &mut self.contents.functions[id];
                        existing.merge(linkage);
                        self.backend.declare_function(name, existing.decl.linkage);
                        Ok(id)
                    }
                    FuncOrDataId::Data(..) => unimplemented!(),
                }
            }
            Vacant(entry) => {
                let id = self.contents.functions.push(ModuleFunction {
                    decl: FunctionDeclaration {
                        name: name.to_owned(),
                        linkage,
                        signature: signature.clone(),
                    },
                    compiled: None,
                    finalized: false,
                });
                entry.insert(FuncOrDataId::Func(id));
                self.backend.declare_function(name, linkage);
                Ok(id)
            }
        }
    }

    /// Declare a data object in this module.
    pub fn declare_data(
        &mut self,
        name: &str,
        linkage: Linkage,
        writable: bool,
    ) -> Result<DataId, CtonError> {
        // TODO: Can we avoid allocating names so often?
        use std::collections::hash_map::Entry::*;
        match self.names.entry(name.to_owned()) {
            Occupied(entry) => {
                match *entry.get() {
                    FuncOrDataId::Data(id) => {
                        let existing = &mut self.contents.data_objects[id];
                        existing.merge(linkage, writable);
                        self.backend.declare_data(
                            name,
                            existing.decl.linkage,
                            existing.decl.writable,
                        );
                        Ok(id)
                    }

                    FuncOrDataId::Func(..) => unimplemented!(),
                }
            }
            Vacant(entry) => {
                let id = self.contents.data_objects.push(ModuleData {
                    decl: DataDeclaration {
                        name: name.to_owned(),
                        linkage,
                        writable,
                    },
                    compiled: None,
                    finalized: false,
                });
                entry.insert(FuncOrDataId::Data(id));
                self.backend.declare_data(name, linkage, writable);
                Ok(id)
            }
        }
    }

    /// Use this when you're building the IR of a function to reference a function.
    ///
    /// TODO: Coalesce redundant decls and signatures.
    /// TODO: Look into ways to reduce the risk of using a FuncRef in the wrong function.
    pub fn declare_func_in_func(&self, func: FuncId, in_func: &mut ir::Function) -> ir::FuncRef {
        let decl = &self.contents.functions[func].decl;
        let signature = in_func.import_signature(decl.signature.clone());
        let colocated = decl.linkage.is_final();
        in_func.import_function(ir::ExtFuncData {
            name: ir::ExternalName::user(0, func.index() as u32),
            signature,
            colocated,
        })
    }

    /// Use this when you're building the IR of a function to reference a data object.
    ///
    /// TODO: Same as above.
    pub fn declare_data_in_func(&self, data: DataId, func: &mut ir::Function) -> ir::GlobalVar {
        let decl = &self.contents.data_objects[data].decl;
        let colocated = decl.linkage.is_final();
        func.create_global_var(ir::GlobalVarData::Sym {
            name: ir::ExternalName::user(1, data.index() as u32),
            colocated,
        })
    }

    /// TODO: Same as above.
    pub fn declare_func_in_data(&self, func: FuncId, ctx: &mut DataContext) -> ir::FuncRef {
        ctx.import_function(ir::ExternalName::user(0, func.index() as u32))
    }

    /// TODO: Same as above.
    pub fn declare_data_in_data(&self, data: DataId, ctx: &mut DataContext) -> ir::GlobalVar {
        ctx.import_global_var(ir::ExternalName::user(1, data.index() as u32))
    }

    /// Define a function, producing the function body from the given `Context`.
    pub fn define_function(&mut self, func: FuncId, ctx: &mut Context) -> CtonResult {
        let compiled = {
            let code_size = ctx.compile(self.backend.isa())?;

            let info = &self.contents.functions[func];
            debug_assert!(
                info.compiled.is_none(),
                "functions can be defined only once"
            );
            debug_assert!(
                info.decl.linkage.is_definable(),
                "imported functions cannot be defined"
            );
            Some(self.backend.define_function(
                &info.decl.name,
                ctx,
                &ModuleNamespace::<B> {
                    contents: &self.contents,
                },
                code_size,
            )?)
        };
        self.contents.functions[func].compiled = compiled;
        Ok(())
    }

    /// Define a function, producing the data contents from the given `DataContext`.
    pub fn define_data(&mut self, data: DataId, data_ctx: &DataContext) -> CtonResult {
        let compiled = {
            let info = &self.contents.data_objects[data];
            debug_assert!(
                info.compiled.is_none(),
                "functions can be defined only once"
            );
            debug_assert!(
                info.decl.linkage.is_definable(),
                "imported functions cannot be defined"
            );
            Some(self.backend.define_data(
                &info.decl.name,
                data_ctx,
                &ModuleNamespace::<B> {
                    contents: &self.contents,
                },
            )?)
        };
        self.contents.data_objects[data].compiled = compiled;
        Ok(())
    }

    /// Write the address of `what` into the data for `data` at `offset`. `data` must refer to a
    /// defined data object.
    pub fn write_data_funcaddr(&mut self, data: DataId, offset: usize, what: ir::FuncRef) {
        let info = &mut self.contents.data_objects[data];
        debug_assert!(
            info.decl.linkage.is_definable(),
            "imported data cannot contain references"
        );
        self.backend.write_data_funcaddr(
            &mut info.compiled.as_mut().expect(
                "`data` must refer to a defined data object",
            ),
            offset,
            what,
        );
    }

    /// Write the address of `what` plus `addend` into the data for `data` at `offset`. `data` must
    /// refer to a defined data object.
    pub fn write_data_dataaddr(
        &mut self,
        data: DataId,
        offset: usize,
        what: ir::GlobalVar,
        addend: binemit::Addend,
    ) {
        let info = &mut self.contents.data_objects[data];
        debug_assert!(
            info.decl.linkage.is_definable(),
            "imported data cannot contain references"
        );
        self.backend.write_data_dataaddr(
            &mut info.compiled.as_mut().expect(
                "`data` must refer to a defined data object",
            ),
            offset,
            what,
            addend,
        );
    }

    /// Perform all outstanding relocations on the given function. This requires all `Local`
    /// and `Export` entities referenced to be defined.
    pub fn finalize_function(&mut self, func: FuncId) -> B::FinalizedFunction {
        let output = {
            let info = &self.contents.functions[func];
            debug_assert!(
                info.decl.linkage.is_definable(),
                "imported data cannot be finalized"
            );
            self.backend.finalize_function(
                info.compiled.as_ref().expect(
                    "function must be compiled before it can be finalized",
                ),
                &ModuleNamespace::<B> { contents: &self.contents },
            )
        };
        self.contents.functions[func].finalized = true;
        output
    }

    /// Perform all outstanding relocations on the given data object. This requires all
    /// `Local` and `Export` entities referenced to be defined.
    pub fn finalize_data(&mut self, data: DataId) -> B::FinalizedData {
        let output = {
            let info = &self.contents.data_objects[data];
            debug_assert!(
                info.decl.linkage.is_definable(),
                "imported data cannot be finalized"
            );
            self.backend.finalize_data(
                info.compiled.as_ref().expect(
                    "data object must be compiled before it can be finalized",
                ),
                &ModuleNamespace::<B> { contents: &self.contents },
            )
        };
        self.contents.data_objects[data].finalized = true;
        output
    }

    /// Finalize all functions and data objects. Note that this doesn't return the
    /// final artifacts returned from `finalize_function` or `finalize_data`.
    pub fn finalize_all(&mut self) {
        // TODO: Could we use something like `into_iter()` here?
        for info in self.contents.functions.values() {
            if info.decl.linkage.is_definable() && !info.finalized {
                self.backend.finalize_function(
                    info.compiled.as_ref().expect(
                        "function must be compiled before it can be finalized",
                    ),
                    &ModuleNamespace::<B> { contents: &self.contents },
                );
            }
        }
        for info in self.contents.data_objects.values() {
            if info.decl.linkage.is_definable() && !info.finalized {
                self.backend.finalize_data(
                    info.compiled.as_ref().expect(
                        "data object must be compiled before it can be finalized",
                    ),
                    &ModuleNamespace::<B> { contents: &self.contents },
                );
            }
        }
    }

    /// Consume the module and return the resulting `Product`. Some `Backend`
    /// implementations may provide additional functionality available after
    /// a `Module` is complete.
    pub fn finish(self) -> B::Product {
        self.backend.finish()
    }
}
