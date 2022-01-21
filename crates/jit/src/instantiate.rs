//! Define the `instantiate` function, which takes a byte array containing an
//! encoded wasm module and returns a live wasm instance. Also, define
//! `CompiledModule` to allow compiling and instantiating to be done as separate
//! steps.

use crate::code_memory::CodeMemory;
use crate::debug::create_gdbjit_image;
use crate::{MmapVec, ProfilingAgent};
use anyhow::{anyhow, Context, Result};
use object::write::{Object, StandardSegment};
use object::{File, Object as _, ObjectSection, SectionKind};
use serde::{Deserialize, Serialize};
use std::ops::Range;
use std::sync::Arc;
use thiserror::Error;
use wasmtime_environ::{
    CompileError, DefinedFuncIndex, FunctionInfo, InstanceSignature, InstanceTypeIndex, Module,
    ModuleSignature, ModuleTranslation, ModuleTypeIndex, PrimaryMap, SignatureIndex,
    StackMapInformation, Trampoline, Tunables, WasmFuncType, ELF_WASMTIME_ADDRMAP,
    ELF_WASMTIME_TRAPS,
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

    /// The trampolines compiled into the text section and their start/length
    /// relative to the start of the text section.
    trampolines: Vec<Trampoline>,

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

/// Finishes compilation of the `translation` specified, producing the final
/// compilation artifact and auxiliary information.
///
/// This function will consume the final results of compiling a wasm module
/// and finish the ELF image in-progress as part of `obj` by appending any
/// compiler-agnostic sections.
///
/// The auxiliary `CompiledModuleInfo` structure returned here has also been
/// serialized into the object returned, but if the caller will quickly
/// turn-around and invoke `CompiledModule::from_artifacts` after this then the
/// information can be passed to that method to avoid extra deserialization.
/// This is done to avoid a serialize-then-deserialize for API calls like
/// `Module::new` where the compiled module is immediately going to be used.
///
/// The `MmapVec` returned here contains the compiled image and resides in
/// mmap'd memory for easily switching permissions to executable afterwards.
pub fn finish_compile(
    translation: ModuleTranslation<'_>,
    mut obj: Object,
    funcs: PrimaryMap<DefinedFuncIndex, FunctionInfo>,
    trampolines: Vec<Trampoline>,
    tunables: &Tunables,
) -> Result<(MmapVec, CompiledModuleInfo)> {
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
        trampolines,
        meta: Metadata {
            native_debug_info_present: tunables.generate_native_debuginfo,
            has_unparsed_debuginfo,
            code_section_offset: debuginfo.wasm_file.code_section_offset,
            has_wasm_debuginfo: tunables.parse_wasm_debuginfo,
        },
    };
    bincode::serialize_into(&mut bytes, &info)?;
    obj.append_section_data(info_id, &bytes, 1);

    return Ok((MmapVec::from_obj(obj)?, info));

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

/// This is intended to mirror the type tables in `wasmtime_environ`, except that
/// it doesn't store the native signatures which are no longer needed past compilation.
#[derive(Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct TypeTables {
    pub wasm_signatures: PrimaryMap<SignatureIndex, WasmFuncType>,
    pub module_signatures: PrimaryMap<ModuleTypeIndex, ModuleSignature>,
    pub instance_signatures: PrimaryMap<InstanceTypeIndex, InstanceSignature>,
}

/// A compiled wasm module, ready to be instantiated.
pub struct CompiledModule {
    wasm_data: Range<usize>,
    address_map_data: Range<usize>,
    trap_data: Range<usize>,
    module: Arc<Module>,
    funcs: PrimaryMap<DefinedFuncIndex, FunctionInfo>,
    trampolines: Vec<Trampoline>,
    meta: Metadata,
    code: Range<usize>,
    code_memory: CodeMemory,
    dbg_jit_registration: Option<GdbJitImageRegistration>,
}

impl CompiledModule {
    /// Creates `CompiledModule` directly from a precompiled artifact.
    ///
    /// The `mmap` argument is expecte to be the result of a previous call to
    /// `finish_compile` above. This is an ELF image, at this time, which
    /// contains all necessary information to create a `CompiledModule` from a
    /// compilation.
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
        mmap: MmapVec,
        info: Option<CompiledModuleInfo>,
        profiler: &dyn ProfilingAgent,
    ) -> Result<Arc<Self>> {
        // Transfer ownership of `obj` to a `CodeMemory` object which will
        // manage permissions, such as the executable bit. Once it's located
        // there we also publish it for being able to execute. Note that this
        // step will also resolve pending relocations in the compiled image.
        let mut code_memory = CodeMemory::new(mmap);
        let code = code_memory
            .publish()
            .context("failed to publish code memory")?;

        let section = |name: &str| {
            code.obj
                .section_by_name(name)
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

        let mut ret = Self {
            module: Arc::new(info.module),
            funcs: info.funcs,
            trampolines: info.trampolines,
            wasm_data: subslice_range(section(ELF_WASM_DATA)?, code.mmap),
            address_map_data: code
                .obj
                .section_by_name(ELF_WASMTIME_ADDRMAP)
                .and_then(|s| s.data().ok())
                .map(|slice| subslice_range(slice, code.mmap))
                .unwrap_or(0..0),
            trap_data: subslice_range(section(ELF_WASMTIME_TRAPS)?, code.mmap),
            code: subslice_range(code.text, code.mmap),
            dbg_jit_registration: None,
            code_memory,
            meta: info.meta,
        };
        ret.register_debug_and_profiling(profiler)?;

        Ok(Arc::new(ret))
    }

    fn register_debug_and_profiling(&mut self, profiler: &dyn ProfilingAgent) -> Result<()> {
        // Register GDB JIT images; initialize profiler and load the wasm module.
        if self.meta.native_debug_info_present {
            let code = self.code();
            let bytes = create_gdbjit_image(self.mmap().to_vec(), (code.as_ptr(), code.len()))
                .map_err(SetupError::DebugInfo)?;
            profiler.module_load(self, Some(&bytes));
            let reg = GdbJitImageRegistration::register(bytes);
            self.dbg_jit_registration = Some(reg);
        } else {
            profiler.module_load(self, None);
        }
        Ok(())
    }

    /// Returns the underlying memory which contains the compiled module's
    /// image.
    pub fn mmap(&self) -> &MmapVec {
        self.code_memory.mmap()
    }

    /// Returns the concatenated list of all data associated with this wasm
    /// module.
    ///
    /// This is used for initialization of memories and all data ranges stored
    /// in a `Module` are relative to the slice returned here.
    pub fn wasm_data(&self) -> &[u8] {
        &self.mmap()[self.wasm_data.clone()]
    }

    /// Returns the encoded address map section used to pass to
    /// `wasmtime_environ::lookup_file_pos`.
    pub fn address_map_data(&self) -> &[u8] {
        &self.mmap()[self.address_map_data.clone()]
    }

    /// Returns the encoded trap information for this compiled image.
    ///
    /// For more information see `wasmtime_environ::trap_encoding`.
    pub fn trap_data(&self) -> &[u8] {
        &self.mmap()[self.trap_data.clone()]
    }

    /// Returns the text section of the ELF image for this compiled module.
    ///
    /// This memory should have the read/execute permissions.
    pub fn code(&self) -> &[u8] {
        &self.mmap()[self.code.clone()]
    }

    /// Return a reference-counting pointer to a module.
    pub fn module(&self) -> &Arc<Module> {
        &self.module
    }

    /// Returns the `FunctionInfo` map for all defined functions.
    pub fn functions(&self) -> &PrimaryMap<DefinedFuncIndex, FunctionInfo> {
        &self.funcs
    }

    /// Return a reference to a mutable module (if possible).
    pub fn module_mut(&mut self) -> Option<&mut Module> {
        Arc::get_mut(&mut self.module)
    }

    /// Returns the map of all finished JIT functions compiled for this module
    #[inline]
    pub fn finished_functions(
        &self,
    ) -> impl ExactSizeIterator<Item = (DefinedFuncIndex, *const [VMFunctionBody])> + '_ {
        let code = self.code();
        self.funcs.iter().map(move |(i, info)| {
            let func = &code[info.start as usize..][..info.length as usize];
            (
                i,
                std::ptr::slice_from_raw_parts(func.as_ptr().cast::<VMFunctionBody>(), func.len()),
            )
        })
    }

    /// Returns the per-signature trampolines for this module.
    pub fn trampolines(&self) -> impl Iterator<Item = (SignatureIndex, VMTrampoline, usize)> + '_ {
        let code = self.code();
        self.trampolines.iter().map(move |info| {
            (
                info.signature,
                unsafe {
                    let ptr = &code[info.start as usize];
                    std::mem::transmute::<*const u8, VMTrampoline>(ptr)
                },
                info.length as usize,
            )
        })
    }

    /// Returns the stack map information for all functions defined in this
    /// module.
    ///
    /// The iterator returned iterates over the span of the compiled function in
    /// memory with the stack maps associated with those bytes.
    pub fn stack_maps(
        &self,
    ) -> impl Iterator<Item = (*const [VMFunctionBody], &[StackMapInformation])> {
        self.finished_functions()
            .map(|(_, f)| f)
            .zip(self.funcs.values().map(|f| f.stack_maps.as_slice()))
    }

    /// Lookups a defined function by a program counter value.
    ///
    /// Returns the defined function index and the relative address of
    /// `text_offset` within the function itself.
    pub fn func_by_text_offset(&self, text_offset: usize) -> Option<(DefinedFuncIndex, u32)> {
        let text_offset = text_offset as u64;

        let index = match self
            .funcs
            .binary_search_values_by_key(&text_offset, |info| {
                debug_assert!(info.length > 0);
                // Return the inclusive "end" of the function
                info.start + u64::from(info.length) - 1
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

        let body = self.funcs.get(index)?;
        let start = body.start;
        let end = body.start + u64::from(body.length);

        if text_offset < start || end < text_offset {
            return None;
        }

        Some((index, (text_offset - body.start) as u32))
    }

    /// Gets the function information for a given function index.
    pub fn func_info(&self, index: DefinedFuncIndex) -> &FunctionInfo {
        self.funcs
            .get(index)
            .expect("defined function should be present")
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
        let obj = File::parse(&self.mmap()[..])
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

    /// Indicates whether this module came with n address map such that lookups
    /// via `wasmtime_environ::lookup_file_pos` will succeed.
    ///
    /// If this function returns `false` then `lookup_file_pos` will always
    /// return `None`.
    pub fn has_address_map(&self) -> bool {
        !self.address_map_data().is_empty()
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

/// Returns the range of `inner` within `outer`, such that `outer[range]` is the
/// same as `inner`.
///
/// This method requires that `inner` is a sub-slice of `outer`, and if that
/// isn't true then this method will panic.
pub fn subslice_range(inner: &[u8], outer: &[u8]) -> Range<usize> {
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
        DebugCuIndex => ".debug_cu_index.wasm",
        DebugTuIndex => ".debug_tu_index.wasm",
    }
}
