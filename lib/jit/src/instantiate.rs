//! Define the `instantiate` function, which takes a byte array containing an
//! encoded wasm module and returns a live wasm instance. Also, define
//! `CompiledModule` to allow compiling and instantiating to be done as separate
//! steps.

use super::HashMap;
use crate::compiler::Compiler;
use crate::link::link_module;
use crate::resolver::Resolver;
use core::cell::RefCell;
use cranelift_entity::{BoxedSlice, PrimaryMap};
use cranelift_wasm::{DefinedFuncIndex, SignatureIndex};
use std::boxed::Box;
use std::rc::Rc;
use std::string::String;
use std::vec::Vec;
use wasmtime_environ::{
    CompileError, DataInitializer, DataInitializerLocation, Module, ModuleEnvironment,
};
use wasmtime_runtime::{
    Export, Imports, InstanceHandle, InstantiationError, VMFunctionBody, VMSharedSignatureIndex,
};

/// An error condition while setting up a wasm instance, be it validation,
/// compilation, or instantiation.
#[derive(Fail, Debug)]
pub enum SetupError {
    /// The module did not pass validation.
    #[fail(display = "Validation error: {}", _0)]
    Validate(String),

    /// A wasm translation error occured.
    #[fail(display = "WebAssembly compilation error: {}", _0)]
    Compile(CompileError),

    /// Some runtime resource was unavailable or insufficient, or the start function
    /// trapped.
    #[fail(display = "Instantiation error: {}", _0)]
    Instantiate(InstantiationError),
}

/// This is similar to `CompiledModule`, but references the data initializers
/// from the wasm buffer rather than holding its own copy.
struct RawCompiledModule<'data> {
    module: Module,
    finished_functions: BoxedSlice<DefinedFuncIndex, *const VMFunctionBody>,
    imports: Imports,
    data_initializers: Box<[DataInitializer<'data>]>,
    signatures: BoxedSlice<SignatureIndex, VMSharedSignatureIndex>,
}

impl<'data> RawCompiledModule<'data> {
    /// Create a new `RawCompiledModule` by compiling the wasm module in `data` and instatiating it.
    fn new(
        compiler: &mut Compiler,
        data: &'data [u8],
        resolver: &mut dyn Resolver,
    ) -> Result<Self, SetupError> {
        let environ = ModuleEnvironment::new(compiler.frontend_config(), compiler.tunables());

        let translation = environ
            .translate(data)
            .map_err(|error| SetupError::Compile(CompileError::Wasm(error)))?;

        let (allocated_functions, relocations) =
            compiler.compile(&translation.module, translation.function_body_inputs)?;

        let imports = link_module(
            &translation.module,
            &allocated_functions,
            relocations,
            resolver,
        )
        .map_err(|err| SetupError::Instantiate(InstantiationError::Link(err)))?;

        // Gather up the pointers to the compiled functions.
        let finished_functions: BoxedSlice<DefinedFuncIndex, *const VMFunctionBody> =
            allocated_functions
                .into_iter()
                .map(|(_index, allocated)| {
                    let fatptr: *const [VMFunctionBody] = *allocated;
                    fatptr as *const VMFunctionBody
                })
                .collect::<PrimaryMap<_, _>>()
                .into_boxed_slice();

        // Compute indices into the shared signature table.
        let signatures = {
            let signature_registry = compiler.signatures();
            let mut signatures = PrimaryMap::new();
            for sig in translation.module.signatures.values() {
                signatures.push(signature_registry.register(sig));
            }
            signatures
        };

        // Make all code compiled thus far executable.
        compiler.publish_compiled_code();

        Ok(Self {
            module: translation.module,
            finished_functions,
            imports,
            data_initializers: translation.data_initializers.into_boxed_slice(),
            signatures: signatures.into_boxed_slice(),
        })
    }
}

/// A compiled wasm module, ready to be instantiated.
pub struct CompiledModule {
    module: Rc<Module>,
    finished_functions: BoxedSlice<DefinedFuncIndex, *const VMFunctionBody>,
    imports: Imports,
    data_initializers: Box<[OwnedDataInitializer]>,
    signatures: BoxedSlice<SignatureIndex, VMSharedSignatureIndex>,
    global_exports: Rc<RefCell<HashMap<String, Option<Export>>>>,
}

impl CompiledModule {
    /// Compile a data buffer into a `CompiledModule`, which may then be instantiated.
    pub fn new<'data>(
        compiler: &mut Compiler,
        data: &'data [u8],
        resolver: &mut dyn Resolver,
        global_exports: Rc<RefCell<HashMap<String, Option<Export>>>>,
    ) -> Result<Self, SetupError> {
        let raw = RawCompiledModule::<'data>::new(compiler, data, resolver)?;

        Ok(Self::from_parts(
            raw.module,
            global_exports,
            raw.finished_functions,
            raw.imports,
            raw.data_initializers
                .iter()
                .map(OwnedDataInitializer::new)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            raw.signatures.clone(),
        ))
    }

    /// Construct a `CompiledModule` from component parts.
    pub fn from_parts(
        module: Module,
        global_exports: Rc<RefCell<HashMap<String, Option<Export>>>>,
        finished_functions: BoxedSlice<DefinedFuncIndex, *const VMFunctionBody>,
        imports: Imports,
        data_initializers: Box<[OwnedDataInitializer]>,
        signatures: BoxedSlice<SignatureIndex, VMSharedSignatureIndex>,
    ) -> Self {
        Self {
            module: Rc::new(module),
            global_exports: Rc::clone(&global_exports),
            finished_functions,
            imports,
            data_initializers,
            signatures,
        }
    }

    /// Crate an `InstanceContents` from this `CompiledModule`.
    ///
    /// Note that if only one instance of this module is needed, it may be more
    /// efficient to call the top-level `instantiate`, since that avoids copying
    /// the data initializers.
    pub fn instantiate(&mut self) -> Result<InstanceHandle, InstantiationError> {
        let data_initializers = self
            .data_initializers
            .iter()
            .map(|init| DataInitializer {
                location: init.location.clone(),
                data: &*init.data,
            })
            .collect::<Vec<_>>();
        InstanceHandle::new(
            Rc::clone(&self.module),
            Rc::clone(&self.global_exports),
            self.finished_functions.clone(),
            self.imports.clone(),
            &data_initializers,
            self.signatures.clone(),
            Box::new(()),
        )
    }
}

/// Similar to `DataInitializer`, but owns its own copy of the data rather
/// than holding a slice of the original module.
pub struct OwnedDataInitializer {
    /// The location where the initialization is to be performed.
    location: DataInitializerLocation,

    /// The initialization data.
    data: Box<[u8]>,
}

impl OwnedDataInitializer {
    fn new(borrowed: &DataInitializer<'_>) -> Self {
        Self {
            location: borrowed.location.clone(),
            data: borrowed.data.to_vec().into_boxed_slice(),
        }
    }
}

/// Create a new wasm instance by compiling the wasm module in `data` and instatiating it.
///
/// This is equivalent to createing a `CompiledModule` and calling `instantiate()` on it,
/// but avoids creating an intermediate copy of the data initializers.
pub fn instantiate(
    compiler: &mut Compiler,
    data: &[u8],
    resolver: &mut dyn Resolver,
    global_exports: Rc<RefCell<HashMap<String, Option<Export>>>>,
) -> Result<InstanceHandle, SetupError> {
    let raw = RawCompiledModule::new(compiler, data, resolver)?;

    InstanceHandle::new(
        Rc::new(raw.module),
        global_exports,
        raw.finished_functions,
        raw.imports,
        &*raw.data_initializers,
        raw.signatures,
        Box::new(()),
    )
    .map_err(SetupError::Instantiate)
}
