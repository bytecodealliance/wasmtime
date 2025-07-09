//! Defines `JITModule`.

use crate::{
    compiled_blob::CompiledBlob,
    memory::{BranchProtection, JITMemoryProvider, SystemMemoryProvider},
};
use cranelift_codegen::binemit::Reloc;
use cranelift_codegen::isa::{OwnedTargetIsa, TargetIsa};
use cranelift_codegen::settings::Configurable;
use cranelift_codegen::{ir, settings};
use cranelift_control::ControlPlane;
use cranelift_entity::SecondaryMap;
use cranelift_module::{
    DataDescription, DataId, FuncId, Init, Linkage, Module, ModuleDeclarations, ModuleError,
    ModuleReloc, ModuleRelocTarget, ModuleResult,
};
use log::info;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;
use std::io::Write;
use std::ptr;
use target_lexicon::PointerWidth;

const WRITABLE_DATA_ALIGNMENT: u64 = 0x8;
const READONLY_DATA_ALIGNMENT: u64 = 0x1;

/// A builder for `JITModule`.
pub struct JITBuilder {
    isa: OwnedTargetIsa,
    symbols: HashMap<String, SendWrapper<*const u8>>,
    lookup_symbols: Vec<Box<dyn Fn(&str) -> Option<*const u8> + Send>>,
    libcall_names: Box<dyn Fn(ir::LibCall) -> String + Send + Sync>,
    memory: Option<Box<dyn JITMemoryProvider>>,
}

impl JITBuilder {
    /// Create a new `JITBuilder`.
    ///
    /// The `libcall_names` function provides a way to translate `cranelift_codegen`'s `ir::LibCall`
    /// enum to symbols. LibCalls are inserted in the IR as part of the legalization for certain
    /// floating point instructions, and for stack probes. If you don't know what to use for this
    /// argument, use `cranelift_module::default_libcall_names()`.
    pub fn new(
        libcall_names: Box<dyn Fn(ir::LibCall) -> String + Send + Sync>,
    ) -> ModuleResult<Self> {
        Self::with_flags(&[], libcall_names)
    }

    /// Create a new `JITBuilder` with the given flags.
    ///
    /// The `libcall_names` function provides a way to translate `cranelift_codegen`'s `ir::LibCall`
    /// enum to symbols. LibCalls are inserted in the IR as part of the legalization for certain
    /// floating point instructions, and for stack probes. If you don't know what to use for this
    /// argument, use `cranelift_module::default_libcall_names()`.
    pub fn with_flags(
        flags: &[(&str, &str)],
        libcall_names: Box<dyn Fn(ir::LibCall) -> String + Send + Sync>,
    ) -> ModuleResult<Self> {
        let mut flag_builder = settings::builder();
        for (name, value) in flags {
            flag_builder.set(name, value)?;
        }

        // On at least AArch64, "colocated" calls use shorter-range relocations,
        // which might not reach all definitions; we can't handle that here, so
        // we require long-range relocation types.
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "false").unwrap();
        let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
            panic!("host machine is not supported: {msg}");
        });
        let isa = isa_builder.finish(settings::Flags::new(flag_builder))?;
        Ok(Self::with_isa(isa, libcall_names))
    }

    /// Create a new `JITBuilder` with an arbitrary target. This is mainly
    /// useful for testing.
    ///
    /// To create a `JITBuilder` for native use, use the `new` or `with_flags`
    /// constructors instead.
    ///
    /// The `libcall_names` function provides a way to translate `cranelift_codegen`'s `ir::LibCall`
    /// enum to symbols. LibCalls are inserted in the IR as part of the legalization for certain
    /// floating point instructions, and for stack probes. If you don't know what to use for this
    /// argument, use `cranelift_module::default_libcall_names()`.
    pub fn with_isa(
        isa: OwnedTargetIsa,
        libcall_names: Box<dyn Fn(ir::LibCall) -> String + Send + Sync>,
    ) -> Self {
        let symbols = HashMap::new();
        let lookup_symbols = vec![Box::new(lookup_with_dlsym) as Box<_>];
        Self {
            isa,
            symbols,
            lookup_symbols,
            libcall_names,
            memory: None,
        }
    }

    /// Define a symbol in the internal symbol table.
    ///
    /// The JIT will use the symbol table to resolve names that are declared,
    /// but not defined, in the module being compiled.  A common example is
    /// external functions.  With this method, functions and data can be exposed
    /// to the code being compiled which are defined by the host.
    ///
    /// If a symbol is defined more than once, the most recent definition will
    /// be retained.
    ///
    /// If the JIT fails to find a symbol in its internal table, it will fall
    /// back to a platform-specific search (this typically involves searching
    /// the current process for public symbols, followed by searching the
    /// platform's C runtime).
    pub fn symbol<K>(&mut self, name: K, ptr: *const u8) -> &mut Self
    where
        K: Into<String>,
    {
        self.symbols.insert(name.into(), SendWrapper(ptr));
        self
    }

    /// Define multiple symbols in the internal symbol table.
    ///
    /// Using this is equivalent to calling `symbol` on each element.
    pub fn symbols<It, K>(&mut self, symbols: It) -> &mut Self
    where
        It: IntoIterator<Item = (K, *const u8)>,
        K: Into<String>,
    {
        for (name, ptr) in symbols {
            self.symbols.insert(name.into(), SendWrapper(ptr));
        }
        self
    }

    /// Add a symbol lookup fn.
    ///
    /// Symbol lookup fn's are used to lookup symbols when they couldn't be found in the internal
    /// symbol table. Symbol lookup fn's are called in reverse of the order in which they were added.
    pub fn symbol_lookup_fn(
        &mut self,
        symbol_lookup_fn: Box<dyn Fn(&str) -> Option<*const u8> + Send>,
    ) -> &mut Self {
        self.lookup_symbols.push(symbol_lookup_fn);
        self
    }

    /// Set the memory provider for the module.
    ///
    /// If unset, defaults to [`SystemMemoryProvider`].
    pub fn memory_provider(&mut self, provider: Box<dyn JITMemoryProvider>) -> &mut Self {
        self.memory = Some(provider);
        self
    }
}

/// A wrapper that impls Send for the contents.
///
/// SAFETY: This must not be used for any types where it would be UB for them to be Send
#[derive(Copy, Clone)]
struct SendWrapper<T>(T);
unsafe impl<T> Send for SendWrapper<T> {}

/// A `JITModule` implements `Module` and emits code and data into memory where it can be
/// directly called and accessed.
///
/// See the `JITBuilder` for a convenient way to construct `JITModule` instances.
pub struct JITModule {
    isa: OwnedTargetIsa,
    symbols: RefCell<HashMap<String, SendWrapper<*const u8>>>,
    lookup_symbols: Vec<Box<dyn Fn(&str) -> Option<*const u8> + Send>>,
    libcall_names: Box<dyn Fn(ir::LibCall) -> String + Send + Sync>,
    memory: Box<dyn JITMemoryProvider>,
    declarations: ModuleDeclarations,
    compiled_functions: SecondaryMap<FuncId, Option<CompiledBlob>>,
    compiled_data_objects: SecondaryMap<DataId, Option<CompiledBlob>>,
    code_ranges: Vec<(usize, usize, FuncId)>,
    functions_to_finalize: Vec<FuncId>,
    data_objects_to_finalize: Vec<DataId>,
}

impl JITModule {
    /// Free memory allocated for code and data segments of compiled functions.
    ///
    /// # Safety
    ///
    /// Because this function invalidates any pointers retrieved from the
    /// corresponding module, it should only be used when none of the functions
    /// from that module are currently executing and none of the `fn` pointers
    /// are called afterwards.
    pub unsafe fn free_memory(mut self) {
        self.memory.free_memory();
    }

    fn lookup_symbol(&self, name: &str) -> Option<*const u8> {
        match self.symbols.borrow_mut().entry(name.to_owned()) {
            std::collections::hash_map::Entry::Occupied(occ) => Some(occ.get().0),
            std::collections::hash_map::Entry::Vacant(vac) => {
                let ptr = self
                    .lookup_symbols
                    .iter()
                    .rev() // Try last lookup function first
                    .find_map(|lookup| lookup(name));
                if let Some(ptr) = ptr {
                    vac.insert(SendWrapper(ptr));
                }
                ptr
            }
        }
    }

    fn get_address(&self, name: &ModuleRelocTarget) -> *const u8 {
        match *name {
            ModuleRelocTarget::User { .. } => {
                let (name, linkage) = if ModuleDeclarations::is_function(name) {
                    let func_id = FuncId::from_name(name);
                    match &self.compiled_functions[func_id] {
                        Some(compiled) => return compiled.ptr,
                        None => {
                            let decl = self.declarations.get_function_decl(func_id);
                            (&decl.name, decl.linkage)
                        }
                    }
                } else {
                    let data_id = DataId::from_name(name);
                    match &self.compiled_data_objects[data_id] {
                        Some(compiled) => return compiled.ptr,
                        None => {
                            let decl = self.declarations.get_data_decl(data_id);
                            (&decl.name, decl.linkage)
                        }
                    }
                };
                let name = name
                    .as_ref()
                    .expect("anonymous symbol must be defined locally");
                if let Some(ptr) = self.lookup_symbol(name) {
                    ptr
                } else if linkage == Linkage::Preemptible {
                    0 as *const u8
                } else {
                    panic!("can't resolve symbol {name}");
                }
            }
            ModuleRelocTarget::LibCall(ref libcall) => {
                let sym = (self.libcall_names)(*libcall);
                self.lookup_symbol(&sym)
                    .unwrap_or_else(|| panic!("can't resolve libcall {sym}"))
            }
            _ => panic!("invalid name"),
        }
    }

    /// Returns the address of a finalized function.
    ///
    /// The pointer remains valid until either [`JITModule::free_memory`] is called or in the future
    /// some way of deallocating this individual function is used.
    pub fn get_finalized_function(&self, func_id: FuncId) -> *const u8 {
        let info = &self.compiled_functions[func_id];
        assert!(
            !self.functions_to_finalize.iter().any(|x| *x == func_id),
            "function not yet finalized"
        );
        info.as_ref()
            .expect("function must be compiled before it can be finalized")
            .ptr
    }

    /// Returns the address and size of a finalized data object.
    ///
    /// The pointer remains valid until either [`JITModule::free_memory`] is called or in the future
    /// some way of deallocating this individual data object is used.
    pub fn get_finalized_data(&self, data_id: DataId) -> (*const u8, usize) {
        let info = &self.compiled_data_objects[data_id];
        assert!(
            !self.data_objects_to_finalize.iter().any(|x| *x == data_id),
            "data object not yet finalized"
        );
        let compiled = info
            .as_ref()
            .expect("data object must be compiled before it can be finalized");

        (compiled.ptr, compiled.size)
    }

    fn record_function_for_perf(&self, ptr: *mut u8, size: usize, name: &str) {
        // The Linux perf tool supports JIT code via a /tmp/perf-$PID.map file,
        // which contains memory regions and their associated names.  If we
        // are profiling with perf and saving binaries to PERF_BUILDID_DIR
        // for post-profile analysis, write information about each function
        // we define.
        if cfg!(unix) && ::std::env::var_os("PERF_BUILDID_DIR").is_some() {
            let mut map_file = ::std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(format!("/tmp/perf-{}.map", ::std::process::id()))
                .unwrap();

            let _ = writeln!(map_file, "{:x} {:x} {}", ptr as usize, size, name);
        }
    }

    /// Finalize all functions and data objects that are defined but not yet finalized.
    /// All symbols referenced in their bodies that are declared as needing a definition
    /// must be defined by this point.
    ///
    /// Use `get_finalized_function` and `get_finalized_data` to obtain the final
    /// artifacts.
    ///
    /// Returns ModuleError in case of allocation or syscall failure
    pub fn finalize_definitions(&mut self) -> ModuleResult<()> {
        for func in std::mem::take(&mut self.functions_to_finalize) {
            let decl = self.declarations.get_function_decl(func);
            assert!(decl.linkage.is_definable());
            let func = self.compiled_functions[func]
                .as_ref()
                .expect("function must be compiled before it can be finalized");
            func.perform_relocations(|name| self.get_address(name));
        }

        for data in std::mem::take(&mut self.data_objects_to_finalize) {
            let decl = self.declarations.get_data_decl(data);
            assert!(decl.linkage.is_definable());
            let data = self.compiled_data_objects[data]
                .as_ref()
                .expect("data object must be compiled before it can be finalized");
            data.perform_relocations(|name| self.get_address(name));
        }

        self.code_ranges
            .sort_unstable_by_key(|(start, _end, _)| *start);

        // Now that we're done patching, prepare the memory for execution!
        let branch_protection = if cfg!(target_arch = "aarch64") && use_bti(&self.isa.isa_flags()) {
            BranchProtection::BTI
        } else {
            BranchProtection::None
        };
        self.memory.finalize(branch_protection)?;

        Ok(())
    }

    /// Create a new `JITModule`.
    pub fn new(builder: JITBuilder) -> Self {
        assert!(
            !builder.isa.flags().is_pic(),
            "cranelift-jit needs is_pic=false"
        );

        let memory = builder
            .memory
            .unwrap_or_else(|| Box::new(SystemMemoryProvider::new()));
        Self {
            isa: builder.isa,
            symbols: RefCell::new(builder.symbols),
            lookup_symbols: builder.lookup_symbols,
            libcall_names: builder.libcall_names,
            memory,
            declarations: ModuleDeclarations::default(),
            compiled_functions: SecondaryMap::new(),
            compiled_data_objects: SecondaryMap::new(),
            code_ranges: Vec::new(),
            functions_to_finalize: Vec::new(),
            data_objects_to_finalize: Vec::new(),
        }
    }

    /// Look up the Wasmtime unwind ExceptionTable and corresponding
    /// base PC, if any, for a given PC that may be within one of the
    /// CompiledBlobs in this module.
    #[cfg(feature = "wasmtime-unwinder")]
    pub fn lookup_wasmtime_exception_data<'a>(
        &'a self,
        pc: usize,
    ) -> Option<(usize, wasmtime_unwinder::ExceptionTable<'a>)> {
        // Search the sorted code-ranges for the PC.
        let idx = match self
            .code_ranges
            .binary_search_by_key(&pc, |(start, _end, _func)| *start)
        {
            Ok(exact_start_match) => Some(exact_start_match),
            Err(least_upper_bound) if least_upper_bound > 0 => {
                let last_range_before_pc = &self.code_ranges[least_upper_bound - 1];
                if last_range_before_pc.0 <= pc && pc < last_range_before_pc.1 {
                    Some(least_upper_bound - 1)
                } else {
                    None
                }
            }
            _ => None,
        }?;

        let (start, _, func) = self.code_ranges[idx];

        // Get the ExceptionTable. The "parse" here simply reads two
        // u32s for lengths and constructs borrowed slices, so it's
        // cheap.
        let data = self.compiled_functions[func]
            .as_ref()
            .unwrap()
            .exception_data
            .as_ref()?;
        let exception_table = wasmtime_unwinder::ExceptionTable::parse(data).ok()?;
        Some((start, exception_table))
    }
}

impl Module for JITModule {
    fn isa(&self) -> &dyn TargetIsa {
        &*self.isa
    }

    fn declarations(&self) -> &ModuleDeclarations {
        &self.declarations
    }

    fn declare_function(
        &mut self,
        name: &str,
        linkage: Linkage,
        signature: &ir::Signature,
    ) -> ModuleResult<FuncId> {
        let (id, _linkage) = self
            .declarations
            .declare_function(name, linkage, signature)?;
        Ok(id)
    }

    fn declare_anonymous_function(&mut self, signature: &ir::Signature) -> ModuleResult<FuncId> {
        let id = self.declarations.declare_anonymous_function(signature)?;
        Ok(id)
    }

    fn declare_data(
        &mut self,
        name: &str,
        linkage: Linkage,
        writable: bool,
        tls: bool,
    ) -> ModuleResult<DataId> {
        assert!(!tls, "JIT doesn't yet support TLS");
        let (id, _linkage) = self
            .declarations
            .declare_data(name, linkage, writable, tls)?;
        Ok(id)
    }

    fn declare_anonymous_data(&mut self, writable: bool, tls: bool) -> ModuleResult<DataId> {
        assert!(!tls, "JIT doesn't yet support TLS");
        let id = self.declarations.declare_anonymous_data(writable, tls)?;
        Ok(id)
    }

    fn define_function_with_control_plane(
        &mut self,
        id: FuncId,
        ctx: &mut cranelift_codegen::Context,
        ctrl_plane: &mut ControlPlane,
    ) -> ModuleResult<()> {
        info!("defining function {}: {}", id, ctx.func.display());
        let decl = self.declarations.get_function_decl(id);
        if !decl.linkage.is_definable() {
            return Err(ModuleError::InvalidImportDefinition(
                decl.linkage_name(id).into_owned(),
            ));
        }

        if !self.compiled_functions[id].is_none() {
            return Err(ModuleError::DuplicateDefinition(
                decl.linkage_name(id).into_owned(),
            ));
        }

        // work around borrow-checker to allow reuse of ctx below
        let res = ctx.compile(self.isa(), ctrl_plane)?;
        let alignment = res.buffer.alignment as u64;
        let compiled_code = ctx.compiled_code().unwrap();

        let size = compiled_code.code_info().total_size as usize;
        let align = alignment
            .max(self.isa.function_alignment().minimum as u64)
            .max(self.isa.symbol_alignment());
        let ptr =
            self.memory
                .allocate_readexec(size, align)
                .map_err(|e| ModuleError::Allocation {
                    message: "unable to alloc function",
                    err: e,
                })?;

        {
            let mem = unsafe { std::slice::from_raw_parts_mut(ptr, size) };
            mem.copy_from_slice(compiled_code.code_buffer());
        }

        let relocs = compiled_code
            .buffer
            .relocs()
            .iter()
            .map(|reloc| ModuleReloc::from_mach_reloc(reloc, &ctx.func, id))
            .collect();

        self.record_function_for_perf(ptr, size, &decl.linkage_name(id));
        self.compiled_functions[id] = Some(CompiledBlob {
            ptr,
            size,
            relocs,
            #[cfg(feature = "wasmtime-unwinder")]
            exception_data: None,
        });

        let range_start = ptr as usize;
        let range_end = range_start + size;
        // These will be sorted when we finalize.
        self.code_ranges.push((range_start, range_end, id));

        #[cfg(feature = "wasmtime-unwinder")]
        {
            let mut exception_builder = wasmtime_unwinder::ExceptionTableBuilder::default();
            exception_builder
                .add_func(0, compiled_code.buffer.call_sites())
                .map_err(|_| {
                    ModuleError::Compilation(cranelift_codegen::CodegenError::Unsupported(
                        "Invalid exception data".into(),
                    ))
                })?;
            self.compiled_functions[id].as_mut().unwrap().exception_data =
                Some(exception_builder.to_vec());
        }

        self.functions_to_finalize.push(id);

        Ok(())
    }

    fn define_function_bytes(
        &mut self,
        id: FuncId,
        alignment: u64,
        bytes: &[u8],
        relocs: &[ModuleReloc],
    ) -> ModuleResult<()> {
        info!("defining function {id} with bytes");
        let decl = self.declarations.get_function_decl(id);
        if !decl.linkage.is_definable() {
            return Err(ModuleError::InvalidImportDefinition(
                decl.linkage_name(id).into_owned(),
            ));
        }

        if !self.compiled_functions[id].is_none() {
            return Err(ModuleError::DuplicateDefinition(
                decl.linkage_name(id).into_owned(),
            ));
        }

        let size = bytes.len();
        let align = alignment
            .max(self.isa.function_alignment().minimum as u64)
            .max(self.isa.symbol_alignment());
        let ptr =
            self.memory
                .allocate_readexec(size, align)
                .map_err(|e| ModuleError::Allocation {
                    message: "unable to alloc function bytes",
                    err: e,
                })?;

        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, size);
        }

        self.record_function_for_perf(ptr, size, &decl.linkage_name(id));
        self.compiled_functions[id] = Some(CompiledBlob {
            ptr,
            size,
            relocs: relocs.to_owned(),
            #[cfg(feature = "wasmtime-unwinder")]
            exception_data: None,
        });

        self.functions_to_finalize.push(id);

        Ok(())
    }

    fn define_data(&mut self, id: DataId, data: &DataDescription) -> ModuleResult<()> {
        let decl = self.declarations.get_data_decl(id);
        if !decl.linkage.is_definable() {
            return Err(ModuleError::InvalidImportDefinition(
                decl.linkage_name(id).into_owned(),
            ));
        }

        if !self.compiled_data_objects[id].is_none() {
            return Err(ModuleError::DuplicateDefinition(
                decl.linkage_name(id).into_owned(),
            ));
        }

        assert!(!decl.tls, "JIT doesn't yet support TLS");

        let &DataDescription {
            ref init,
            function_decls: _,
            data_decls: _,
            function_relocs: _,
            data_relocs: _,
            custom_segment_section: _,
            align,
            used: _,
        } = data;

        // Make sure to allocate at least 1 byte. Allocating 0 bytes is UB. Previously a dummy
        // value was used, however as it turns out this will cause pc-relative relocations to
        // fail on architectures where pc-relative offsets are range restricted as the dummy
        // value is not close enough to the code that has the pc-relative relocation.
        let alloc_size = std::cmp::max(init.size(), 1);

        let ptr = if decl.writable {
            self.memory
                .allocate_readwrite(alloc_size, align.unwrap_or(WRITABLE_DATA_ALIGNMENT))
                .map_err(|e| ModuleError::Allocation {
                    message: "unable to alloc writable data",
                    err: e,
                })?
        } else {
            self.memory
                .allocate_readonly(alloc_size, align.unwrap_or(READONLY_DATA_ALIGNMENT))
                .map_err(|e| ModuleError::Allocation {
                    message: "unable to alloc readonly data",
                    err: e,
                })?
        };

        if ptr.is_null() {
            // FIXME pass a Layout to allocate and only compute the layout once.
            std::alloc::handle_alloc_error(
                std::alloc::Layout::from_size_align(
                    alloc_size,
                    align.unwrap_or(READONLY_DATA_ALIGNMENT).try_into().unwrap(),
                )
                .unwrap(),
            );
        }

        match *init {
            Init::Uninitialized => {
                panic!("data is not initialized yet");
            }
            Init::Zeros { size } => {
                unsafe { ptr::write_bytes(ptr, 0, size) };
            }
            Init::Bytes { ref contents } => {
                let src = contents.as_ptr();
                unsafe { ptr::copy_nonoverlapping(src, ptr, contents.len()) };
            }
        }

        let pointer_reloc = match self.isa.triple().pointer_width().unwrap() {
            PointerWidth::U16 => panic!(),
            PointerWidth::U32 => Reloc::Abs4,
            PointerWidth::U64 => Reloc::Abs8,
        };
        let relocs = data.all_relocs(pointer_reloc).collect::<Vec<_>>();

        self.compiled_data_objects[id] = Some(CompiledBlob {
            ptr,
            size: init.size(),
            relocs,
            #[cfg(feature = "wasmtime-unwinder")]
            exception_data: None,
        });
        self.data_objects_to_finalize.push(id);

        Ok(())
    }

    fn get_name(&self, name: &str) -> Option<cranelift_module::FuncOrDataId> {
        self.declarations().get_name(name)
    }

    fn target_config(&self) -> cranelift_codegen::isa::TargetFrontendConfig {
        self.isa().frontend_config()
    }

    fn make_context(&self) -> cranelift_codegen::Context {
        let mut ctx = cranelift_codegen::Context::new();
        ctx.func.signature.call_conv = self.isa().default_call_conv();
        ctx
    }

    fn clear_context(&self, ctx: &mut cranelift_codegen::Context) {
        ctx.clear();
        ctx.func.signature.call_conv = self.isa().default_call_conv();
    }

    fn make_signature(&self) -> ir::Signature {
        ir::Signature::new(self.isa().default_call_conv())
    }

    fn clear_signature(&self, sig: &mut ir::Signature) {
        sig.clear(self.isa().default_call_conv());
    }
}

#[cfg(not(windows))]
fn lookup_with_dlsym(name: &str) -> Option<*const u8> {
    let c_str = CString::new(name).unwrap();
    let c_str_ptr = c_str.as_ptr();
    let sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c_str_ptr) };
    if sym.is_null() {
        None
    } else {
        Some(sym as *const u8)
    }
}

#[cfg(windows)]
fn lookup_with_dlsym(name: &str) -> Option<*const u8> {
    use std::os::windows::io::RawHandle;
    use windows_sys::Win32::Foundation::HMODULE;
    use windows_sys::Win32::System::LibraryLoader;

    const UCRTBASE: &[u8] = b"ucrtbase.dll\0";

    let c_str = CString::new(name).unwrap();
    let c_str_ptr = c_str.as_ptr();

    unsafe {
        let handles = [
            // try to find the searched symbol in the currently running executable
            ptr::null_mut(),
            // try to find the searched symbol in local c runtime
            LibraryLoader::GetModuleHandleA(UCRTBASE.as_ptr()) as RawHandle,
        ];

        for handle in &handles {
            let addr = LibraryLoader::GetProcAddress(*handle as HMODULE, c_str_ptr.cast());
            match addr {
                None => continue,
                Some(addr) => return Some(addr as *const u8),
            }
        }

        None
    }
}

fn use_bti(isa_flags: &Vec<settings::Value>) -> bool {
    isa_flags
        .iter()
        .find(|&f| f.name == "use_bti")
        .map_or(false, |f| f.as_bool().unwrap_or(false))
}
