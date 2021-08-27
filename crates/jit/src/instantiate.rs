//! Define the `instantiate` function, which takes a byte array containing an
//! encoded wasm module and returns a live wasm instance. Also, define
//! `CompiledModule` to allow compiling and instantiating to be done as separate
//! steps.

use crate::code_memory::CodeMemory;
use crate::debug::create_gdbjit_image;
use crate::link::link_module;
use crate::ProfilingAgent;
use anyhow::{anyhow, Context, Result};
use object::read::File;
use object::write::{Object, StandardSegment};
use object::{Object as _, ObjectSection, SectionKind};
use serde::{Deserialize, Serialize};
use std::ops::Range;
use std::sync::Arc;
use thiserror::Error;
use wasmtime_environ::{
    CompileError, DefinedFuncIndex, FunctionInfo, InstanceSignature, InstanceTypeIndex, Module,
    ModuleSignature, ModuleTranslation, ModuleTypeIndex, PrimaryMap, SignatureIndex,
    StackMapInformation, Tunables, WasmFuncType,
};
use wasmtime_runtime::{GdbJitImageRegistration, InstantiationError, VMFunctionBody, VMTrampoline};

/// This is the name of the section in the final ELF image which contains
/// concatenated data segments from the original wasm module.
///
/// This section is simply a list of bytes and ranges into this section are
/// stored within a `Module` for each data segment. Memory initialization and
/// passive segment management all index data directly located in this section.
///
/// Note that this implementation does not afford any method of leveraging the
/// `data.drop` instruction to actually release the data back to the OS. The
/// data section is simply always present in the ELF image. If we wanted to
/// release the data it's probably best to figure out what the best
/// implementation is for it at the time given a particular set of constraints.
const ELF_WASM_DATA: &'static str = ".rodata.wasm";

/// This is the name of the section in the final ELF image which contains a
/// `bincode`-encoded `CompiledModuleInfo`.
///
/// This section is optionally decoded in `CompiledModule::from_artifacts`
/// depending on whether or not a `CompiledModuleInfo` is already available. In
/// cases like `Module::new` where compilation directly leads into consumption,
/// it's available. In cases like `Module::deserialize` this section must be
/// decoded to get all the relevant information.
const ELF_WASMTIME_INFO: &'static str = ".wasmtime.info";

/// An error condition while setting up a wasm instance, be it validation,
/// compilation, or instantiation.
#[derive(Error, Debug)]
pub enum SetupError {
    /// The module did not pass validation.
    #[error("Validation error: {0}")]
    Validate(String),

    /// A wasm translation error occurred.
    #[error("WebAssembly failed to compile")]
    Compile(#[from] CompileError),

    /// Some runtime resource was unavailable or insufficient, or the start function
    /// trapped.
    #[error("Instantiation failed during setup")]
    Instantiate(#[from] InstantiationError),

    /// Debug information generation error occurred.
    #[error("Debug information error")]
    DebugInfo(#[from] anyhow::Error),
}

/// Final result of compilation which supports serialization to disk.
#[derive(Serialize, Deserialize)]
pub struct CompilationArtifacts {
    // NB: this structure is in a transitionary phase and will soon go away. At
    // this time it only contains the ELF image created by compilation, and in
    // the near future even this will be removed.
    obj: Box<[u8]>,
}

/// Secondary in-memory results of compilation.
///
/// This opaque structure can be optionally passed back to
/// `CompiledModule::from_artifacts` to avoid decoding extra information there.
#[derive(Serialize, Deserialize)]
pub struct CompiledModuleInfo {
    /// Type information about the compiled WebAssembly module.
    module: Module,

    /// Metadata about each compiled function.
    funcs: PrimaryMap<DefinedFuncIndex, FunctionInfo>,

    /// General compilation metadata.
    meta: Metadata,
}

#[derive(Serialize, Deserialize)]
struct Metadata {
    /// Whether or not native debug information is available in `obj`
    native_debug_info_present: bool,

    /// Whether or not the original wasm module contained debug information that
    /// we skipped and did not parse.
    has_unparsed_debuginfo: bool,

    /// Offset in the original wasm file to the code section.
    code_section_offset: u64,

    /// Whether or not custom wasm-specific dwarf sections were inserted into
    /// the ELF image.
    ///
    /// Note that even if this flag is `true` sections may be missing if they
    /// weren't found in the original wasm module itself.
    has_wasm_debuginfo: bool,
}

impl CompilationArtifacts {
    /// Finishes compilation of the `translation` specified, producing the final
    /// compilation artifacts and auxiliary information.
    ///
    /// This function will consume the final results of compiling a wasm module
    /// and finish the ELF image in-progress as part of `obj` by appending any
    /// compiler-agnostic sections.
    ///
    /// The auxiliary `CompiledModuleInfo` structure returned here has also been
    /// serialized into `CompilationArtifacts`, but if the caller will quickly
    /// turn-around and invoke `CompiledModule::from_artifacts` after this then
    /// the information can be passed to that method to avoid extra
    /// deserialization. This is done to avoid a serialize-then-deserialize for
    /// API calls like `Module::new` where the compiled module is immediately
    /// going to be used.
    pub fn new(
        translation: ModuleTranslation<'_>,
        mut obj: Object,
        funcs: PrimaryMap<DefinedFuncIndex, FunctionInfo>,
        tunables: &Tunables,
    ) -> Result<(CompilationArtifacts, CompiledModuleInfo)> {
        let ModuleTranslation {
            mut module,
            debuginfo,
            has_unparsed_debuginfo,
            data,
            passive_data,
            ..
        } = translation;

        // Place all data from the wasm module into a section which will the
        // source of the data later at runtime.
        let data_id = obj.add_section(
            obj.segment_name(StandardSegment::Data).to_vec(),
            ELF_WASM_DATA.as_bytes().to_vec(),
            SectionKind::ReadOnlyData,
        );
        let mut total_data_len = 0;
        for data in data.iter() {
            obj.append_section_data(data_id, data, 1);
            total_data_len += data.len();
        }
        for data in passive_data.iter() {
            obj.append_section_data(data_id, data, 1);
        }

        // Update passive data offsets since they're all located after the other
        // data in the module.
        for (_, range) in module.passive_data_map.iter_mut() {
            range.start = range.start.checked_add(total_data_len as u32).unwrap();
            range.end = range.end.checked_add(total_data_len as u32).unwrap();
        }

        // Insert the wasm raw wasm-based debuginfo into the output, if
        // requested. Note that this is distinct from the native debuginfo
        // possibly generated by the native compiler, hence these sections
        // getting wasm-specific names.
        if tunables.parse_wasm_debuginfo {
            push_debug(&mut obj, &debuginfo.dwarf.debug_abbrev);
            push_debug(&mut obj, &debuginfo.dwarf.debug_addr);
            push_debug(&mut obj, &debuginfo.dwarf.debug_aranges);
            push_debug(&mut obj, &debuginfo.dwarf.debug_info);
            push_debug(&mut obj, &debuginfo.dwarf.debug_line);
            push_debug(&mut obj, &debuginfo.dwarf.debug_line_str);
            push_debug(&mut obj, &debuginfo.dwarf.debug_str);
            push_debug(&mut obj, &debuginfo.dwarf.debug_str_offsets);
            push_debug(&mut obj, &debuginfo.debug_ranges);
            push_debug(&mut obj, &debuginfo.debug_rnglists);
        }

        // Encode a `CompiledModuleInfo` structure into the `ELF_WASMTIME_INFO`
        // section of this image. This is not necessary when the returned module
        // is never serialized to disk, which is also why we return a copy of
        // the `CompiledModuleInfo` structure to the caller in case they don't
        // want to deserialize this value immediately afterwards from the
        // section. Otherwise, though, this is necessary to reify a `Module` on
        // the other side from disk-serialized artifacts in
        // `Module::deserialize` (a Wasmtime API).
        let info_id = obj.add_section(
            obj.segment_name(StandardSegment::Data).to_vec(),
            ELF_WASMTIME_INFO.as_bytes().to_vec(),
            SectionKind::ReadOnlyData,
        );
        let mut bytes = Vec::new();
        let info = CompiledModuleInfo {
            module,
            funcs,
            meta: Metadata {
                native_debug_info_present: tunables.generate_native_debuginfo,
                has_unparsed_debuginfo,
                code_section_offset: debuginfo.wasm_file.code_section_offset,
                has_wasm_debuginfo: tunables.parse_wasm_debuginfo,
            },
        };
        bincode::serialize_into(&mut bytes, &info)?;
        obj.append_section_data(info_id, &bytes, 1);

        return Ok((
            CompilationArtifacts {
                obj: obj.write()?.into(),
            },
            info,
        ));

        fn push_debug<'a, T>(obj: &mut Object, section: &T)
        where
            T: gimli::Section<gimli::EndianSlice<'a, gimli::LittleEndian>>,
        {
            let data = section.reader().slice();
            if data.is_empty() {
                return;
            }
            let section_id = obj.add_section(
                obj.segment_name(StandardSegment::Debug).to_vec(),
                wasm_section_name(T::id()).as_bytes().to_vec(),
                SectionKind::Debug,
            );
            obj.append_section_data(section_id, data, 1);
        }
    }
}

struct FinishedFunctions(PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>);
unsafe impl Send for FinishedFunctions {}
unsafe impl Sync for FinishedFunctions {}

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
    range: (usize, usize),
    #[allow(dead_code)]
    code_memory: CodeMemory,
    #[allow(dead_code)]
    dbg_jit_registration: Option<GdbJitImageRegistration>,
}

impl ModuleCode {
    /// Gets the [begin, end) range of the module's code.
    pub fn range(&self) -> (usize, usize) {
        self.range
    }
}

/// A compiled wasm module, ready to be instantiated.
pub struct CompiledModule {
    wasm_data: Range<usize>,
    artifacts: CompilationArtifacts,
    module: Arc<Module>,
    funcs: PrimaryMap<DefinedFuncIndex, FunctionInfo>,
    meta: Metadata,
    code: Arc<ModuleCode>,
    finished_functions: FinishedFunctions,
    trampolines: Vec<(SignatureIndex, VMTrampoline)>,
}

impl CompiledModule {
    /// Creates `CompiledModule` directly from `CompilationArtifacts`.
    ///
    /// This method also takes `info`, an optionally-provided deserialization of
    /// the artifacts' compilation metadata section. If this information is not
    /// provided (e.g. it's set to `None`) then the information will be
    /// deserialized from the image of the compilation artifacts. Otherwise it
    /// will be assumed to be what would otherwise happen if the section were to
    /// be deserialized.
    ///
    /// The `profiler` argument here is used to inform JIT profiling runtimes
    /// about new code that is loaded.
    pub fn from_artifacts(
        artifacts: CompilationArtifacts,
        info: Option<CompiledModuleInfo>,
        profiler: &dyn ProfilingAgent,
    ) -> Result<Arc<Self>> {
        let obj = File::parse(&artifacts.obj[..])
            .with_context(|| "failed to parse internal ELF compilation artifact")?;

        let section = |name: &str| {
            obj.section_by_name(name)
                .and_then(|s| s.data().ok())
                .ok_or_else(|| anyhow!("missing section `{}` in compilation artifacts", name))
        };

        // Acquire the `CompiledModuleInfo`, either because it was passed in or
        // by deserializing it from the compiliation image.
        let info = match info {
            Some(info) => info,
            None => bincode::deserialize(section(ELF_WASMTIME_INFO)?)
                .context("failed to deserialize wasmtime module info")?,
        };
        let module = Arc::new(info.module);
        let funcs = info.funcs;
        let wasm_data = subslice_range(section(ELF_WASM_DATA)?, &artifacts.obj);

        // Allocate all of the compiled functions into executable memory,
        // copying over their contents.
        let (code_memory, code_range, finished_functions, trampolines) =
            build_code_memory(&obj, &module).map_err(|message| {
                SetupError::Instantiate(InstantiationError::Resource(anyhow::anyhow!(
                    "failed to build code memory for functions: {}",
                    message
                )))
            })?;

        let finished_functions = FinishedFunctions(finished_functions);
        let start = code_range.0 as usize;
        let end = start + code_range.1;

        let mut ret = Self {
            meta: info.meta,
            funcs,
            module,
            artifacts,
            wasm_data,
            code: Arc::new(ModuleCode {
                range: (start, end),
                code_memory,
                dbg_jit_registration: None,
            }),
            finished_functions,
            trampolines,
        };
        ret.register_debug_and_profiling(profiler)?;

        Ok(Arc::new(ret))
    }

    fn register_debug_and_profiling(&mut self, profiler: &dyn ProfilingAgent) -> Result<()> {
        // Register GDB JIT images; initialize profiler and load the wasm module.
        let dbg_jit_registration = if self.meta.native_debug_info_present {
            let bytes = create_gdbjit_image(
                self.artifacts.obj.to_vec(),
                (
                    self.code.range.0 as *const u8,
                    self.code.range.1 - self.code.range.0,
                ),
            )
            .map_err(SetupError::DebugInfo)?;
            profiler.module_load(self, Some(&bytes));
            let reg = GdbJitImageRegistration::register(bytes);
            Some(reg)
        } else {
            profiler.module_load(self, None);
            None
        };

        Arc::get_mut(&mut self.code).unwrap().dbg_jit_registration = dbg_jit_registration;
        Ok(())
    }

    /// Extracts `CompilationArtifacts` from the compiled module.
    pub fn compilation_artifacts(&self) -> &CompilationArtifacts {
        &self.artifacts
    }

    /// Returns the concatenated list of all data associated with this wasm
    /// module.
    ///
    /// This is used for initialization of memories and all data ranges stored
    /// in a `Module` are relative to the slice returned here.
    pub fn wasm_data(&self) -> &[u8] {
        &self.artifacts.obj[self.wasm_data.clone()]
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
    #[inline]
    pub fn finished_functions(&self) -> &PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]> {
        &self.finished_functions.0
    }

    /// Returns the per-signature trampolines for this module.
    pub fn trampolines(&self) -> &[(SignatureIndex, VMTrampoline)] {
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
        self.finished_functions()
            .values()
            .copied()
            .zip(self.funcs.values().map(|f| f.stack_maps.as_slice()))
    }

    /// Lookups a defined function by a program counter value.
    ///
    /// Returns the defined function index, the start address, and the end address (exclusive).
    pub fn func_by_pc(&self, pc: usize) -> Option<(DefinedFuncIndex, usize, usize)> {
        let functions = self.finished_functions();

        let index = match functions.binary_search_values_by_key(&pc, |body| unsafe {
            debug_assert!(!(**body).is_empty());
            // Return the inclusive "end" of the function
            (**body).as_ptr() as usize + (**body).len() - 1
        }) {
            Ok(k) => {
                // Exact match, pc is at the end of this function
                k
            }
            Err(k) => {
                // Not an exact match, k is where `pc` would be "inserted"
                // Since we key based on the end, function `k` might contain `pc`,
                // so we'll validate on the range check below
                k
            }
        };

        let body = functions.get(index)?;
        let (start, end) = unsafe {
            let ptr = (**body).as_ptr();
            let len = (**body).len();
            (ptr as usize, ptr as usize + len)
        };

        if pc < start || end < pc {
            return None;
        }

        Some((index, start, end))
    }

    /// Gets the function information for a given function index.
    pub fn func_info(&self, index: DefinedFuncIndex) -> &FunctionInfo {
        self.funcs
            .get(index)
            .expect("defined function should be present")
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
    pub fn symbolize_context(&self) -> Result<Option<SymbolizeContext<'_>>> {
        use gimli::EndianSlice;
        if !self.meta.has_wasm_debuginfo {
            return Ok(None);
        }
        let obj = File::parse(&self.artifacts.obj[..])
            .context("failed to parse internal ELF file representation")?;
        let dwarf = gimli::Dwarf::load(|id| -> Result<_> {
            let data = obj
                .section_by_name(wasm_section_name(id))
                .and_then(|s| s.data().ok())
                .unwrap_or(&[]);
            Ok(EndianSlice::new(data, gimli::LittleEndian))
        })?;
        let cx = addr2line::Context::from_dwarf(dwarf)
            .context("failed to create addr2line dwarf mapping context")?;
        Ok(Some(SymbolizeContext {
            inner: cx,
            code_section_offset: self.meta.code_section_offset,
        }))
    }

    /// Returns whether the original wasm module had unparsed debug information
    /// based on the tunables configuration.
    pub fn has_unparsed_debuginfo(&self) -> bool {
        self.meta.has_unparsed_debuginfo
    }
}

type Addr2LineContext<'a> = addr2line::Context<gimli::EndianSlice<'a, gimli::LittleEndian>>;

/// A context which contains dwarf debug information to translate program
/// counters back to filenames and line numbers.
pub struct SymbolizeContext<'a> {
    inner: Addr2LineContext<'a>,
    code_section_offset: u64,
}

impl<'a> SymbolizeContext<'a> {
    /// Returns access to the [`addr2line::Context`] which can be used to query
    /// frame information with.
    pub fn addr2line(&self) -> &Addr2LineContext<'a> {
        &self.inner
    }

    /// Returns the offset of the code section in the original wasm file, used
    /// to calculate lookup values into the DWARF.
    pub fn code_section_offset(&self) -> u64 {
        self.code_section_offset
    }
}

fn build_code_memory(
    obj: &File,
    module: &Module,
) -> Result<(
    CodeMemory,
    (*const u8, usize),
    PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
    Vec<(SignatureIndex, VMTrampoline)>,
)> {
    let mut code_memory = CodeMemory::new();

    let allocation = code_memory.allocate_for_object(obj)?;

    // Populate the finished functions from the allocation
    let mut finished_functions = PrimaryMap::with_capacity(allocation.funcs_len());
    for (i, fat_ptr) in allocation.funcs() {
        let start = fat_ptr.as_ptr() as usize;
        let fat_ptr: *mut [VMFunctionBody] = fat_ptr;
        // Assert that the function bodies are pushed in sort order
        // This property is relied upon to search for functions by PC values
        assert!(
            start
                > finished_functions
                    .last()
                    .map(|f: &*mut [VMFunctionBody]| unsafe { (**f).as_ptr() as usize })
                    .unwrap_or(0)
        );
        assert_eq!(
            Some(finished_functions.push(fat_ptr)),
            module.defined_func_index(i)
        );
    }

    // Populate the trampolines from the allocation
    let mut trampolines = Vec::with_capacity(allocation.trampolines_len());
    for (i, fat_ptr) in allocation.trampolines() {
        let fnptr =
            unsafe { std::mem::transmute::<*const VMFunctionBody, VMTrampoline>(fat_ptr.as_ptr()) };
        trampolines.push((i, fnptr));
    }

    link_module(obj, allocation.code_range);

    let code_range = (allocation.code_range.as_ptr(), allocation.code_range.len());

    // Make all code compiled thus far executable.
    code_memory.publish();

    Ok((code_memory, code_range, finished_functions, trampolines))
}

/// Returns the range of `inner` within `outer`, such that `outer[range]` is the
/// same as `inner`.
///
/// This method requires that `inner` is a sub-slice of `outer`, and if that
/// isn't true then this method will panic.
fn subslice_range(inner: &[u8], outer: &[u8]) -> Range<usize> {
    if inner.len() == 0 {
        return 0..0;
    }

    assert!(outer.as_ptr() <= inner.as_ptr());
    assert!((&inner[inner.len() - 1] as *const _) <= (&outer[outer.len() - 1] as *const _));

    let start = inner.as_ptr() as usize - outer.as_ptr() as usize;
    start..start + inner.len()
}

/// Returns the Wasmtime-specific section name for dwarf debugging sections.
///
/// These sections, if configured in Wasmtime, will contain the original raw
/// dwarf debugging information found in the wasm file, unmodified. These tables
/// are then consulted later to convert wasm program counters to original wasm
/// source filenames/line numbers with `addr2line`.
fn wasm_section_name(id: gimli::SectionId) -> &'static str {
    use gimli::SectionId::*;
    match id {
        DebugAbbrev => ".debug_abbrev.wasm",
        DebugAddr => ".debug_addr.wasm",
        DebugAranges => ".debug_aranges.wasm",
        DebugFrame => ".debug_frame.wasm",
        EhFrame => ".eh_frame.wasm",
        EhFrameHdr => ".eh_frame_hdr.wasm",
        DebugInfo => ".debug_info.wasm",
        DebugLine => ".debug_line.wasm",
        DebugLineStr => ".debug_line_str.wasm",
        DebugLoc => ".debug_loc.wasm",
        DebugLocLists => ".debug_loc_lists.wasm",
        DebugMacinfo => ".debug_macinfo.wasm",
        DebugMacro => ".debug_macro.wasm",
        DebugPubNames => ".debug_pub_names.wasm",
        DebugPubTypes => ".debug_pub_types.wasm",
        DebugRanges => ".debug_ranges.wasm",
        DebugRngLists => ".debug_rng_lists.wasm",
        DebugStr => ".debug_str.wasm",
        DebugStrOffsets => ".debug_str_offsets.wasm",
        DebugTypes => ".debug_types.wasm",
    }
}
