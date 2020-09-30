//! Defines `SimpleJITModule`.

use crate::memory::Memory;
use cranelift_codegen::binemit::{
    Addend, CodeInfo, CodeOffset, Reloc, RelocSink, StackMap, StackMapSink, TrapSink,
};
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::settings::Configurable;
use cranelift_codegen::{self, ir, settings};
use cranelift_entity::SecondaryMap;
use cranelift_module::{
    DataContext, DataDescription, DataId, FuncId, FuncOrDataId, Init, Linkage, Module,
    ModuleCompiledFunction, ModuleDeclarations, ModuleError, ModuleResult,
};
use cranelift_native;
#[cfg(not(windows))]
use libc;
use log::info;
use std::collections::HashMap;
use std::convert::TryInto;
use std::ffi::CString;
use std::io::Write;
use std::ptr;
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
    libcall_names: Box<dyn Fn(ir::LibCall) -> String>,
}

impl SimpleJITBuilder {
    /// Create a new `SimpleJITBuilder`.
    ///
    /// The `libcall_names` function provides a way to translate `cranelift_codegen`'s `ir::LibCall`
    /// enum to symbols. LibCalls are inserted in the IR as part of the legalization for certain
    /// floating point instructions, and for stack probes. If you don't know what to use for this
    /// argument, use `cranelift_module::default_libcall_names()`.
    pub fn new(libcall_names: Box<dyn Fn(ir::LibCall) -> String>) -> Self {
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
        libcall_names: Box<dyn Fn(ir::LibCall) -> String>,
    ) -> Self {
        debug_assert!(!isa.flags().is_pic(), "SimpleJIT requires non-PIC code");
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
    memory: SimpleJITMemoryHandle,
    declarations: ModuleDeclarations,
    functions: SecondaryMap<FuncId, Option<SimpleJITCompiledFunction>>,
    data_objects: SecondaryMap<DataId, Option<SimpleJITCompiledData>>,
    functions_to_finalize: Vec<FuncId>,
    data_objects_to_finalize: Vec<DataId>,
}

/// A record of a relocation to perform.
#[derive(Clone)]
struct RelocRecord {
    offset: CodeOffset,
    reloc: Reloc,
    name: ir::ExternalName,
    addend: Addend,
}

struct StackMapRecord {
    #[allow(dead_code)]
    offset: CodeOffset,
    #[allow(dead_code)]
    stack_map: StackMap,
}

#[derive(Clone)]
pub struct SimpleJITCompiledFunction {
    code: *mut u8,
    size: usize,
    relocs: Vec<RelocRecord>,
}

#[derive(Clone)]
pub struct SimpleJITCompiledData {
    storage: *mut u8,
    size: usize,
    relocs: Vec<RelocRecord>,
}

/// A handle to allow freeing memory allocated by the `Backend`.
struct SimpleJITMemoryHandle {
    code: Memory,
    readonly: Memory,
    writable: Memory,
}

/// A `SimpleJITProduct` allows looking up the addresses of all functions and data objects
/// defined in the original module.
pub struct SimpleJITProduct {
    memory: SimpleJITMemoryHandle,
    declarations: ModuleDeclarations,
    functions: SecondaryMap<FuncId, Option<SimpleJITCompiledFunction>>,
    data_objects: SecondaryMap<DataId, Option<SimpleJITCompiledData>>,
}

impl SimpleJITProduct {
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

    /// Get the `FuncOrDataId` associated with the given name.
    pub fn func_or_data_for_func(&self, name: &str) -> Option<FuncOrDataId> {
        self.declarations.get_name(name)
    }

    /// Return the address of a function.
    pub fn lookup_func(&self, func_id: FuncId) -> *const u8 {
        self.functions[func_id]
            .as_ref()
            .unwrap_or_else(|| panic!("{} is not defined", func_id))
            .code
    }

    /// Return the address and size of a data object.
    pub fn lookup_data(&self, data_id: DataId) -> (*const u8, usize) {
        let data = self.data_objects[data_id]
            .as_ref()
            .unwrap_or_else(|| panic!("{} is not defined", data_id));
        (data.storage, data.size)
    }
}

impl SimpleJITModule {
    fn lookup_symbol(&self, name: &str) -> *const u8 {
        match self.symbols.get(name) {
            Some(&ptr) => ptr,
            None => lookup_with_dlsym(name),
        }
    }

    fn get_definition(&self, name: &ir::ExternalName) -> *const u8 {
        match *name {
            ir::ExternalName::User { .. } => {
                if self.declarations.is_function(name) {
                    let func_id = self.declarations.get_function_id(name);
                    match &self.functions[func_id] {
                        Some(compiled) => compiled.code,
                        None => {
                            self.lookup_symbol(&self.declarations.get_function_decl(func_id).name)
                        }
                    }
                } else {
                    let data_id = self.declarations.get_data_id(name);
                    match &self.data_objects[data_id] {
                        Some(compiled) => compiled.storage,
                        None => self.lookup_symbol(&self.declarations.get_data_decl(data_id).name),
                    }
                }
            }
            ir::ExternalName::LibCall(ref libcall) => {
                let sym = (self.libcall_names)(*libcall);
                self.lookup_symbol(&sym)
            }
            _ => panic!("invalid ExternalName {}", name),
        }
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

    fn finalize_function(&mut self, id: FuncId) {
        use std::ptr::write_unaligned;

        let func = self.functions[id]
            .as_ref()
            .expect("function must be compiled before it can be finalized");

        for &RelocRecord {
            reloc,
            offset,
            ref name,
            addend,
        } in &func.relocs
        {
            let ptr = func.code;
            debug_assert!((offset as usize) < func.size);
            let at = unsafe { ptr.offset(offset as isize) };
            let base = self.get_definition(name);
            // TODO: Handle overflow.
            let what = unsafe { base.offset(addend as isize) };
            match reloc {
                Reloc::Abs4 => {
                    // TODO: Handle overflow.
                    #[cfg_attr(feature = "cargo-clippy", allow(clippy::cast_ptr_alignment))]
                    unsafe {
                        write_unaligned(at as *mut u32, what as u32)
                    };
                }
                Reloc::Abs8 => {
                    #[cfg_attr(feature = "cargo-clippy", allow(clippy::cast_ptr_alignment))]
                    unsafe {
                        write_unaligned(at as *mut u64, what as u64)
                    };
                }
                Reloc::X86PCRel4 | Reloc::X86CallPCRel4 => {
                    // TODO: Handle overflow.
                    let pcrel = ((what as isize) - (at as isize)) as i32;
                    #[cfg_attr(feature = "cargo-clippy", allow(clippy::cast_ptr_alignment))]
                    unsafe {
                        write_unaligned(at as *mut i32, pcrel)
                    };
                }
                Reloc::X86GOTPCRel4 | Reloc::X86CallPLTRel4 => panic!("unexpected PIC relocation"),
                _ => unimplemented!(),
            }
        }
    }

    fn finalize_data(&mut self, id: DataId) {
        use std::ptr::write_unaligned;

        let data = self.data_objects[id]
            .as_ref()
            .expect("data object must be compiled before it can be finalized");

        for &RelocRecord {
            reloc,
            offset,
            ref name,
            addend,
        } in &data.relocs
        {
            let ptr = data.storage;
            debug_assert!((offset as usize) < data.size);
            let at = unsafe { ptr.offset(offset as isize) };
            let base = self.get_definition(name);
            // TODO: Handle overflow.
            let what = unsafe { base.offset(addend as isize) };
            match reloc {
                Reloc::Abs4 => {
                    // TODO: Handle overflow.
                    #[cfg_attr(feature = "cargo-clippy", allow(clippy::cast_ptr_alignment))]
                    unsafe {
                        write_unaligned(at as *mut u32, what as u32)
                    };
                }
                Reloc::Abs8 => {
                    #[cfg_attr(feature = "cargo-clippy", allow(clippy::cast_ptr_alignment))]
                    unsafe {
                        write_unaligned(at as *mut u64, what as u64)
                    };
                }
                Reloc::X86PCRel4
                | Reloc::X86CallPCRel4
                | Reloc::X86GOTPCRel4
                | Reloc::X86CallPLTRel4 => panic!("unexpected text relocation in data"),
                _ => unimplemented!(),
            }
        }
    }

    /// Create a new `SimpleJITBackend`.
    pub fn new(builder: SimpleJITBuilder) -> Self {
        let memory = SimpleJITMemoryHandle {
            code: Memory::new(),
            readonly: Memory::new(),
            writable: Memory::new(),
        };

        Self {
            isa: builder.isa,
            symbols: builder.symbols,
            libcall_names: builder.libcall_names,
            memory,
            declarations: ModuleDeclarations::default(),
            functions: SecondaryMap::new(),
            data_objects: SecondaryMap::new(),
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
        Ok(id)
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

        if !self.functions[id].is_none() {
            return Err(ModuleError::DuplicateDefinition(decl.name.to_owned()));
        }

        self.functions_to_finalize.push(id);
        let size = code_size as usize;
        let ptr = self
            .memory
            .code
            .allocate(size, EXECUTABLE_DATA_ALIGNMENT)
            .expect("TODO: handle OOM etc.");

        self.record_function_for_perf(ptr, size, &decl.name);

        let mut reloc_sink = SimpleJITRelocSink::new();
        let mut stack_map_sink = SimpleJITStackMapSink::new();
        unsafe {
            ctx.emit_to_memory(
                &*self.isa,
                ptr,
                &mut reloc_sink,
                trap_sink,
                &mut stack_map_sink,
            )
        };

        self.functions[id] = Some(SimpleJITCompiledFunction {
            code: ptr,
            size,
            relocs: reloc_sink.relocs,
        });

        Ok(ModuleCompiledFunction { size: code_size })
    }

    fn define_function_bytes(
        &mut self,
        id: FuncId,
        bytes: &[u8],
    ) -> ModuleResult<ModuleCompiledFunction> {
        let decl = self.declarations.get_function_decl(id);
        if !decl.linkage.is_definable() {
            return Err(ModuleError::InvalidImportDefinition(decl.name.clone()));
        }

        let total_size: u32 = match bytes.len().try_into() {
            Ok(total_size) => total_size,
            _ => Err(ModuleError::FunctionTooLarge(decl.name.clone()))?,
        };

        if !self.functions[id].is_none() {
            return Err(ModuleError::DuplicateDefinition(decl.name.to_owned()));
        }

        self.functions_to_finalize.push(id);
        let size = bytes.len();
        let ptr = self
            .memory
            .code
            .allocate(size, EXECUTABLE_DATA_ALIGNMENT)
            .expect("TODO: handle OOM etc.");

        self.record_function_for_perf(ptr, size, &decl.name);

        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, size);
        }

        self.functions[id] = Some(SimpleJITCompiledFunction {
            code: ptr,
            size,
            relocs: vec![],
        });

        Ok(ModuleCompiledFunction { size: total_size })
    }

    fn define_data(&mut self, id: DataId, data: &DataContext) -> ModuleResult<()> {
        let decl = self.declarations.get_data_decl(id);
        if !decl.linkage.is_definable() {
            return Err(ModuleError::InvalidImportDefinition(decl.name.clone()));
        }

        if !self.data_objects[id].is_none() {
            return Err(ModuleError::DuplicateDefinition(decl.name.to_owned()));
        }

        assert!(!decl.tls, "SimpleJIT doesn't yet support TLS");

        self.data_objects_to_finalize.push(id);

        let &DataDescription {
            ref init,
            ref function_decls,
            ref data_decls,
            ref function_relocs,
            ref data_relocs,
            custom_segment_section: _,
            align,
        } = data.description();

        let size = init.size();
        let storage = if decl.writable {
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
                unsafe { ptr::write_bytes(storage, 0, size) };
            }
            Init::Bytes { ref contents } => {
                let src = contents.as_ptr();
                unsafe { ptr::copy_nonoverlapping(src, storage, size) };
            }
        }

        let reloc = match self.isa.triple().pointer_width().unwrap() {
            PointerWidth::U16 => panic!(),
            PointerWidth::U32 => Reloc::Abs4,
            PointerWidth::U64 => Reloc::Abs8,
        };
        let mut relocs = Vec::new();
        for &(offset, id) in function_relocs {
            relocs.push(RelocRecord {
                reloc,
                offset,
                name: function_decls[id].clone(),
                addend: 0,
            });
        }
        for &(offset, id, addend) in data_relocs {
            relocs.push(RelocRecord {
                reloc,
                offset,
                name: data_decls[id].clone(),
                addend,
            });
        }

        self.data_objects[id] = Some(SimpleJITCompiledData {
            storage,
            size,
            relocs,
        });

        Ok(())
    }
}

impl SimpleJITModule {
    /// SimpleJIT emits code and data into memory as it processes them. This
    /// method performs no additional processing, but returns a handle which
    /// allows freeing the allocated memory. Otherwise said memory is leaked
    /// to enable safe handling of the resulting pointers.
    ///
    /// This method does not need to be called when access to the memory
    /// handle is not required.
    pub fn finish(mut self) -> SimpleJITProduct {
        for func in std::mem::take(&mut self.functions_to_finalize) {
            let decl = self.declarations.get_function_decl(func);
            debug_assert!(decl.linkage.is_definable());
            self.finalize_function(func);
        }
        for data in std::mem::take(&mut self.data_objects_to_finalize) {
            let decl = self.declarations.get_data_decl(data);
            debug_assert!(decl.linkage.is_definable());
            self.finalize_data(data);
        }

        // Now that we're done patching, prepare the memory for execution!
        self.memory.readonly.set_readonly();
        self.memory.code.set_readable_and_executable();

        SimpleJITProduct {
            memory: self.memory,
            declarations: self.declarations,
            functions: self.functions,
            data_objects: self.data_objects,
        }
    }
}

#[cfg(not(windows))]
fn lookup_with_dlsym(name: &str) -> *const u8 {
    let c_str = CString::new(name).unwrap();
    let c_str_ptr = c_str.as_ptr();
    let sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c_str_ptr) };
    if sym.is_null() {
        panic!("can't resolve symbol {}", name);
    }
    sym as *const u8
}

#[cfg(windows)]
fn lookup_with_dlsym(name: &str) -> *const u8 {
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
            return addr as *const u8;
        }

        let msg = if handles[1].is_null() {
            "(msvcrt not loaded)"
        } else {
            ""
        };
        panic!("cannot resolve address of symbol {} {}", name, msg);
    }
}

struct SimpleJITRelocSink {
    pub relocs: Vec<RelocRecord>,
}

impl SimpleJITRelocSink {
    pub fn new() -> Self {
        Self { relocs: Vec::new() }
    }
}

impl RelocSink for SimpleJITRelocSink {
    fn reloc_block(&mut self, _offset: CodeOffset, _reloc: Reloc, _block_offset: CodeOffset) {
        unimplemented!();
    }

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

struct SimpleJITStackMapSink {
    pub stack_maps: Vec<StackMapRecord>,
}

impl SimpleJITStackMapSink {
    pub fn new() -> Self {
        Self {
            stack_maps: Vec::new(),
        }
    }
}

impl StackMapSink for SimpleJITStackMapSink {
    fn add_stack_map(&mut self, offset: CodeOffset, stack_map: StackMap) {
        self.stack_maps.push(StackMapRecord { offset, stack_map });
    }
}
