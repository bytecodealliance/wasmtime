//! Define the `instantiate` function, which takes a byte array containing an
//! encoded wasm module and returns a live wasm instance. Also, define
//! `CompiledModule` to allow compiling and instantiating to be done as separate
//! steps.

use crate::code_memory::CodeMemory;
use crate::compiler::{Compilation, Compiler};
use crate::link::link_module;
use crate::object::ObjectUnwindInfo;
use object::File as ObjectFile;
#[cfg(feature = "parallel-compilation")]
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::sync::Arc;
use thiserror::Error;
use wasmtime_debug::create_gdbjit_image;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::isa::TargetIsa;
use wasmtime_environ::wasm::{DefinedFuncIndex, SignatureIndex};
use wasmtime_environ::{
    CompileError, DataInitializer, DataInitializerLocation, FunctionAddressMap, Module,
    ModuleEnvironment, ModuleTranslation, StackMapInformation, TrapInformation,
};
use wasmtime_profiling::ProfilingAgent;
use wasmtime_runtime::{
    GdbJitImageRegistration, Imports, InstanceHandle, InstantiationError, RuntimeMemoryCreator,
    StackMapRegistry, VMExternRefActivationsTable, VMFunctionBody, VMInterrupts,
    VMSharedSignatureIndex, VMTrampoline,
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

/// Contains all compilation artifacts.
#[derive(Serialize, Deserialize)]
pub struct CompilationArtifacts {
    /// Module metadata.
    module: Module,

    /// ELF image with functions code.
    obj: Box<[u8]>,

    /// Unwind information for function code.
    unwind_info: Box<[ObjectUnwindInfo]>,

    /// Data initiailizers.
    data_initializers: Box<[OwnedDataInitializer]>,

    /// Descriptions of compiled functions
    funcs: PrimaryMap<DefinedFuncIndex, FunctionInfo>,

    /// Debug info presence flags.
    debug_info: bool,
}

impl CompilationArtifacts {
    /// Creates a `CompilationArtifacts` for a singular translated wasm module.
    pub fn build(
        compiler: &Compiler,
        data: &[u8],
    ) -> Result<Vec<CompilationArtifacts>, SetupError> {
        let translations = ModuleEnvironment::new(
            compiler.frontend_config(),
            compiler.tunables(),
            compiler.features(),
        )
        .translate(data)
        .map_err(|error| SetupError::Compile(CompileError::Wasm(error)))?;

        maybe_parallel!(translations.(into_iter | into_par_iter))
            .map(|mut translation| {
                let Compilation {
                    obj,
                    unwind_info,
                    funcs,
                } = compiler.compile(&mut translation)?;

                let ModuleTranslation {
                    module,
                    data_initializers,
                    ..
                } = translation;

                let data_initializers = data_initializers
                    .into_iter()
                    .map(OwnedDataInitializer::new)
                    .collect::<Vec<_>>()
                    .into_boxed_slice();

                let obj = obj.write().map_err(|_| {
                    SetupError::Instantiate(InstantiationError::Resource(
                        "failed to create image memory".to_string(),
                    ))
                })?;

                Ok(CompilationArtifacts {
                    module,
                    obj: obj.into_boxed_slice(),
                    unwind_info: unwind_info.into_boxed_slice(),
                    data_initializers,
                    funcs: funcs
                        .into_iter()
                        .map(|(_, func)| FunctionInfo {
                            stack_maps: func.stack_maps,
                            traps: func.traps,
                            address_map: func.address_map,
                        })
                        .collect(),
                    debug_info: compiler.tunables().debug_info,
                })
            })
            .collect::<Result<Vec<_>, SetupError>>()
    }
}

struct FinishedFunctions(PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>);

#[derive(Serialize, Deserialize, Clone)]
struct FunctionInfo {
    traps: Vec<TrapInformation>,
    address_map: FunctionAddressMap,
    stack_maps: Vec<StackMapInformation>,
}

unsafe impl Send for FinishedFunctions {}
unsafe impl Sync for FinishedFunctions {}

/// Container for data needed for an Instance function to exist.
pub struct ModuleCode {
    code_memory: CodeMemory,
    #[allow(dead_code)]
    dbg_jit_registration: Option<GdbJitImageRegistration>,
}

/// A compiled wasm module, ready to be instantiated.
pub struct CompiledModule {
    artifacts: CompilationArtifacts,
    module: Arc<Module>,
    code: Arc<ModuleCode>,
    finished_functions: FinishedFunctions,
    trampolines: PrimaryMap<SignatureIndex, VMTrampoline>,
}

impl CompiledModule {
    /// Creates a list of compiled modules from the given list of compilation
    /// artifacts.
    pub fn from_artifacts_list(
        artifacts: Vec<CompilationArtifacts>,
        isa: &dyn TargetIsa,
        profiler: &dyn ProfilingAgent,
    ) -> Result<Vec<Self>, SetupError> {
        maybe_parallel!(artifacts.(into_iter | into_par_iter))
            .map(|a| CompiledModule::from_artifacts(a, isa, profiler))
            .collect()
    }

    /// Creates `CompiledModule` directly from `CompilationArtifacts`.
    pub fn from_artifacts(
        artifacts: CompilationArtifacts,
        isa: &dyn TargetIsa,
        profiler: &dyn ProfilingAgent,
    ) -> Result<Self, SetupError> {
        // Allocate all of the compiled functions into executable memory,
        // copying over their contents.
        let (code_memory, code_range, finished_functions, trampolines) = build_code_memory(
            isa,
            &artifacts.obj,
            &artifacts.module,
            &artifacts.unwind_info,
        )
        .map_err(|message| {
            SetupError::Instantiate(InstantiationError::Resource(format!(
                "failed to build code memory for functions: {}",
                message
            )))
        })?;

        // Register GDB JIT images; initialize profiler and load the wasm module.
        let dbg_jit_registration = if artifacts.debug_info {
            let bytes = create_dbg_image(
                artifacts.obj.to_vec(),
                code_range,
                &artifacts.module,
                &finished_functions,
            )?;
            profiler.module_load(&artifacts.module, &finished_functions, Some(&bytes));
            let reg = GdbJitImageRegistration::register(bytes);
            Some(reg)
        } else {
            profiler.module_load(&artifacts.module, &finished_functions, None);
            None
        };

        let finished_functions = FinishedFunctions(finished_functions);

        Ok(Self {
            module: Arc::new(artifacts.module.clone()),
            artifacts,
            code: Arc::new(ModuleCode {
                code_memory,
                dbg_jit_registration,
            }),
            finished_functions,
            trampolines,
        })
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
        imports: Imports<'_>,
        lookup_shared_signature: &dyn Fn(SignatureIndex) -> VMSharedSignatureIndex,
        mem_creator: Option<&dyn RuntimeMemoryCreator>,
        interrupts: *const VMInterrupts,
        host_state: Box<dyn Any>,
        externref_activations_table: *mut VMExternRefActivationsTable,
        stack_map_registry: *mut StackMapRegistry,
    ) -> Result<InstanceHandle, InstantiationError> {
        InstanceHandle::new(
            self.module.clone(),
            self.code.clone(),
            &self.finished_functions.0,
            imports,
            mem_creator,
            lookup_shared_signature,
            host_state,
            interrupts,
            externref_activations_table,
            stack_map_registry,
        )
    }
    /// Extracts `CompilationArtifacts` from the compiled module.
    pub fn compilation_artifacts(&self) -> &CompilationArtifacts {
        &self.artifacts
    }

    /// Returns data initializers to pass to `InstanceHandle::initialize`
    pub fn data_initializers(&self) -> Vec<DataInitializer<'_>> {
        self.artifacts
            .data_initializers
            .iter()
            .map(|init| DataInitializer {
                location: init.location.clone(),
                data: &*init.data,
            })
            .collect()
    }

    /// Return a reference-counting pointer to a module.
    pub fn module(&self) -> &Arc<Module> {
        &self.module
    }

    /// Return a reference to a mutable module (if possible).
    pub fn module_mut(&mut self) -> Option<&mut Module> {
        Arc::get_mut(&mut self.module)
    }

    /// Returns the map of all finished JIT functions compiled for this module
    pub fn finished_functions(&self) -> &PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]> {
        &self.finished_functions.0
    }

    /// Returns the per-signature trampolines for this module.
    pub fn trampolines(&self) -> &PrimaryMap<SignatureIndex, VMTrampoline> {
        &self.trampolines
    }

    /// Returns the stack map information for all functions defined in this
    /// module.
    ///
    /// The iterator returned iterates over the span of the compiled function in
    /// memory with the stack maps associated with those bytes.
    pub fn stack_maps(
        &self,
    ) -> impl Iterator<Item = (*mut [VMFunctionBody], &[StackMapInformation])> {
        self.finished_functions().values().copied().zip(
            self.artifacts
                .funcs
                .values()
                .map(|f| f.stack_maps.as_slice()),
        )
    }

    /// Iterates over all functions in this module, returning information about
    /// how to decode traps which happen in the function.
    pub fn trap_information(
        &self,
    ) -> impl Iterator<
        Item = (
            DefinedFuncIndex,
            *mut [VMFunctionBody],
            &[TrapInformation],
            &FunctionAddressMap,
        ),
    > {
        self.finished_functions()
            .iter()
            .zip(self.artifacts.funcs.values())
            .map(|((i, alloc), func)| (i, *alloc, func.traps.as_slice(), &func.address_map))
    }

    /// Returns all ranges convered by JIT code.
    pub fn jit_code_ranges<'a>(&'a self) -> impl Iterator<Item = (usize, usize)> + 'a {
        self.code.code_memory.published_ranges()
    }

    /// Returns module's JIT code.
    pub fn code(&self) -> &Arc<ModuleCode> {
        &self.code
    }
}

/// Similar to `DataInitializer`, but owns its own copy of the data rather
/// than holding a slice of the original module.
#[derive(Clone, Serialize, Deserialize)]
pub struct OwnedDataInitializer {
    /// The location where the initialization is to be performed.
    location: DataInitializerLocation,

    /// The initialization data.
    data: Box<[u8]>,
}

impl OwnedDataInitializer {
    fn new(borrowed: DataInitializer<'_>) -> Self {
        Self {
            location: borrowed.location.clone(),
            data: borrowed.data.to_vec().into_boxed_slice(),
        }
    }
}

fn create_dbg_image(
    obj: Vec<u8>,
    code_range: (*const u8, usize),
    module: &Module,
    finished_functions: &PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
) -> Result<Vec<u8>, SetupError> {
    let funcs = finished_functions
        .values()
        .map(|allocated: &*mut [VMFunctionBody]| (*allocated) as *const u8)
        .collect::<Vec<_>>();
    create_gdbjit_image(obj, code_range, module.num_imported_funcs, &funcs)
        .map_err(SetupError::DebugInfo)
}

fn build_code_memory(
    isa: &dyn TargetIsa,
    obj: &[u8],
    module: &Module,
    unwind_info: &Box<[ObjectUnwindInfo]>,
) -> Result<
    (
        CodeMemory,
        (*const u8, usize),
        PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
        PrimaryMap<SignatureIndex, VMTrampoline>,
    ),
    String,
> {
    let obj = ObjectFile::parse(obj).map_err(|_| "Unable to read obj".to_string())?;

    let mut code_memory = CodeMemory::new();

    let allocation = code_memory.allocate_for_object(&obj, unwind_info)?;

    // Second, create a PrimaryMap from result vector of pointers.
    let mut finished_functions = PrimaryMap::new();
    for (i, fat_ptr) in allocation.funcs() {
        let fat_ptr: *mut [VMFunctionBody] = fat_ptr;
        assert_eq!(
            Some(finished_functions.push(fat_ptr)),
            module.defined_func_index(i)
        );
    }

    let mut trampolines = PrimaryMap::new();
    for (i, fat_ptr) in allocation.trampolines() {
        let fat_ptr =
            unsafe { std::mem::transmute::<*const VMFunctionBody, VMTrampoline>(fat_ptr.as_ptr()) };
        assert_eq!(trampolines.push(fat_ptr), i);
    }

    let code_range = allocation.code_range();

    link_module(&obj, &module, code_range, &finished_functions);

    let code_range = (code_range.as_ptr(), code_range.len());

    // Make all code compiled thus far executable.
    code_memory.publish(isa);

    Ok((code_memory, code_range, finished_functions, trampolines))
}
