//! Defines `SimpleJITModule`.

use crate::{compiled_blob::CompiledBlob, memory::Memory};
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::settings::Configurable;
use cranelift_codegen::{self, ir, settings};
use cranelift_codegen::{
    binemit::{self, Addend, CodeInfo, CodeOffset, Reloc, RelocSink, TrapSink},
    CodegenError,
};
use cranelift_entity::SecondaryMap;
use cranelift_module::{
    DataContext, DataDescription, DataId, FuncId, Init, Linkage, Module, ModuleCompiledFunction,
    ModuleDeclarations, ModuleError, ModuleResult, RelocRecord,
};
use cranelift_native;
#[cfg(not(windows))]
use libc;
use log::info;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::ffi::CString;
use std::io::Write;
use std::ptr;
use std::ptr::NonNull;
use target_lexicon::PointerWidth;
#[cfg(windows)]
use winapi;

const EXECUTABLE_DATA_ALIGNMENT: u64 = 0x10;
const WRITABLE_DATA_ALIGNMENT: u64 = 0x8;
const READONLY_DATA_ALIGNMENT: u64 = 0x1;

/// A builder for `SimpleJITModule`.
pub struct SimpleJITBuilder {
    isa: Box<dyn TargetIsa>,
    symbols: HashMap<String, *const u8>,
    libcall_names: Box<dyn Fn(ir::LibCall) -> String + Send + Sync>,
}

impl SimpleJITBuilder {
    /// Create a new `SimpleJITBuilder`.
    ///
    /// The `libcall_names` function provides a way to translate `cranelift_codegen`'s `ir::LibCall`
    /// enum to symbols. LibCalls are inserted in the IR as part of the legalization for certain
    /// floating point instructions, and for stack probes. If you don't know what to use for this
    /// argument, use `cranelift_module::default_libcall_names()`.
    pub fn new(libcall_names: Box<dyn Fn(ir::LibCall) -> String + Send + Sync>) -> Self {
        let mut flag_builder = settings::builder();
        // On at least AArch64, "colocated" calls use shorter-range relocations,
        // which might not reach all definitions; we can't handle that here, so
        // we require long-range relocation types.
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
            panic!("host machine is not supported: {}", msg);
        });
        let isa = isa_builder.finish(settings::Flags::new(flag_builder));
        Self::with_isa(isa, libcall_names)
    }

    /// Create a new `SimpleJITBuilder` with an arbitrary target. This is mainly
    /// useful for testing.
    ///
    /// SimpleJIT requires a `TargetIsa` configured for non-PIC.
    ///
    /// To create a `SimpleJITBuilder` for native use, use the `new` constructor
    /// instead.
    ///
    /// The `libcall_names` function provides a way to translate `cranelift_codegen`'s `ir::LibCall`
    /// enum to symbols. LibCalls are inserted in the IR as part of the legalization for certain
    /// floating point instructions, and for stack probes. If you don't know what to use for this
    /// argument, use `cranelift_module::default_libcall_names()`.
    pub fn with_isa(
        isa: Box<dyn TargetIsa>,
        libcall_names: Box<dyn Fn(ir::LibCall) -> String + Send + Sync>,
    ) -> Self {
        assert!(isa.flags().is_pic(), "SimpleJIT requires PIC code");
        let symbols = HashMap::new();
        Self {
            isa,
            symbols,
            libcall_names,
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
    pub fn symbol<K>(&mut self, name: K, ptr: *const u8) -> &Self
    where
        K: Into<String>,
    {
        self.symbols.insert(name.into(), ptr);
        self
    }

    /// Define multiple symbols in the internal symbol table.
    ///
    /// Using this is equivalent to calling `symbol` on each element.
    pub fn symbols<It, K>(&mut self, symbols: It) -> &Self
    where
        It: IntoIterator<Item = (K, *const u8)>,
        K: Into<String>,
    {
        for (name, ptr) in symbols {
            self.symbols.insert(name.into(), ptr);
        }
        self
    }
}

/// A `SimpleJITModule` implements `Module` and emits code and data into memory where it can be
/// directly called and accessed.
///
/// See the `SimpleJITBuilder` for a convenient way to construct `SimpleJITModule` instances.
pub struct SimpleJITModule {
    isa: Box<dyn TargetIsa>,
    symbols: HashMap<String, *const u8>,
    libcall_names: Box<dyn Fn(ir::LibCall) -> String>,
    memory: MemoryHandle,
    declarations: ModuleDeclarations,
    function_got_entries: SecondaryMap<FuncId, Option<NonNull<*const u8>>>,
    function_plt_entries: SecondaryMap<FuncId, Option<NonNull<[u8; 16]>>>,
    data_object_got_entries: SecondaryMap<DataId, Option<NonNull<*const u8>>>,
    libcall_got_entries: HashMap<ir::LibCall, NonNull<*const u8>>,
    libcall_plt_entries: HashMap<ir::LibCall, NonNull<[u8; 16]>>,
    compiled_functions: SecondaryMap<FuncId, Option<CompiledBlob>>,
    compiled_data_objects: SecondaryMap<DataId, Option<CompiledBlob>>,
    functions_to_finalize: Vec<FuncId>,
    data_objects_to_finalize: Vec<DataId>,
}

/// A handle to allow freeing memory allocated by the `Module`.
struct MemoryHandle {
    code: Memory,
    readonly: Memory,
    writable: Memory,
}

impl SimpleJITModule {
    /// Free memory allocated for code and data segments of compiled functions.
    ///
    /// # Safety
    ///
    /// Because this function invalidates any pointers retrived from the
    /// corresponding module, it should only be used when none of the functions
    /// from that module are currently executing and none of the `fn` pointers
    /// are called afterwards.
    pub unsafe fn free_memory(&mut self) {
        self.memory.code.free_memory();
        self.memory.readonly.free_memory();
        self.memory.writable.free_memory();
    }

    fn lookup_symbol(&self, name: &str) -> Option<*const u8> {
        self.symbols
            .get(name)
            .copied()
            .or_else(|| lookup_with_dlsym(name))
    }

    unsafe fn write_plt_entry_bytes(plt_ptr: *mut [u8; 16], got_ptr: *mut *const u8) {
        assert!(cfg!(target_arch = "x86_64"), "PLT is currently only supported on x86_64");
        // jmp *got_ptr; ud2; ud2; ud2; ud2; ud2
        let mut plt_val = [
            0xff, 0x25, 0, 0, 0, 0, 0x0f, 0x0b, 0x0f, 0x0b, 0x0f, 0x0b, 0x0f, 0x0b, 0x0f, 0x0b,
        ];
        let what = got_ptr as isize - 4;
        let at = plt_ptr as isize + 2;
        plt_val[2..6].copy_from_slice(&i32::to_ne_bytes(i32::try_from(what - at).unwrap()));
        std::ptr::write(plt_ptr, plt_val);
    }

    fn get_address(&self, name: &ir::ExternalName) -> *const u8 {
        match *name {
            ir::ExternalName::User { .. } => {
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
                if let Some(ptr) = self.lookup_symbol(&name) {
                    ptr
                } else if linkage == Linkage::Preemptible {
                    0 as *const u8
                } else {
                    panic!("can't resolve symbol {}", name);
                }
            }
            ir::ExternalName::LibCall(ref libcall) => {
                let sym = (self.libcall_names)(*libcall);
                self.lookup_symbol(&sym)
                    .unwrap_or_else(|| panic!("can't resolve libcall {}", sym))
            }
            _ => panic!("invalid ExternalName {}", name),
        }
    }

    fn get_got_address(&self, name: &ir::ExternalName) -> *const u8 {
        match *name {
            ir::ExternalName::User { .. } => {
                if ModuleDeclarations::is_function(name) {
                    let func_id = FuncId::from_name(name);
                    self.function_got_entries[func_id]
                        .unwrap()
                        .as_ptr()
                        .cast::<u8>()
                } else {
                    let data_id = DataId::from_name(name);
                    self.data_object_got_entries[data_id]
                        .unwrap()
                        .as_ptr()
                        .cast::<u8>()
                }
            }
            ir::ExternalName::LibCall(ref libcall) => self
                .libcall_got_entries
                .get(libcall)
                .unwrap_or_else(|| panic!("can't resolve libcall {}", libcall))
                .as_ptr()
                .cast::<u8>(),
            _ => panic!("invalid ExternalName {}", name),
        }
    }

    fn get_plt_address(&self, name: &ir::ExternalName) -> *const u8 {
        match *name {
            ir::ExternalName::User { .. } => {
                if ModuleDeclarations::is_function(name) {
                    let func_id = FuncId::from_name(name);
                    self.function_plt_entries[func_id]
                        .unwrap()
                        .as_ptr()
                        .cast::<u8>()
                } else {
                    unreachable!("PLT relocations can only have functions as target");
                }
            }
            ir::ExternalName::LibCall(ref libcall) => self
                .libcall_plt_entries
                .get(libcall)
                .unwrap_or_else(|| panic!("can't resolve libcall {}", libcall))
                .as_ptr()
                .cast::<u8>(),
            _ => panic!("invalid ExternalName {}", name),
        }
    }

    /// Returns the address of a finalized function.
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
        if cfg!(target_os = "linux") && ::std::env::var_os("PERF_BUILDID_DIR").is_some() {
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
    pub fn finalize_definitions(&mut self) {
        for func in std::mem::take(&mut self.functions_to_finalize) {
            let decl = self.declarations.get_function_decl(func);
            assert!(decl.linkage.is_definable());
            let func_blob = self.compiled_functions[func]
                .as_ref()
                .expect("function must be compiled before it can be finalized");
            func_blob.perform_relocations(
                |name| unreachable!("non-GOT/PLT relocation in function {} to {}", func, name),
                |name| self.get_got_address(name),
                |name| self.get_plt_address(name),
            );
        }
        for data in std::mem::take(&mut self.data_objects_to_finalize) {
            let decl = self.declarations.get_data_decl(data);
            assert!(decl.linkage.is_definable());
            let data = self.compiled_data_objects[data]
                .as_ref()
                .expect("data object must be compiled before it can be finalized");
            data.perform_relocations(
                |name| self.get_address(name),
                |name| self.get_got_address(name),
                |name| self.get_plt_address(name),
            );
        }

        // Now that we're done patching, prepare the memory for execution!
        self.memory.readonly.set_readonly();
        self.memory.code.set_readable_and_executable();
    }

    /// Create a new `SimpleJITModule`.
    pub fn new(builder: SimpleJITBuilder) -> Self {
        let mut memory = MemoryHandle {
            code: Memory::new(),
            readonly: Memory::new(),
            writable: Memory::new(),
        };

        let mut libcall_got_entries = HashMap::new();
        let mut libcall_plt_entries = HashMap::new();

        // Pre-create a GOT entry for each libcall.
        for &libcall in ir::LibCall::all_libcalls() {
            let got_entry = memory
                .writable
                .allocate(
                    std::mem::size_of::<*const u8>(),
                    std::mem::align_of::<*const u8>().try_into().unwrap(),
                )
                .unwrap()
                .cast::<*const u8>();
            libcall_got_entries.insert(libcall, NonNull::new(got_entry).unwrap());
            let sym = (builder.libcall_names)(libcall);
            let addr = if let Some(addr) = builder
                .symbols
                .get(&sym)
                .copied()
                .or_else(|| lookup_with_dlsym(&sym))
            {
                addr
            } else {
                continue;
            };
            unsafe {
                std::ptr::write(got_entry, addr);
            }
            let plt_entry = memory
                .code
                .allocate(std::mem::size_of::<[u8; 16]>(), EXECUTABLE_DATA_ALIGNMENT)
                .unwrap()
                .cast::<[u8; 16]>();
            libcall_plt_entries.insert(libcall, NonNull::new(plt_entry).unwrap());
            unsafe {
                Self::write_plt_entry_bytes(plt_entry, got_entry);
            }
        }

        Self {
            isa: builder.isa,
            symbols: builder.symbols,
            libcall_names: builder.libcall_names,
            memory,
            declarations: ModuleDeclarations::default(),
            function_got_entries: SecondaryMap::new(),
            function_plt_entries: SecondaryMap::new(),
            data_object_got_entries: SecondaryMap::new(),
            libcall_got_entries,
            libcall_plt_entries,
            compiled_functions: SecondaryMap::new(),
            compiled_data_objects: SecondaryMap::new(),
            functions_to_finalize: Vec::new(),
            data_objects_to_finalize: Vec::new(),
        }
    }
}

impl<'simple_jit_backend> Module for SimpleJITModule {
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
        let (id, _decl) = self
            .declarations
            .declare_function(name, linkage, signature)?;
        if self.function_got_entries[id].is_none() {
            let got_entry = self
                .memory
                .writable
                .allocate(
                    std::mem::size_of::<*const u8>(),
                    std::mem::align_of::<*const u8>().try_into().unwrap(),
                )
                .unwrap()
                .cast::<*const u8>();
            self.function_got_entries[id] = Some(NonNull::new(got_entry).unwrap());
            // FIXME populate got entries with a null pointer when defined
            let val = self.lookup_symbol(name).unwrap_or(std::ptr::null());
            unsafe {
                std::ptr::write(got_entry, val);
            }
            let plt_entry = self
                .memory
                .code
                .allocate(std::mem::size_of::<[u8; 16]>(), EXECUTABLE_DATA_ALIGNMENT)
                .unwrap()
                .cast::<[u8; 16]>();
            self.function_plt_entries[id] = Some(NonNull::new(plt_entry).unwrap());
            unsafe {
                Self::write_plt_entry_bytes(plt_entry, got_entry);
            }
        }
        Ok(id)
    }

    fn declare_data(
        &mut self,
        name: &str,
        linkage: Linkage,
        writable: bool,
        tls: bool,
    ) -> ModuleResult<DataId> {
        assert!(!tls, "SimpleJIT doesn't yet support TLS");
        let (id, _decl) = self
            .declarations
            .declare_data(name, linkage, writable, tls)?;
        if self.data_object_got_entries[id].is_none() {
            let got_entry = self
                .memory
                .writable
                .allocate(
                    std::mem::size_of::<*const u8>(),
                    std::mem::align_of::<*const u8>().try_into().unwrap(),
                )
                .unwrap()
                .cast::<*const u8>();
            self.data_object_got_entries[id] = Some(NonNull::new(got_entry).unwrap());
            // FIXME populate got entries with a null pointer when defined
            let val = self.lookup_symbol(name).unwrap_or(std::ptr::null());
            unsafe {
                std::ptr::write(got_entry, val);
            }
        }
        Ok(id)
    }

    /// Use this when you're building the IR of a function to reference a function.
    ///
    /// TODO: Coalesce redundant decls and signatures.
    /// TODO: Look into ways to reduce the risk of using a FuncRef in the wrong function.
    fn declare_func_in_func(&self, func: FuncId, in_func: &mut ir::Function) -> ir::FuncRef {
        let decl = self.declarations.get_function_decl(func);
        let signature = in_func.import_signature(decl.signature.clone());
        in_func.import_function(ir::ExtFuncData {
            name: ir::ExternalName::user(0, func.as_u32()),
            signature,
            colocated: false,
        })
    }

    /// Use this when you're building the IR of a function to reference a data object.
    ///
    /// TODO: Same as above.
    fn declare_data_in_func(&self, data: DataId, func: &mut ir::Function) -> ir::GlobalValue {
        let decl = self.declarations.get_data_decl(data);
        func.create_global_value(ir::GlobalValueData::Symbol {
            name: ir::ExternalName::user(1, data.as_u32()),
            offset: ir::immediates::Imm64::new(0),
            colocated: false,
            tls: decl.tls,
        })
    }

    /// TODO: Same as above.
    fn declare_func_in_data(&self, func: FuncId, ctx: &mut DataContext) -> ir::FuncRef {
        ctx.import_function(ir::ExternalName::user(0, func.as_u32()))
    }

    /// TODO: Same as above.
    fn declare_data_in_data(&self, data: DataId, ctx: &mut DataContext) -> ir::GlobalValue {
        ctx.import_global_value(ir::ExternalName::user(1, data.as_u32()))
    }

    fn define_function<TS>(
        &mut self,
        id: FuncId,
        ctx: &mut cranelift_codegen::Context,
        trap_sink: &mut TS,
    ) -> ModuleResult<ModuleCompiledFunction>
    where
        TS: TrapSink,
    {
        info!("defining function {}: {}", id, ctx.func.display(self.isa()));
        let CodeInfo {
            total_size: code_size,
            ..
        } = ctx.compile(self.isa())?;

        let decl = self.declarations.get_function_decl(id);
        if !decl.linkage.is_definable() {
            return Err(ModuleError::InvalidImportDefinition(decl.name.clone()));
        }

        if !self.compiled_functions[id].is_none() {
            return Err(ModuleError::DuplicateDefinition(decl.name.to_owned()));
        }

        let size = code_size as usize;
        let ptr = self
            .memory
            .code
            .allocate(size, EXECUTABLE_DATA_ALIGNMENT)
            .expect("TODO: handle OOM etc.");

        let mut reloc_sink = SimpleJITRelocSink::default();
        let mut stack_map_sink = binemit::NullStackMapSink {};
        unsafe {
            ctx.emit_to_memory(
                &*self.isa,
                ptr,
                &mut reloc_sink,
                trap_sink,
                &mut stack_map_sink,
            )
        };

        self.record_function_for_perf(ptr, size, &decl.name);
        self.compiled_functions[id] = Some(CompiledBlob {
            ptr,
            size,
            relocs: reloc_sink.relocs,
        });
        // FIXME immediately perform relocations
        self.functions_to_finalize.push(id);
        unsafe {
            std::ptr::write(self.function_got_entries[id].unwrap().as_ptr(), ptr);
        }

        Ok(ModuleCompiledFunction { size: code_size })
    }

    fn define_function_bytes(
        &mut self,
        id: FuncId,
        bytes: &[u8],
        relocs: &[RelocRecord],
    ) -> ModuleResult<ModuleCompiledFunction> {
        info!("defining function {} with bytes", id);
        let total_size: u32 = match bytes.len().try_into() {
            Ok(total_size) => total_size,
            _ => Err(CodegenError::CodeTooLarge)?,
        };

        let decl = self.declarations.get_function_decl(id);
        if !decl.linkage.is_definable() {
            return Err(ModuleError::InvalidImportDefinition(decl.name.clone()));
        }

        if !self.compiled_functions[id].is_none() {
            return Err(ModuleError::DuplicateDefinition(decl.name.to_owned()));
        }

        let size = bytes.len();
        let ptr = self
            .memory
            .code
            .allocate(size, EXECUTABLE_DATA_ALIGNMENT)
            .expect("TODO: handle OOM etc.");

        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, size);
        }

        self.record_function_for_perf(ptr, size, &decl.name);
        self.compiled_functions[id] = Some(CompiledBlob {
            ptr,
            size,
            relocs: relocs.to_vec(),
        });
        self.functions_to_finalize.push(id);
        unsafe {
            std::ptr::write(self.function_got_entries[id].unwrap().as_ptr(), ptr);
        }

        Ok(ModuleCompiledFunction { size: total_size })
    }

    fn define_data(&mut self, id: DataId, data: &DataContext) -> ModuleResult<()> {
        let decl = self.declarations.get_data_decl(id);
        if !decl.linkage.is_definable() {
            return Err(ModuleError::InvalidImportDefinition(decl.name.clone()));
        }

        if !self.compiled_data_objects[id].is_none() {
            return Err(ModuleError::DuplicateDefinition(decl.name.to_owned()));
        }

        assert!(!decl.tls, "SimpleJIT doesn't yet support TLS");

        let &DataDescription {
            ref init,
            function_decls: _,
            data_decls: _,
            function_relocs: _,
            data_relocs: _,
            custom_segment_section: _,
            align,
        } = data.description();

        let size = init.size();
        let ptr = if decl.writable {
            self.memory
                .writable
                .allocate(size, align.unwrap_or(WRITABLE_DATA_ALIGNMENT))
                .expect("TODO: handle OOM etc.")
        } else {
            self.memory
                .readonly
                .allocate(size, align.unwrap_or(READONLY_DATA_ALIGNMENT))
                .expect("TODO: handle OOM etc.")
        };

        match *init {
            Init::Uninitialized => {
                panic!("data is not initialized yet");
            }
            Init::Zeros { .. } => {
                unsafe { ptr::write_bytes(ptr, 0, size) };
            }
            Init::Bytes { ref contents } => {
                let src = contents.as_ptr();
                unsafe { ptr::copy_nonoverlapping(src, ptr, size) };
            }
        }

        let pointer_reloc = match self.isa.triple().pointer_width().unwrap() {
            PointerWidth::U16 => panic!(),
            PointerWidth::U32 => Reloc::Abs4,
            PointerWidth::U64 => Reloc::Abs8,
        };
        let relocs = data
            .description()
            .all_relocs(pointer_reloc)
            .collect::<Vec<_>>();

        self.compiled_data_objects[id] = Some(CompiledBlob { ptr, size, relocs });
        self.data_objects_to_finalize.push(id);
        unsafe {
            std::ptr::write(self.data_object_got_entries[id].unwrap().as_ptr(), ptr);
        }

        Ok(())
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
    const MSVCRT_DLL: &[u8] = b"msvcrt.dll\0";

    let c_str = CString::new(name).unwrap();
    let c_str_ptr = c_str.as_ptr();

    unsafe {
        let handles = [
            // try to find the searched symbol in the currently running executable
            ptr::null_mut(),
            // try to find the searched symbol in local c runtime
            winapi::um::libloaderapi::GetModuleHandleA(MSVCRT_DLL.as_ptr() as *const i8),
        ];

        for handle in &handles {
            let addr = winapi::um::libloaderapi::GetProcAddress(*handle, c_str_ptr);
            if addr.is_null() {
                continue;
            }
            return Some(addr as *const u8);
        }

        None
    }
}

#[derive(Default)]
struct SimpleJITRelocSink {
    relocs: Vec<RelocRecord>,
}

impl RelocSink for SimpleJITRelocSink {
    fn reloc_external(
        &mut self,
        offset: CodeOffset,
        _srcloc: ir::SourceLoc,
        reloc: Reloc,
        name: &ir::ExternalName,
        addend: Addend,
    ) {
        self.relocs.push(RelocRecord {
            offset,
            reloc,
            name: name.clone(),
            addend,
        });
    }

    fn reloc_jt(&mut self, _offset: CodeOffset, reloc: Reloc, _jt: ir::JumpTable) {
        match reloc {
            Reloc::X86PCRelRodata4 => {
                // Not necessary to record this unless we are going to split apart code and its
                // jumptbl/rodata.
            }
            _ => {
                panic!("Unhandled reloc");
            }
        }
    }

    fn reloc_constant(&mut self, _offset: CodeOffset, reloc: Reloc, _constant: ir::ConstantOffset) {
        match reloc {
            Reloc::X86PCRelRodata4 => {
                // Not necessary to record this unless we are going to split apart code and its
                // jumptbl/rodata.
            }
            _ => {
                panic!("Unhandled reloc");
            }
        }
    }
}
