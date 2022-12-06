//! Define the `instantiate` function, which takes a byte array containing an
//! encoded wasm module and returns a live wasm instance. Also, define
//! `CompiledModule` to allow compiling and instantiating to be done as separate
//! steps.

use crate::code_memory::CodeMemory;
use crate::debug::create_gdbjit_image;
use crate::ProfilingAgent;
use anyhow::{bail, Context, Error, Result};
use object::write::{Object, SectionId, StandardSegment, WritableBuffer};
use object::SectionKind;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::ops::Range;
use std::str;
use std::sync::Arc;
use wasmtime_environ::obj;
use wasmtime_environ::{
    DefinedFuncIndex, FuncIndex, FunctionLoc, MemoryInitialization, Module, ModuleTranslation,
    PrimaryMap, SignatureIndex, StackMapInformation, Tunables, WasmFunctionInfo,
};
use wasmtime_runtime::{
    CompiledModuleId, CompiledModuleIdAllocator, GdbJitImageRegistration, MmapVec, VMTrampoline,
};

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
    pub trampolines: Vec<(SignatureIndex, FunctionLoc)>,

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

    /// Dwarf sections and the offsets at which they're stored in the
    /// ELF_WASMTIME_DWARF
    dwarf: Vec<(u8, Range<u64>)>,
}

/// Helper structure to create an ELF file as a compilation artifact.
///
/// This structure exposes the process which Wasmtime will encode a core wasm
/// module into an ELF file, notably managing data sections and all that good
/// business going into the final file.
pub struct ObjectBuilder<'a> {
    /// The `object`-crate-defined ELF file write we're using.
    obj: Object<'a>,

    /// General compilation configuration.
    tunables: &'a Tunables,

    /// The section identifier for "rodata" which is where wasm data segments
    /// will go.
    data: SectionId,

    /// The section identifier for function name information, or otherwise where
    /// the `name` custom section of wasm is copied into.
    ///
    /// This is optional and lazily created on demand.
    names: Option<SectionId>,

    /// The section identifier for dwarf information copied from the original
    /// wasm files.
    ///
    /// This is optional and lazily created on demand.
    dwarf: Option<SectionId>,
}

impl<'a> ObjectBuilder<'a> {
    /// Creates a new builder for the `obj` specified.
    pub fn new(mut obj: Object<'a>, tunables: &'a Tunables) -> ObjectBuilder<'a> {
        let data = obj.add_section(
            obj.segment_name(StandardSegment::Data).to_vec(),
            obj::ELF_WASM_DATA.as_bytes().to_vec(),
            SectionKind::ReadOnlyData,
        );
        ObjectBuilder {
            obj,
            tunables,
            data,
            names: None,
            dwarf: None,
        }
    }

    /// Completes compilation of the `translation` specified, inserting
    /// everything necessary into the `Object` being built.
    ///
    /// This function will consume the final results of compiling a wasm module
    /// and finish the ELF image in-progress as part of `self.obj` by appending
    /// any compiler-agnostic sections.
    ///
    /// The auxiliary `CompiledModuleInfo` structure returned here has also been
    /// serialized into the object returned, but if the caller will quickly
    /// turn-around and invoke `CompiledModule::from_artifacts` after this then
    /// the information can be passed to that method to avoid extra
    /// deserialization. This is done to avoid a serialize-then-deserialize for
    /// API calls like `Module::new` where the compiled module is immediately
    /// going to be used.
    ///
    /// The various arguments here are:
    ///
    /// * `translation` - the core wasm translation that's being completed.
    ///
    /// * `funcs` - compilation metadata about functions within the translation
    ///   as well as where the functions are located in the text section.
    ///
    /// * `trampolines` - list of all trampolines necessary for this module
    ///   and where they're located in the text section.
    ///
    /// Returns the `CompiledModuleInfo` corresopnding to this core wasm module
    /// as a result of this append operation. This is then serialized into the
    /// final artifact by the caller.
    pub fn append(
        &mut self,
        translation: ModuleTranslation<'_>,
        funcs: PrimaryMap<DefinedFuncIndex, (WasmFunctionInfo, FunctionLoc)>,
        trampolines: Vec<(SignatureIndex, FunctionLoc)>,
    ) -> Result<CompiledModuleInfo> {
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
        // source of the data later at runtime. This additionally keeps track of
        // the offset of
        let mut total_data_len = 0;
        let data_offset = self
            .obj
            .append_section_data(self.data, &[], data_align.unwrap_or(1));
        for (i, data) in data.iter().enumerate() {
            // The first data segment has its alignment specified as the alignment
            // for the entire section, but everything afterwards is adjacent so it
            // has alignment of 1.
            let align = if i == 0 { data_align.unwrap_or(1) } else { 1 };
            self.obj.append_section_data(self.data, data, align);
            total_data_len += data.len();
        }
        for data in passive_data.iter() {
            self.obj.append_section_data(self.data, data, 1);
        }

        // If any names are present in the module then the `ELF_NAME_DATA` section
        // is create and appended.
        let mut func_names = Vec::new();
        if debuginfo.name_section.func_names.len() > 0 {
            let name_id = *self.names.get_or_insert_with(|| {
                self.obj.add_section(
                    self.obj.segment_name(StandardSegment::Data).to_vec(),
                    obj::ELF_NAME_DATA.as_bytes().to_vec(),
                    SectionKind::ReadOnlyData,
                )
            });
            let mut sorted_names = debuginfo.name_section.func_names.iter().collect::<Vec<_>>();
            sorted_names.sort_by_key(|(idx, _name)| *idx);
            for (idx, name) in sorted_names {
                let offset = self.obj.append_section_data(name_id, name.as_bytes(), 1);
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

        // Data offsets in `MemoryInitialization` are offsets within the
        // `translation.data` list concatenated which is now present in the data
        // segment that's appended to the object. Increase the offsets by
        // `self.data_size` to account for any previously added module.
        let data_offset = u32::try_from(data_offset).unwrap();
        match &mut module.memory_initialization {
            MemoryInitialization::Segmented(list) => {
                for segment in list {
                    segment.data.start = segment.data.start.checked_add(data_offset).unwrap();
                    segment.data.end = segment.data.end.checked_add(data_offset).unwrap();
                }
            }
            MemoryInitialization::Static { map } => {
                for (_, segment) in map {
                    if let Some(segment) = segment {
                        segment.data.start = segment.data.start.checked_add(data_offset).unwrap();
                        segment.data.end = segment.data.end.checked_add(data_offset).unwrap();
                    }
                }
            }
        }

        // Data offsets for passive data are relative to the start of
        // `translation.passive_data` which was appended to the data segment
        // of this object, after active data in `translation.data`. Update the
        // offsets to account prior modules added in addition to active data.
        let data_offset = data_offset + u32::try_from(total_data_len).unwrap();
        for (_, range) in module.passive_data_map.iter_mut() {
            range.start = range.start.checked_add(data_offset).unwrap();
            range.end = range.end.checked_add(data_offset).unwrap();
        }

        // Insert the wasm raw wasm-based debuginfo into the output, if
        // requested. Note that this is distinct from the native debuginfo
        // possibly generated by the native compiler, hence these sections
        // getting wasm-specific names.
        let mut dwarf = Vec::new();
        if self.tunables.parse_wasm_debuginfo {
            self.push_debug(&mut dwarf, &debuginfo.dwarf.debug_abbrev);
            self.push_debug(&mut dwarf, &debuginfo.dwarf.debug_addr);
            self.push_debug(&mut dwarf, &debuginfo.dwarf.debug_aranges);
            self.push_debug(&mut dwarf, &debuginfo.dwarf.debug_info);
            self.push_debug(&mut dwarf, &debuginfo.dwarf.debug_line);
            self.push_debug(&mut dwarf, &debuginfo.dwarf.debug_line_str);
            self.push_debug(&mut dwarf, &debuginfo.dwarf.debug_str);
            self.push_debug(&mut dwarf, &debuginfo.dwarf.debug_str_offsets);
            self.push_debug(&mut dwarf, &debuginfo.debug_ranges);
            self.push_debug(&mut dwarf, &debuginfo.debug_rnglists);
        }
        // Sort this for binary-search-lookup later in `symbolize_context`.
        dwarf.sort_by_key(|(id, _)| *id);

        Ok(CompiledModuleInfo {
            module,
            funcs,
            trampolines,
            func_names,
            meta: Metadata {
                native_debug_info_present: self.tunables.generate_native_debuginfo,
                has_unparsed_debuginfo,
                code_section_offset: debuginfo.wasm_file.code_section_offset,
                has_wasm_debuginfo: self.tunables.parse_wasm_debuginfo,
                dwarf,
            },
        })
    }

    fn push_debug<'b, T>(&mut self, dwarf: &mut Vec<(u8, Range<u64>)>, section: &T)
    where
        T: gimli::Section<gimli::EndianSlice<'b, gimli::LittleEndian>>,
    {
        let data = section.reader().slice();
        if data.is_empty() {
            return;
        }
        let section_id = *self.dwarf.get_or_insert_with(|| {
            self.obj.add_section(
                self.obj.segment_name(StandardSegment::Debug).to_vec(),
                obj::ELF_WASMTIME_DWARF.as_bytes().to_vec(),
                SectionKind::Debug,
            )
        });
        let offset = self.obj.append_section_data(section_id, data, 1);
        dwarf.push((T::id() as u8, offset..offset + data.len() as u64));
    }

    /// Creates the `ELF_WASMTIME_INFO` section from the given serializable data
    /// structure.
    pub fn serialize_info<T>(&mut self, info: &T)
    where
        T: serde::Serialize,
    {
        let section = self.obj.add_section(
            self.obj.segment_name(StandardSegment::Data).to_vec(),
            obj::ELF_WASMTIME_INFO.as_bytes().to_vec(),
            SectionKind::ReadOnlyData,
        );
        let data = bincode::serialize(info).unwrap();
        self.obj.set_section_data(section, data, 1);
    }

    /// Creates a new `MmapVec` from `self.`
    ///
    /// The returned `MmapVec` will contain the serialized version of `self`
    /// and is sized appropriately to the exact size of the object serialized.
    pub fn finish(self) -> Result<MmapVec> {
        let mut result = ObjectMmap::default();
        return match self.obj.emit(&mut result) {
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
    /// The `code_memory` argument is expected to be the result of a previous
    /// call to `ObjectBuilder::finish` above. This is an ELF image, at this
    /// time, which contains all necessary information to create a
    /// `CompiledModule` from a compilation.
    ///
    /// This method also takes `info`, an optionally-provided deserialization
    /// of the artifacts' compilation metadata section. If this information is
    /// not provided then the information will be
    /// deserialized from the image of the compilation artifacts. Otherwise it
    /// will be assumed to be what would otherwise happen if the section were
    /// to be deserialized.
    ///
    /// The `profiler` argument here is used to inform JIT profiling runtimes
    /// about new code that is loaded.
    pub fn from_artifacts(
        code_memory: Arc<CodeMemory>,
        info: CompiledModuleInfo,
        profiler: &dyn ProfilingAgent,
        id_allocator: &CompiledModuleIdAllocator,
    ) -> Result<Self> {
        let mut ret = Self {
            module: Arc::new(info.module),
            funcs: info.funcs,
            trampolines: info.trampolines,
            dbg_jit_registration: None,
            code_memory,
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
                .context("failed to create jit image for gdb")?;
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

    /// Returns an iterator over all functions defined within this module with
    /// their index and their body in memory.
    #[inline]
    pub fn finished_functions(
        &self,
    ) -> impl ExactSizeIterator<Item = (DefinedFuncIndex, &[u8])> + '_ {
        self.funcs
            .iter()
            .map(move |(i, _)| (i, self.finished_function(i)))
    }

    /// Returns the body of the function that `index` points to.
    #[inline]
    pub fn finished_function(&self, index: DefinedFuncIndex) -> &[u8] {
        let (_, loc) = &self.funcs[index];
        &self.text()[loc.start as usize..][..loc.length as usize]
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
    pub fn stack_maps(&self) -> impl Iterator<Item = (&[u8], &[StackMapInformation])> {
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
            // Lookup the `id` in the `dwarf` array prepared for this module
            // during module serialization where it's sorted by the `id` key. If
            // found this is a range within the general module's concatenated
            // dwarf section which is extracted here, otherwise it's just an
            // empty list to represent that it's not present.
            let data = self
                .meta
                .dwarf
                .binary_search_by_key(&(id as u8), |(id, _)| *id)
                .map(|i| {
                    let (_, range) = &self.meta.dwarf[i];
                    &self.code_memory().dwarf()[range.start as usize..range.end as usize]
                })
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
