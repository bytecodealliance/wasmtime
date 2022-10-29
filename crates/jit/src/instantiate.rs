//! Define the `instantiate` function, which takes a byte array containing an
//! encoded wasm module and returns a live wasm instance. Also, define
//! `CompiledModule` to allow compiling and instantiating to be done as separate
//! steps.

use crate::code_memory::CodeMemory;
use crate::debug::create_gdbjit_image;
use crate::ProfilingAgent;
use anyhow::{anyhow, bail, Context, Error, Result};
use object::write::{Object, StandardSegment, WritableBuffer};
use object::{File, Object as _, ObjectSection, SectionKind};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::ops::Range;
use std::str;
use std::sync::Arc;
use thiserror::Error;
use wasmtime_environ::{
    CompileError, DefinedFuncIndex, FuncIndex, FunctionInfo, Module, ModuleTranslation, PrimaryMap,
    SignatureIndex, StackMapInformation, Trampoline, Tunables, ELF_WASMTIME_ADDRMAP,
    ELF_WASMTIME_TRAPS,
};
use wasmtime_runtime::{
    CompiledModuleId, CompiledModuleIdAllocator, GdbJitImageRegistration, InstantiationError,
    MmapVec, VMFunctionBody, VMTrampoline,
};

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

/// This is the name of the section in the final ELF image which contains a
/// concatenated list of all function names.
///
/// This section is optionally included in the final artifact depending on
/// whether the wasm module has any name data at all (or in the future if we add
/// an option to not preserve name data). This section is a concatenated list of
/// strings where `CompiledModuleInfo::func_names` stores offsets/lengths into
/// this section.
///
/// Note that the goal of this section is to avoid having to decode names at
/// module-load time if we can. Names are typically only used for debugging or
/// things like backtraces so there's no need to eagerly load all of them. By
/// storing the data in a separate section the hope is that the data, which is
/// sometimes quite large (3MB seen for spidermonkey-compiled-to-wasm), can be
/// paged in lazily from an mmap and is never paged in if we never reference it.
const ELF_NAME_DATA: &'static str = ".name.wasm";

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

    /// Sorted list, by function index, of names we have for this module.
    func_names: Vec<FunctionName>,

    /// The trampolines compiled into the text section and their start/length
    /// relative to the start of the text section.
    trampolines: Vec<Trampoline>,

    /// General compilation metadata.
    meta: Metadata,
}

#[derive(Serialize, Deserialize)]
struct FunctionName {
    idx: FuncIndex,
    offset: u32,
    len: u32,
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
        data_align,
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
    for (i, data) in data.iter().enumerate() {
        // The first data segment has its alignment specified as the alignment
        // for the entire section, but everything afterwards is adjacent so it
        // has alignment of 1.
        let align = if i == 0 { data_align.unwrap_or(1) } else { 1 };
        obj.append_section_data(data_id, data, align);
        total_data_len += data.len();
    }
    for data in passive_data.iter() {
        obj.append_section_data(data_id, data, 1);
    }

    // If any names are present in the module then the `ELF_NAME_DATA` section
    // is create and appended.
    let mut func_names = Vec::new();
    if debuginfo.name_section.func_names.len() > 0 {
        let name_id = obj.add_section(
            obj.segment_name(StandardSegment::Data).to_vec(),
            ELF_NAME_DATA.as_bytes().to_vec(),
            SectionKind::ReadOnlyData,
        );
        let mut sorted_names = debuginfo.name_section.func_names.iter().collect::<Vec<_>>();
        sorted_names.sort_by_key(|(idx, _name)| *idx);
        for (idx, name) in sorted_names {
            let offset = obj.append_section_data(name_id, name.as_bytes(), 1);
            let offset = match u32::try_from(offset) {
                Ok(offset) => offset,
                Err(_) => bail!("name section too large (> 4gb)"),
            };
            let len = u32::try_from(name.len()).unwrap();
            func_names.push(FunctionName {
                idx: *idx,
                offset,
                len,
            });
        }
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
        func_names,
        meta: Metadata {
            native_debug_info_present: tunables.generate_native_debuginfo,
            has_unparsed_debuginfo,
            code_section_offset: debuginfo.wasm_file.code_section_offset,
            has_wasm_debuginfo: tunables.parse_wasm_debuginfo,
        },
    };
    bincode::serialize_into(&mut bytes, &info)?;
    obj.append_section_data(info_id, &bytes, 1);

    return Ok((mmap_vec_from_obj(obj)?, info));

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
            format!("{}.wasm", T::id().name()).into_bytes(),
            SectionKind::Debug,
        );
        obj.append_section_data(section_id, data, 1);
    }
}

/// Creates a new `MmapVec` from serializing the specified `obj`.
///
/// The returned `MmapVec` will contain the serialized version of `obj` and
/// is sized appropriately to the exact size of the object serialized.
pub fn mmap_vec_from_obj(obj: Object) -> Result<MmapVec> {
    let mut result = ObjectMmap::default();
    return match obj.emit(&mut result) {
        Ok(()) => {
            assert!(result.mmap.is_some(), "no reserve");
            let mmap = result.mmap.expect("reserve not called");
            assert_eq!(mmap.len(), result.len);
            Ok(mmap)
        }
        Err(e) => match result.err.take() {
            Some(original) => Err(original.context(e)),
            None => Err(e.into()),
        },
    };

    /// Helper struct to implement the `WritableBuffer` trait from the `object`
    /// crate.
    ///
    /// This enables writing an object directly into an mmap'd memory so it's
    /// immediately usable for execution after compilation. This implementation
    /// relies on a call to `reserve` happening once up front with all the needed
    /// data, and the mmap internally does not attempt to grow afterwards.
    #[derive(Default)]
    struct ObjectMmap {
        mmap: Option<MmapVec>,
        len: usize,
        err: Option<Error>,
    }

    impl WritableBuffer for ObjectMmap {
        fn len(&self) -> usize {
            self.len
        }

        fn reserve(&mut self, additional: usize) -> Result<(), ()> {
            assert!(self.mmap.is_none(), "cannot reserve twice");
            self.mmap = match MmapVec::with_capacity(additional) {
                Ok(mmap) => Some(mmap),
                Err(e) => {
                    self.err = Some(e);
                    return Err(());
                }
            };
            Ok(())
        }

        fn resize(&mut self, new_len: usize) {
            // Resizing always appends 0 bytes and since new mmaps start out as 0
            // bytes we don't actually need to do anything as part of this other
            // than update our own length.
            if new_len <= self.len {
                return;
            }
            self.len = new_len;
        }

        fn write_bytes(&mut self, val: &[u8]) {
            let mmap = self.mmap.as_mut().expect("write before reserve");
            mmap[self.len..][..val.len()].copy_from_slice(val);
            self.len += val.len();
        }
    }
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
    /// A unique ID used to register this module with the engine.
    unique_id: CompiledModuleId,
    func_names: Vec<FunctionName>,
    func_name_data: Range<usize>,
    /// Map of dwarf sections indexed by `gimli::SectionId` which points to the
    /// range within `code_memory`'s mmap as to the contents of the section.
    dwarf_sections: Vec<Range<usize>>,
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
        mut info: Option<CompiledModuleInfo>,
        profiler: &dyn ProfilingAgent,
        id_allocator: &CompiledModuleIdAllocator,
    ) -> Result<Self> {
        use gimli::SectionId::*;

        // Parse the `code_memory` as an object file and extract information
        // about where all of its sections are located, stored into the
        // `CompiledModule` created here.
        //
        // Note that dwarf sections here specifically are those that are carried
        // over directly from the original wasm module's dwarf sections, not the
        // wasmtime-generated host DWARF sections.
        let obj = File::parse(&mmap[..]).context("failed to parse internal elf file")?;
        let mut wasm_data = None;
        let mut address_map_data = None;
        let mut func_name_data = None;
        let mut trap_data = None;
        let mut code = None;
        let mut dwarf_sections = Vec::new();
        for section in obj.sections() {
            let name = section.name()?;
            let data = section.data()?;
            let range = subslice_range(data, &mmap);
            let mut gimli = |id: gimli::SectionId| {
                let idx = id as usize;
                if dwarf_sections.len() <= idx {
                    dwarf_sections.resize(idx + 1, 0..0);
                }
                dwarf_sections[idx] = range.clone();
            };

            match name {
                ELF_WASM_DATA => wasm_data = Some(range),
                ELF_WASMTIME_ADDRMAP => address_map_data = Some(range),
                ELF_WASMTIME_TRAPS => trap_data = Some(range),
                ELF_NAME_DATA => func_name_data = Some(range),
                ".text" => code = Some(range),

                // Parse the metadata if it's not already available
                // in-memory.
                ELF_WASMTIME_INFO => {
                    if info.is_none() {
                        info = Some(
                            bincode::deserialize(data)
                                .context("failed to deserialize wasmtime module info")?,
                        );
                    }
                }

                // Register dwarf sections into the `dwarf_sections`
                // array which is indexed by `gimli::SectionId`
                ".debug_abbrev.wasm" => gimli(DebugAbbrev),
                ".debug_addr.wasm" => gimli(DebugAddr),
                ".debug_aranges.wasm" => gimli(DebugAranges),
                ".debug_frame.wasm" => gimli(DebugFrame),
                ".eh_frame.wasm" => gimli(EhFrame),
                ".eh_frame_hdr.wasm" => gimli(EhFrameHdr),
                ".debug_info.wasm" => gimli(DebugInfo),
                ".debug_line.wasm" => gimli(DebugLine),
                ".debug_line_str.wasm" => gimli(DebugLineStr),
                ".debug_loc.wasm" => gimli(DebugLoc),
                ".debug_loc_lists.wasm" => gimli(DebugLocLists),
                ".debug_macinfo.wasm" => gimli(DebugMacinfo),
                ".debug_macro.wasm" => gimli(DebugMacro),
                ".debug_pub_names.wasm" => gimli(DebugPubNames),
                ".debug_pub_types.wasm" => gimli(DebugPubTypes),
                ".debug_ranges.wasm" => gimli(DebugRanges),
                ".debug_rng_lists.wasm" => gimli(DebugRngLists),
                ".debug_str.wasm" => gimli(DebugStr),
                ".debug_str_offsets.wasm" => gimli(DebugStrOffsets),
                ".debug_types.wasm" => gimli(DebugTypes),
                ".debug_cu_index.wasm" => gimli(DebugCuIndex),
                ".debug_tu_index.wasm" => gimli(DebugTuIndex),

                _ => log::debug!("ignoring section {name}"),
            }
        }

        let info = info.ok_or_else(|| anyhow!("failed to find wasm info section"))?;

        let mut ret = Self {
            module: Arc::new(info.module),
            funcs: info.funcs,
            trampolines: info.trampolines,
            wasm_data: wasm_data.ok_or_else(|| anyhow!("missing wasm data section"))?,
            address_map_data: address_map_data.unwrap_or(0..0),
            func_name_data: func_name_data.unwrap_or(0..0),
            trap_data: trap_data.ok_or_else(|| anyhow!("missing trap data section"))?,
            code: code.ok_or_else(|| anyhow!("missing code section"))?,
            dbg_jit_registration: None,
            code_memory: CodeMemory::new(mmap),
            meta: info.meta,
            unique_id: id_allocator.alloc(),
            func_names: info.func_names,
            dwarf_sections,
        };
        ret.code_memory
            .publish()
            .context("failed to publish code memory")?;
        ret.register_debug_and_profiling(profiler)?;

        Ok(ret)
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

    /// Get this module's unique ID. It is unique with respect to a
    /// single allocator (which is ordinarily held on a Wasm engine).
    pub fn unique_id(&self) -> CompiledModuleId {
        self.unique_id
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

    /// Looks up the `name` section name for the function index `idx`, if one
    /// was specified in the original wasm module.
    pub fn func_name(&self, idx: FuncIndex) -> Option<&str> {
        // Find entry for `idx`, if present.
        let i = self.func_names.binary_search_by_key(&idx, |n| n.idx).ok()?;
        let name = &self.func_names[i];

        // Here we `unwrap` the `from_utf8` but this can theoretically be a
        // `from_utf8_unchecked` if we really wanted since this section is
        // guaranteed to only have valid utf-8 data. Until it's a problem it's
        // probably best to double-check this though.
        let data = &self.mmap()[self.func_name_data.clone()];
        Some(str::from_utf8(&data[name.offset as usize..][..name.len as usize]).unwrap())
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
        let dwarf = gimli::Dwarf::load(|id| -> Result<_> {
            let range = self
                .dwarf_sections
                .get(id as usize)
                .cloned()
                .unwrap_or(0..0);
            let data = &self.mmap()[range];
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

    /// Returns the bounds, in host memory, of where this module's compiled
    /// image resides.
    pub fn image_range(&self) -> Range<usize> {
        let base = self.mmap().as_ptr() as usize;
        let len = self.mmap().len();
        base..base + len
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
