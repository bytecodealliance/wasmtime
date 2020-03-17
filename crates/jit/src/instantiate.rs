//! Define the `instantiate` function, which takes a byte array containing an
//! encoded wasm module and returns a live wasm instance. Also, define
//! `CompiledModule` to allow compiling and instantiating to be done as separate
//! steps.

use crate::compiler::Compiler;
use crate::imports::resolve_imports;
use crate::link::link_module;
use crate::resolver::Resolver;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use wasmtime_debug::read_debuginfo;
use wasmtime_environ::entity::{BoxedSlice, PrimaryMap};
use wasmtime_environ::wasm::{DefinedFuncIndex, SignatureIndex};
use wasmtime_environ::{
    CompileError, DataInitializer, DataInitializerLocation, Module, ModuleEnvironment,
};
use wasmtime_profiling::ProfilingAgent;
use wasmtime_runtime::{
    GdbJitImageRegistration, InstanceHandle, InstantiationError, SignatureRegistry,
    TrapRegistration, VMFunctionBody, VMSharedSignatureIndex, VMTrampoline,
};

/// An error condition while setting up a wasm instance, be it validation,
/// compilation, or instantiation.
#[derive(Error, Debug)]
pub enum SetupError {
    /// The module did not pass validation.
    #[error("Validation error: {0}")]
    Validate(String),

    /// A wasm translation error occured.
    #[error("WebAssembly failed to compile")]
    Compile(#[from] CompileError),

    /// Some runtime resource was unavailable or insufficient, or the start function
    /// trapped.
    #[error("Instantiation failed during setup")]
    Instantiate(#[from] InstantiationError),

    /// Debug information generation error occured.
    #[error("Debug information error")]
    DebugInfo(#[from] anyhow::Error),
}

/// This is similar to `CompiledModule`, but references the data initializers
/// from the wasm buffer rather than holding its own copy.
struct RawCompiledModule<'data> {
    module: Module,
    finished_functions: BoxedSlice<DefinedFuncIndex, *mut [VMFunctionBody]>,
    trampolines: HashMap<VMSharedSignatureIndex, VMTrampoline>,
    data_initializers: Box<[DataInitializer<'data>]>,
    signatures: BoxedSlice<SignatureIndex, VMSharedSignatureIndex>,
    dbg_jit_registration: Option<GdbJitImageRegistration>,
    trap_registration: TrapRegistration,
}

impl<'data> RawCompiledModule<'data> {
    /// Create a new `RawCompiledModule` by compiling the wasm module in `data` and instatiating it.
    fn new(
        compiler: &mut Compiler,
        data: &'data [u8],
        debug_info: bool,
        profiler: Option<&Arc<Mutex<Box<dyn ProfilingAgent + Send>>>>,
    ) -> Result<Self, SetupError> {
        let environ = ModuleEnvironment::new(compiler.frontend_config(), compiler.tunables());

        let translation = environ
            .translate(data)
            .map_err(|error| SetupError::Compile(CompileError::Wasm(error)))?;

        let debug_data = if debug_info {
            // TODO Do we want to ignore invalid DWARF data?
            let debug_data = read_debuginfo(&data)?;
            Some(debug_data)
        } else {
            None
        };

        let compilation = compiler.compile(
            &translation.module,
            translation.module_translation.as_ref().unwrap(),
            translation.function_body_inputs,
            debug_data,
        )?;

        link_module(&translation.module, &compilation);

        // Compute indices into the shared signature table.
        let signatures = {
            let signature_registry = compiler.signatures();
            translation
                .module
                .local
                .signatures
                .values()
                .map(|sig| signature_registry.register(sig))
                .collect::<PrimaryMap<_, _>>()
        };

        // Make all code compiled thus far executable.
        compiler.publish_compiled_code();

        // Initialize profiler and load the wasm module
        match profiler {
            Some(_) => {
                let region_name = String::from("wasm_module");
                let mut profiler = profiler.unwrap().lock().unwrap();
                match &compilation.dbg_image {
                    Some(dbg) => {
                        compiler.profiler_module_load(&mut profiler, &region_name, Some(&dbg))
                    }
                    _ => compiler.profiler_module_load(&mut profiler, &region_name, None),
                };
            }
            _ => (),
        };

        let dbg_jit_registration = if let Some(img) = compilation.dbg_image {
            let mut bytes = Vec::new();
            bytes.write_all(&img).expect("all written");
            let reg = GdbJitImageRegistration::register(bytes);
            Some(reg)
        } else {
            None
        };

        Ok(Self {
            module: translation.module,
            finished_functions: compilation.finished_functions.into_boxed_slice(),
            trampolines: compilation.trampolines,
            data_initializers: translation.data_initializers.into_boxed_slice(),
            signatures: signatures.into_boxed_slice(),
            dbg_jit_registration,
            trap_registration: compilation.trap_registration,
        })
    }
}

/// A compiled wasm module, ready to be instantiated.
pub struct CompiledModule {
    module: Arc<Module>,
    finished_functions: BoxedSlice<DefinedFuncIndex, *mut [VMFunctionBody]>,
    trampolines: HashMap<VMSharedSignatureIndex, VMTrampoline>,
    data_initializers: Box<[OwnedDataInitializer]>,
    signatures: BoxedSlice<SignatureIndex, VMSharedSignatureIndex>,
    dbg_jit_registration: Option<Rc<GdbJitImageRegistration>>,
    trap_registration: TrapRegistration,
}

impl CompiledModule {
    /// Compile a data buffer into a `CompiledModule`, which may then be instantiated.
    pub fn new<'data>(
        compiler: &mut Compiler,
        data: &'data [u8],
        debug_info: bool,
        profiler: Option<&Arc<Mutex<Box<dyn ProfilingAgent + Send>>>>,
    ) -> Result<Self, SetupError> {
        let raw = RawCompiledModule::<'data>::new(compiler, data, debug_info, profiler)?;

        Ok(Self::from_parts(
            raw.module,
            raw.finished_functions,
            raw.trampolines,
            raw.data_initializers
                .iter()
                .map(OwnedDataInitializer::new)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            raw.signatures.clone(),
            raw.dbg_jit_registration,
            raw.trap_registration,
        ))
    }

    /// Construct a `CompiledModule` from component parts.
    pub fn from_parts(
        module: Module,
        finished_functions: BoxedSlice<DefinedFuncIndex, *mut [VMFunctionBody]>,
        trampolines: HashMap<VMSharedSignatureIndex, VMTrampoline>,
        data_initializers: Box<[OwnedDataInitializer]>,
        signatures: BoxedSlice<SignatureIndex, VMSharedSignatureIndex>,
        dbg_jit_registration: Option<GdbJitImageRegistration>,
        trap_registration: TrapRegistration,
    ) -> Self {
        Self {
            module: Arc::new(module),
            finished_functions,
            trampolines,
            data_initializers,
            signatures,
            dbg_jit_registration: dbg_jit_registration.map(Rc::new),
            trap_registration,
        }
    }

    /// Crate an `Instance` from this `CompiledModule`.
    ///
    /// Note that if only one instance of this module is needed, it may be more
    /// efficient to call the top-level `instantiate`, since that avoids copying
    /// the data initializers.
    ///
    /// # Unsafety
    ///
    /// See `InstanceHandle::new`
    pub unsafe fn instantiate(
        &self,
        is_bulk_memory: bool,
        resolver: &mut dyn Resolver,
        sig_registry: &SignatureRegistry,
    ) -> Result<InstanceHandle, InstantiationError> {
        let data_initializers = self
            .data_initializers
            .iter()
            .map(|init| DataInitializer {
                location: init.location.clone(),
                data: &*init.data,
            })
            .collect::<Vec<_>>();
        let imports = resolve_imports(&self.module, &sig_registry, resolver)?;
        InstanceHandle::new(
            Arc::clone(&self.module),
            self.trap_registration.clone(),
            self.finished_functions.clone(),
            self.trampolines.clone(),
            imports,
            &data_initializers,
            self.signatures.clone(),
            self.dbg_jit_registration.as_ref().map(|r| Rc::clone(&r)),
            is_bulk_memory,
            Box::new(()),
        )
    }

    /// Return a reference-counting pointer to a module.
    pub fn module(&self) -> &Arc<Module> {
        &self.module
    }

    /// Return a reference-counting pointer to a module.
    pub fn module_mut(&mut self) -> &mut Arc<Module> {
        &mut self.module
    }

    /// Return a reference to a module.
    pub fn module_ref(&self) -> &Module {
        &self.module
    }

    /// Returns the map of all finished JIT functions compiled for this module
    pub fn finished_functions(&self) -> &BoxedSlice<DefinedFuncIndex, *mut [VMFunctionBody]> {
        &self.finished_functions
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
/// This is equivalent to creating a `CompiledModule` and calling `instantiate()` on it,
/// but avoids creating an intermediate copy of the data initializers.
///
/// # Unsafety
///
/// See `InstanceHandle::new`
#[allow(clippy::implicit_hasher)]
pub unsafe fn instantiate(
    compiler: &mut Compiler,
    data: &[u8],
    resolver: &mut dyn Resolver,
    debug_info: bool,
    is_bulk_memory: bool,
    profiler: Option<&Arc<Mutex<Box<dyn ProfilingAgent + Send>>>>,
) -> Result<InstanceHandle, SetupError> {
    let instance = CompiledModule::new(compiler, data, debug_info, profiler)?.instantiate(
        is_bulk_memory,
        resolver,
        compiler.signatures(),
    )?;
    Ok(instance)
}
