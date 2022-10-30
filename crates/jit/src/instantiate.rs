//! Define the `instantiate` function, which takes a byte array containing an
//! encoded wasm module and returns a live wasm instance. Also, define
//! `CompiledModule` to allow compiling and instantiating to be done as separate
//! steps.

use crate::code_memory::CodeMemory;
use crate::debug::create_gdbjit_image;
use crate::ProfilingAgent;
use anyhow::{bail, Context, Error, Result};
use object::write::{Object, StandardSegment, WritableBuffer};
use object::SectionKind;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::ops::Range;
use std::str;
use std::sync::Arc;
use thiserror::Error;
use wasmtime_environ::obj;
use wasmtime_environ::{
    CompileError, DefinedFuncIndex, FuncIndex, FunctionLoc, Module, ModuleTranslation, PrimaryMap,
    SignatureIndex, StackMapInformation, Tunables, WasmFunctionInfo,
};
use wasmtime_runtime::{
    CompiledModuleId, CompiledModuleIdAllocator, GdbJitImageRegistration, InstantiationError,
    MmapVec, VMFunctionBody, VMTrampoline,
};

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
    funcs: PrimaryMap<DefinedFuncIndex, (WasmFunctionInfo, FunctionLoc)>,

    /// Sorted list, by function index, of names we have for this module.
    func_names: Vec<FunctionName>,

    /// The trampolines compiled into the text section and their start/length
    /// relative to the start of the text section.
    trampolines: Vec<(SignatureIndex, FunctionLoc)>,

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
    funcs: PrimaryMap<DefinedFuncIndex, (WasmFunctionInfo, FunctionLoc)>,
    trampolines: Vec<(SignatureIndex, FunctionLoc)>,
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
        obj::ELF_WASM_DATA.as_bytes().to_vec(),
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
            obj::ELF_NAME_DATA.as_bytes().to_vec(),
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
        obj::ELF_WASMTIME_INFO.as_bytes().to_vec(),
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
    module: Arc<Module>,
    funcs: PrimaryMap<DefinedFuncIndex, (WasmFunctionInfo, FunctionLoc)>,
    trampolines: Vec<(SignatureIndex, FunctionLoc)>,
    meta: Metadata,
    code_memory: Arc<CodeMemory>,
    dbg_jit_registration: Option<GdbJitImageRegistration>,
    /// A unique ID used to register this module with the engine.
    unique_id: CompiledModuleId,
    func_names: Vec<FunctionName>,
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
        id_allocator: &CompiledModuleIdAllocator,
    ) -> Result<Self> {
        let mut code_memory = CodeMemory::new(mmap)?;
        code_memory
            .publish()
            .context("failed to publish code memory")?;

        let info = match info {
            Some(info) => info,
            None => {
                let section = code_memory.wasmtime_info();
                bincode::deserialize(section)
                    .context("failed to deserialize wasmtime module info")?
            }
        };

        let mut ret = Self {
            module: Arc::new(info.module),
            funcs: info.funcs,
            trampolines: info.trampolines,
            dbg_jit_registration: None,
            code_memory: Arc::new(code_memory),
            meta: info.meta,
            unique_id: id_allocator.alloc(),
            func_names: info.func_names,
        };
        ret.register_debug_and_profiling(profiler)?;

        Ok(ret)
    }

    fn register_debug_and_profiling(&mut self, profiler: &dyn ProfilingAgent) -> Result<()> {
        // Register GDB JIT images; initialize profiler and load the wasm module.
        if self.meta.native_debug_info_present {
            let text = self.text();
            let bytes = create_gdbjit_image(self.mmap().to_vec(), (text.as_ptr(), text.len()))
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

    /// Returns the underlying owned mmap of this compiled image.
    pub fn code_memory(&self) -> &Arc<CodeMemory> {
        &self.code_memory
    }

    /// Returns the text section of the ELF image for this compiled module.
    ///
    /// This memory should have the read/execute permissions.
    pub fn text(&self) -> &[u8] {
        self.code_memory.text()
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
        let data = self.code_memory().func_name_data();
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
        let text = self.text();
        self.funcs.iter().map(move |(i, (_, loc))| {
            let func = &text[loc.start as usize..][..loc.length as usize];
            (
                i,
                std::ptr::slice_from_raw_parts(func.as_ptr().cast::<VMFunctionBody>(), func.len()),
            )
        })
    }

    /// Returns the per-signature trampolines for this module.
    pub fn trampolines(&self) -> impl Iterator<Item = (SignatureIndex, VMTrampoline, usize)> + '_ {
        let text = self.text();
        self.trampolines.iter().map(move |(signature, loc)| {
            (
                *signature,
                unsafe {
                    let ptr = &text[loc.start as usize];
                    std::mem::transmute::<*const u8, VMTrampoline>(ptr)
                },
                loc.length as usize,
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
            .zip(self.funcs.values().map(|f| &f.0.stack_maps[..]))
    }

    /// Lookups a defined function by a program counter value.
    ///
    /// Returns the defined function index and the relative address of
    /// `text_offset` within the function itself.
    pub fn func_by_text_offset(&self, text_offset: usize) -> Option<(DefinedFuncIndex, u32)> {
        let text_offset = u32::try_from(text_offset).unwrap();

        let index = match self
            .funcs
            .binary_search_values_by_key(&text_offset, |(_, loc)| {
                debug_assert!(loc.length > 0);
                // Return the inclusive "end" of the function
                loc.start + loc.length - 1
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

        let (_, loc) = self.funcs.get(index)?;
        let start = loc.start;
        let end = loc.start + loc.length;

        if text_offset < start || end < text_offset {
            return None;
        }

        Some((index, text_offset - loc.start))
    }

    /// Gets the function location information for a given function index.
    pub fn func_loc(&self, index: DefinedFuncIndex) -> &FunctionLoc {
        &self
            .funcs
            .get(index)
            .expect("defined function should be present")
            .1
    }

    /// Gets the function information for a given function index.
    pub fn wasm_func_info(&self, index: DefinedFuncIndex) -> &WasmFunctionInfo {
        &self
            .funcs
            .get(index)
            .expect("defined function should be present")
            .0
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
            let data = self.code_memory().dwarf_section(id);
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
        !self.code_memory.address_map_data().is_empty()
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
