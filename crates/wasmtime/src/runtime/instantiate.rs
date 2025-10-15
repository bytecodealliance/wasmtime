//! Define the `instantiate` function, which takes a byte array containing an
//! encoded wasm module and returns a live wasm instance. Also, define
//! `CompiledModule` to allow compiling and instantiating to be done as separate
//! steps.

use crate::prelude::*;
use crate::runtime::vm::{CompiledModuleId, MmapVec};
use crate::{code_memory::CodeMemory, profiling_agent::ProfilingAgent};
use alloc::sync::Arc;
use core::str;
use wasmtime_environ::{
    BuiltinFunctionIndex, CompiledFunctionsTable, CompiledModuleInfo, DefinedFuncIndex, EntityRef,
    FilePos, FuncIndex, FuncKey, FunctionLoc, FunctionName, Metadata, Module,
    ModuleInternedTypeIndex, StaticModuleIndex,
};

/// A compiled wasm module, ready to be instantiated.
pub struct CompiledModule {
    /// A unique ID used to register this module with the engine.
    unique_id: CompiledModuleId,
    code_memory: Arc<CodeMemory>,
    module: Arc<Module>,
    meta: Metadata,
    index: Arc<CompiledFunctionsTable>,
    /// Sorted list, by function index, of names we have for this module.
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
        index: Arc<CompiledFunctionsTable>,
        profiler: &dyn ProfilingAgent,
    ) -> Result<Self> {
        let mut ret = Self {
            unique_id: CompiledModuleId::new(),
            code_memory,
            module: Arc::new(info.module),
            meta: info.meta,
            index,
            func_names: info.func_names,
        };
        ret.register_profiling(profiler)?;

        Ok(ret)
    }

    fn register_profiling(&mut self, profiler: &dyn ProfilingAgent) -> Result<()> {
        // TODO-Bug?: "code_memory" is not exclusive for this module in the case of components,
        // so we may be registering the same code range multiple times here.
        profiler.register_module(&self.code_memory.mmap()[..], &|addr| {
            let idx = self.func_by_text_offset(addr)?;
            let idx = self.module.func_index(idx);
            let name = self.func_name(idx)?;
            let mut demangled = String::new();
            wasmtime_environ::demangle_function_name(&mut demangled, name).unwrap();
            Some(demangled)
        });
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
    #[inline]
    pub fn text(&self) -> &[u8] {
        self.code_memory.text()
    }

    /// Return a reference-counting pointer to a module.
    pub fn module(&self) -> &Arc<Module> {
        &self.module
    }

    fn module_index(&self) -> StaticModuleIndex {
        self.module.module_index
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

    /// Returns an iterator over all functions defined within this module with
    /// their index and their body in memory.
    #[inline]
    pub fn finished_functions(
        &self,
    ) -> impl ExactSizeIterator<Item = (DefinedFuncIndex, &[u8])> + '_ {
        self.module
            .defined_func_indices()
            .map(|i| (i, self.finished_function(i)))
    }

    /// Returns the body of the function that `index` points to.
    #[inline]
    pub fn finished_function(&self, def_func_index: DefinedFuncIndex) -> &[u8] {
        let loc = self.func_loc(def_func_index);
        &self.text()[loc.start as usize..][..loc.length as usize]
    }

    /// Get the array-to-Wasm trampoline for the function `index` points to.
    ///
    /// If the function `index` points to does not escape, then `None` is
    /// returned.
    ///
    /// These trampolines are used for array callers (e.g. `Func::new`)
    /// calling Wasm callees.
    pub fn array_to_wasm_trampoline(&self, def_func_index: DefinedFuncIndex) -> Option<&[u8]> {
        assert!(def_func_index.index() < self.module.num_defined_funcs());
        let key = FuncKey::ArrayToWasmTrampoline(self.module_index(), def_func_index);
        let loc = self.index.func_loc(key)?;
        Some(&self.text()[loc.start as usize..][..loc.length as usize])
    }

    /// Get the Wasm-to-array trampoline for the given signature.
    ///
    /// These trampolines are used for filling in
    /// `VMFuncRef::wasm_call` for `Func::wrap`-style host funcrefs
    /// that don't have access to a compiler when created.
    pub fn wasm_to_array_trampoline(&self, signature: ModuleInternedTypeIndex) -> Option<&[u8]> {
        let key = FuncKey::WasmToArrayTrampoline(signature);
        let loc = self.index.func_loc(key)?;
        Some(&self.text()[loc.start as usize..][..loc.length as usize])
    }

    /// Get the Wasm-to-builtin trampoline for the given builtin function.
    ///
    /// These trampolines are ordinarily invoked only from native
    /// compiled Wasm code. However, in certain cases (e.g. when
    /// synthesizing a hostcall upon a signal) we may need the address
    /// as well.
    pub fn wasm_to_builtin_trampoline(&self, builtin: BuiltinFunctionIndex) -> Option<&[u8]> {
        let key = FuncKey::WasmToBuiltinTrampoline(builtin);
        let loc = self.index.func_loc(key)?;
        Some(&self.text()[loc.start as usize..][..loc.length as usize])
    }

    /// Lookups a defined function by a program counter value.
    ///
    /// Returns the defined function index and the relative address of
    /// `text_offset` within the function itself.
    pub fn func_by_text_offset(&self, text_offset: usize) -> Option<DefinedFuncIndex> {
        let text_offset = u32::try_from(text_offset).unwrap();
        let key = self.index.func_by_text_offset(text_offset)?;
        match key {
            FuncKey::DefinedWasmFunction(module, def_func_index) => {
                debug_assert_eq!(module, self.module_index());
                Some(def_func_index)
            }
            _ => None,
        }
    }

    /// Gets the function location information for a given function index.
    pub fn func_loc(&self, def_func_index: DefinedFuncIndex) -> &FunctionLoc {
        assert!(def_func_index.index() < self.module.num_defined_funcs());
        let key = FuncKey::DefinedWasmFunction(self.module_index(), def_func_index);
        self.index
            .func_loc(key)
            .expect("defined function should be present")
    }

    /// Returns the original binary offset in the file that `index` was defined
    /// at.
    pub fn func_start_srcloc(&self, def_func_index: DefinedFuncIndex) -> FilePos {
        assert!(def_func_index.index() < self.module.num_defined_funcs());
        let key = FuncKey::DefinedWasmFunction(self.module_index(), def_func_index);
        self.index
            .src_loc(key)
            .expect("defined function should be present")
    }

    /// Creates a new symbolication context which can be used to further
    /// symbolicate stack traces.
    ///
    /// Basically this makes a thing which parses debuginfo and can tell you
    /// what filename and line number a wasm pc comes from.
    #[cfg(feature = "addr2line")]
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
                .ok()
                .and_then(|i| {
                    let (_, range) = &self.meta.dwarf[i];
                    let start = range.start.try_into().ok()?;
                    let end = range.end.try_into().ok()?;
                    self.code_memory().wasm_dwarf().get(start..end)
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
}

#[cfg(feature = "addr2line")]
type Addr2LineContext<'a> = addr2line::Context<gimli::EndianSlice<'a, gimli::LittleEndian>>;

/// A context which contains dwarf debug information to translate program
/// counters back to filenames and line numbers.
#[cfg(feature = "addr2line")]
pub struct SymbolizeContext<'a> {
    inner: Addr2LineContext<'a>,
    code_section_offset: u64,
}

#[cfg(feature = "addr2line")]
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
