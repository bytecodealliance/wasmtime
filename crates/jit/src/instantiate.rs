//! Define the `instantiate` function, which takes a byte array containing an
//! encoded wasm module and returns a live wasm instance. Also, define
//! `CompiledModule` to allow compiling and instantiating to be done as separate
//! steps.

use crate::code_memory::CodeMemory;
use crate::compiler::{Compilation, Compiler};
use crate::imports::resolve_imports;
use crate::link::link_module;
use crate::resolver::Resolver;
use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use thiserror::Error;
use wasmtime_debug::{read_debuginfo, write_debugsections_image, DwarfSection};
use wasmtime_environ::entity::{BoxedSlice, PrimaryMap};
use wasmtime_environ::isa::TargetIsa;
use wasmtime_environ::wasm::{DefinedFuncIndex, SignatureIndex};
use wasmtime_environ::{
    CompileError, DataInitializer, DataInitializerLocation, Module, ModuleAddressMap,
    ModuleEnvironment, ModuleTranslation, StackMaps, Traps,
};
use wasmtime_profiling::ProfilingAgent;
use wasmtime_runtime::VMInterrupts;
use wasmtime_runtime::{
    GdbJitImageRegistration, InstanceHandle, InstantiationError, RuntimeMemoryCreator,
    SignatureRegistry, StackMapRegistry, VMExternRefActivationsTable, VMFunctionBody, VMTrampoline,
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

struct FinishedFunctions(BoxedSlice<DefinedFuncIndex, *mut [VMFunctionBody]>);

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
    module: Arc<Module>,
    code: Arc<ModuleCode>,
    finished_functions: FinishedFunctions,
    trampolines: PrimaryMap<SignatureIndex, VMTrampoline>,
    data_initializers: Box<[OwnedDataInitializer]>,
    traps: Traps,
    stack_maps: StackMaps,
    address_transform: ModuleAddressMap,
}

impl CompiledModule {
    /// Compile a data buffer into a `CompiledModule`, which may then be instantiated.
    pub fn new<'data>(
        compiler: &Compiler,
        data: &'data [u8],
        profiler: &dyn ProfilingAgent,
    ) -> Result<Self, SetupError> {
        let environ = ModuleEnvironment::new(compiler.frontend_config(), compiler.tunables());

        let translation = environ
            .translate(data)
            .map_err(|error| SetupError::Compile(CompileError::Wasm(error)))?;

        let mut debug_data = None;
        if compiler.tunables().debug_info {
            // TODO Do we want to ignore invalid DWARF data?
            debug_data = Some(read_debuginfo(&data)?);
        }

        let Compilation {
            mut code_memory,
            finished_functions,
            code_range,
            trampolines,
            jt_offsets,
            dwarf_sections,
            traps,
            stack_maps,
            address_transform,
        } = compiler.compile(&translation, debug_data)?;

        let ModuleTranslation {
            module,
            data_initializers,
            ..
        } = translation;

        link_module(&mut code_memory, &module, &finished_functions, &jt_offsets);

        // Make all code compiled thus far executable.
        code_memory.publish(compiler.isa());

        let data_initializers = data_initializers
            .into_iter()
            .map(OwnedDataInitializer::new)
            .collect::<Vec<_>>()
            .into_boxed_slice();

        // Register GDB JIT images; initialize profiler and load the wasm module.
        let dbg_jit_registration = if !dwarf_sections.is_empty() {
            let bytes = create_dbg_image(
                dwarf_sections,
                compiler.isa(),
                code_range,
                &finished_functions,
            )?;

            profiler.module_load(&module, &finished_functions, Some(&bytes));

            let reg = GdbJitImageRegistration::register(bytes);
            Some(reg)
        } else {
            profiler.module_load(&module, &finished_functions, None);
            None
        };

        let finished_functions = FinishedFunctions(finished_functions.into_boxed_slice());

        Ok(Self {
            module: Arc::new(module),
            code: Arc::new(ModuleCode {
                code_memory,
                dbg_jit_registration,
            }),
            finished_functions,
            trampolines,
            data_initializers,
            traps,
            stack_maps,
            address_transform,
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
        resolver: &mut dyn Resolver,
        signature_registry: &mut SignatureRegistry,
        mem_creator: Option<&dyn RuntimeMemoryCreator>,
        interrupts: Arc<VMInterrupts>,
        host_state: Box<dyn Any>,
        externref_activations_table: Rc<VMExternRefActivationsTable>,
        stack_map_registry: Arc<StackMapRegistry>,
    ) -> Result<InstanceHandle, InstantiationError> {
        // Compute indices into the shared signature table.
        let signatures = {
            self.module
                .local
                .signatures
                .values()
                .map(|(wasm_sig, native)| {
                    signature_registry.register(wasm_sig.clone(), native.clone())
                })
                .collect::<PrimaryMap<_, _>>()
        };

        let mut trampolines = HashMap::new();
        for (i, trampoline) in self.trampolines.iter() {
            trampolines.insert(signatures[i], trampoline.clone());
        }

        let finished_functions = self.finished_functions.0.clone();

        let imports = resolve_imports(&self.module, signature_registry, resolver)?;
        InstanceHandle::new(
            self.module.clone(),
            self.code.clone(),
            finished_functions,
            trampolines,
            imports,
            mem_creator,
            signatures.into_boxed_slice(),
            host_state,
            interrupts,
            externref_activations_table,
            stack_map_registry,
        )
    }

    /// Returns data initializers to pass to `InstanceHandle::initialize`
    pub fn data_initializers(&self) -> Vec<DataInitializer<'_>> {
        self.data_initializers
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
    pub fn finished_functions(&self) -> &BoxedSlice<DefinedFuncIndex, *mut [VMFunctionBody]> {
        &self.finished_functions.0
    }

    /// Returns the map for all traps in this module.
    pub fn traps(&self) -> &Traps {
        &self.traps
    }

    /// Returns the map for each of this module's stack maps.
    pub fn stack_maps(&self) -> &StackMaps {
        &self.stack_maps
    }

    /// Returns a map of compiled addresses back to original bytecode offsets.
    pub fn address_transform(&self) -> &ModuleAddressMap {
        &self.address_transform
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
    dwarf_sections: Vec<DwarfSection>,
    isa: &dyn TargetIsa,
    code_range: (*const u8, usize),
    finished_functions: &PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
) -> Result<Vec<u8>, SetupError> {
    let funcs = finished_functions
        .values()
        .map(|allocated: &*mut [VMFunctionBody]| (*allocated) as *const u8)
        .collect::<Vec<_>>();
    write_debugsections_image(isa, dwarf_sections, code_range, &funcs)
        .map_err(SetupError::DebugInfo)
}
