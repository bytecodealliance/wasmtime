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
use std::ops::Range;
use std::sync::Arc;
use thiserror::Error;
use wasmtime_debug::create_gdbjit_image;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::isa::TargetIsa;
use wasmtime_environ::wasm::{
    DefinedFuncIndex, InstanceTypeIndex, ModuleTypeIndex, SignatureIndex, WasmFuncType,
};
use wasmtime_environ::{
    CompileError, DataInitializer, DataInitializerLocation, DebugInfoData, FunctionAddressMap,
    InstanceSignature, Module, ModuleEnvironment, ModuleSignature, ModuleTranslation,
    StackMapInformation, TrapInformation,
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

    /// Whether or not native debug information is available in `obj`
    native_debug_info_present: bool,

    /// Whether or not the original wasm module contained debug information that
    /// we skipped and did not parse.
    has_unparsed_debuginfo: bool,

    /// Debug information found in the wasm file, used for symbolicating
    /// backtraces.
    debug_info: Option<DebugInfo>,
}

#[derive(Serialize, Deserialize)]
struct DebugInfo {
    data: Box<[u8]>,
    code_section_offset: u64,
    debug_abbrev: Range<usize>,
    debug_addr: Range<usize>,
    debug_info: Range<usize>,
    debug_line: Range<usize>,
    debug_line_str: Range<usize>,
    debug_ranges: Range<usize>,
    debug_rnglists: Range<usize>,
    debug_str: Range<usize>,
    debug_str_offsets: Range<usize>,
}

impl CompilationArtifacts {
    /// Creates a `CompilationArtifacts` for a singular translated wasm module.
    pub fn build(
        compiler: &Compiler,
        data: &[u8],
    ) -> Result<(usize, Vec<CompilationArtifacts>, TypeTables), SetupError> {
        let (main_module, translations, types) = ModuleEnvironment::new(
            compiler.frontend_config(),
            compiler.tunables(),
            compiler.features(),
        )
        .translate(data)
        .map_err(|error| SetupError::Compile(CompileError::Wasm(error)))?;

        let list = maybe_parallel!(translations.(into_iter | into_par_iter))
            .map(|mut translation| {
                let Compilation {
                    obj,
                    unwind_info,
                    funcs,
                } = compiler.compile(&mut translation, &types)?;

                let ModuleTranslation {
                    module,
                    data_initializers,
                    debuginfo,
                    has_unparsed_debuginfo,
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
                    native_debug_info_present: compiler.tunables().generate_native_debuginfo,
                    debug_info: if compiler.tunables().parse_wasm_debuginfo {
                        Some(debuginfo.into())
                    } else {
                        None
                    },
                    has_unparsed_debuginfo,
                })
            })
            .collect::<Result<Vec<_>, SetupError>>()?;
        Ok((
            main_module,
            list,
            TypeTables {
                wasm_signatures: types.wasm_signatures,
                module_signatures: types.module_signatures,
                instance_signatures: types.instance_signatures,
            },
        ))
    }
}

struct FinishedFunctions(PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>);
unsafe impl Send for FinishedFunctions {}
unsafe impl Sync for FinishedFunctions {}

#[derive(Serialize, Deserialize, Clone)]
struct FunctionInfo {
    traps: Vec<TrapInformation>,
    address_map: FunctionAddressMap,
    stack_maps: Vec<StackMapInformation>,
}

/// This is intended to mirror the type tables in `wasmtime_environ`, except that
/// it doesn't store the native signatures which are no longer needed past compilation.
#[derive(Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct TypeTables {
    pub wasm_signatures: PrimaryMap<SignatureIndex, WasmFuncType>,
    pub module_signatures: PrimaryMap<ModuleTypeIndex, ModuleSignature>,
    pub instance_signatures: PrimaryMap<InstanceTypeIndex, InstanceSignature>,
}

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
    ) -> Result<Vec<Arc<Self>>, SetupError> {
        maybe_parallel!(artifacts.(into_iter | into_par_iter))
            .map(|a| CompiledModule::from_artifacts(a, isa, profiler))
            .collect()
    }

    /// Creates `CompiledModule` directly from `CompilationArtifacts`.
    pub fn from_artifacts(
        artifacts: CompilationArtifacts,
        isa: &dyn TargetIsa,
        profiler: &dyn ProfilingAgent,
    ) -> Result<Arc<Self>, SetupError> {
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
        let dbg_jit_registration = if artifacts.native_debug_info_present {
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

        Ok(Arc::new(Self {
            module: Arc::new(artifacts.module.clone()),
            artifacts,
            code: Arc::new(ModuleCode {
                code_memory,
                dbg_jit_registration,
            }),
            finished_functions,
            trampolines,
        }))
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

    /// Creates a new symbolication context which can be used to further
    /// symbolicate stack traces.
    ///
    /// Basically this makes a thing which parses debuginfo and can tell you
    /// what filename and line number a wasm pc comes from.
    pub fn symbolize_context(&self) -> Result<Option<SymbolizeContext>, gimli::Error> {
        use gimli::EndianSlice;
        let info = match &self.artifacts.debug_info {
            Some(info) => info,
            None => return Ok(None),
        };
        // For now we clone the data into the `SymbolizeContext`, but if this
        // becomes prohibitive we could always `Arc` it with our own allocation
        // here.
        let data = info.data.clone();
        let endian = gimli::LittleEndian;
        let cx = addr2line::Context::from_sections(
            EndianSlice::new(&data[info.debug_abbrev.clone()], endian).into(),
            EndianSlice::new(&data[info.debug_addr.clone()], endian).into(),
            EndianSlice::new(&data[info.debug_info.clone()], endian).into(),
            EndianSlice::new(&data[info.debug_line.clone()], endian).into(),
            EndianSlice::new(&data[info.debug_line_str.clone()], endian).into(),
            EndianSlice::new(&data[info.debug_ranges.clone()], endian).into(),
            EndianSlice::new(&data[info.debug_rnglists.clone()], endian).into(),
            EndianSlice::new(&data[info.debug_str.clone()], endian).into(),
            EndianSlice::new(&data[info.debug_str_offsets.clone()], endian).into(),
            EndianSlice::new(&[], endian),
        )?;
        Ok(Some(SymbolizeContext {
            // See comments on `SymbolizeContext` for why we do this static
            // lifetime promotion.
            inner: unsafe {
                std::mem::transmute::<Addr2LineContext<'_>, Addr2LineContext<'static>>(cx)
            },
            code_section_offset: info.code_section_offset,
            _data: data,
        }))
    }

    /// Returns whether the original wasm module had unparsed debug information
    /// based on the tunables configuration.
    pub fn has_unparsed_debuginfo(&self) -> bool {
        self.artifacts.has_unparsed_debuginfo
    }
}

type Addr2LineContext<'a> = addr2line::Context<gimli::EndianSlice<'a, gimli::LittleEndian>>;

/// A context which contains dwarf debug information to translate program
/// counters back to filenames and line numbers.
pub struct SymbolizeContext {
    // Note the `'static` lifetime on `inner`. That's actually a bunch of slices
    // which point back into the `_data` field. We currently unsafely manage
    // this by saying that when inside the struct it's `'static` (since we own
    // the referenced data just next to it) and we only loan out borrowed
    // references.
    _data: Box<[u8]>,
    inner: Addr2LineContext<'static>,
    code_section_offset: u64,
}

impl SymbolizeContext {
    /// Returns access to the [`addr2line::Context`] which can be used to query
    /// frame information with.
    pub fn addr2line(&self) -> &Addr2LineContext<'_> {
        // Here we demote our synthetic `'static` lifetime which doesn't
        // actually exist back to a lifetime that's tied to `&self`, which
        // should be safe.
        unsafe {
            std::mem::transmute::<&Addr2LineContext<'static>, &Addr2LineContext<'_>>(&self.inner)
        }
    }

    /// Returns the offset of the code section in the original wasm file, used
    /// to calculate lookup values into the DWARF.
    pub fn code_section_offset(&self) -> u64 {
        self.code_section_offset
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

impl From<DebugInfoData<'_>> for DebugInfo {
    fn from(raw: DebugInfoData<'_>) -> DebugInfo {
        use gimli::Section;

        let mut data = Vec::new();
        let mut push = |section: &[u8]| {
            data.extend_from_slice(section);
            data.len() - section.len()..data.len()
        };
        let debug_abbrev = push(raw.dwarf.debug_abbrev.reader().slice());
        let debug_addr = push(raw.dwarf.debug_addr.reader().slice());
        let debug_info = push(raw.dwarf.debug_info.reader().slice());
        let debug_line = push(raw.dwarf.debug_line.reader().slice());
        let debug_line_str = push(raw.dwarf.debug_line_str.reader().slice());
        let debug_ranges = push(raw.debug_ranges.reader().slice());
        let debug_rnglists = push(raw.debug_rnglists.reader().slice());
        let debug_str = push(raw.dwarf.debug_str.reader().slice());
        let debug_str_offsets = push(raw.dwarf.debug_str_offsets.reader().slice());
        DebugInfo {
            data: data.into(),
            debug_abbrev,
            debug_addr,
            debug_info,
            debug_line,
            debug_line_str,
            debug_ranges,
            debug_rnglists,
            debug_str,
            debug_str_offsets,
            code_section_offset: raw.wasm_file.code_section_offset,
        }
    }
}
