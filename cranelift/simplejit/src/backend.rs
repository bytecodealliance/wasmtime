//! Defines `SimpleJITBackend`.

use crate::memory::Memory;
use cranelift_codegen::binemit::{
    Addend, CodeOffset, NullTrapSink, Reloc, RelocSink, Stackmap, StackmapSink,
};
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::{self, ir, settings};
use cranelift_module::{
    Backend, DataContext, DataDescription, DataId, FuncId, Init, Linkage, ModuleNamespace,
    ModuleResult, TrapSite,
};
use cranelift_native;
#[cfg(not(windows))]
use libc;
use std::collections::HashMap;
use std::ffi::CString;
use std::io::Write;
use std::ptr;
use target_lexicon::PointerWidth;
#[cfg(windows)]
use winapi;

const EXECUTABLE_DATA_ALIGNMENT: u8 = 0x10;
const WRITABLE_DATA_ALIGNMENT: u8 = 0x8;
const READONLY_DATA_ALIGNMENT: u8 = 0x1;

/// A builder for `SimpleJITBackend`.
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
        let flag_builder = settings::builder();
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

/// A `SimpleJITBackend` implements `Backend` and emits code and data into memory where it can be
/// directly called and accessed.
///
/// See the `SimpleJITBuilder` for a convenient way to construct `SimpleJITBackend` instances.
pub struct SimpleJITBackend {
    isa: Box<dyn TargetIsa>,
    symbols: HashMap<String, *const u8>,
    libcall_names: Box<dyn Fn(ir::LibCall) -> String>,
    memory: SimpleJITMemoryHandle,
}

/// A record of a relocation to perform.
struct RelocRecord {
    offset: CodeOffset,
    reloc: Reloc,
    name: ir::ExternalName,
    addend: Addend,
}

struct StackmapRecord {
    #[allow(dead_code)]
    offset: CodeOffset,
    #[allow(dead_code)]
    stackmap: Stackmap,
}

pub struct SimpleJITCompiledFunction {
    code: *mut u8,
    size: usize,
    relocs: Vec<RelocRecord>,
}

pub struct SimpleJITCompiledData {
    storage: *mut u8,
    size: usize,
    relocs: Vec<RelocRecord>,
}

/// A handle to allow freeing memory allocated by the `Backend`.
pub struct SimpleJITMemoryHandle {
    code: Memory,
    readonly: Memory,
    writable: Memory,
}

impl SimpleJITBackend {
    fn lookup_symbol(&self, name: &str) -> *const u8 {
        match self.symbols.get(name) {
            Some(&ptr) => ptr,
            None => lookup_with_dlsym(name),
        }
    }

    fn get_definition(
        &self,
        namespace: &ModuleNamespace<Self>,
        name: &ir::ExternalName,
    ) -> *const u8 {
        match *name {
            ir::ExternalName::User { .. } => {
                if namespace.is_function(name) {
                    let (def, name_str, _signature) = namespace.get_function_definition(&name);
                    match def {
                        Some(compiled) => compiled.code,
                        None => self.lookup_symbol(name_str),
                    }
                } else {
                    let (def, name_str, _writable) = namespace.get_data_definition(&name);
                    match def {
                        Some(compiled) => compiled.storage,
                        None => self.lookup_symbol(name_str),
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
}

impl<'simple_jit_backend> Backend for SimpleJITBackend {
    type Builder = SimpleJITBuilder;

    /// SimpleJIT compiled function and data objects may have outstanding
    /// relocations that need to be performed before the memory can be used.
    /// These relocations are performed within `finalize_function` and
    /// `finalize_data`.
    type CompiledFunction = SimpleJITCompiledFunction;
    type CompiledData = SimpleJITCompiledData;

    /// SimpleJIT emits code and data into memory, and provides raw pointers
    /// to them. They are valid for the remainder of the program's life, unless
    /// [`free_memory`] is used.
    ///
    /// [`free_memory`]: #method.free_memory
    type FinalizedFunction = *const u8;
    type FinalizedData = (*mut u8, usize);

    /// SimpleJIT emits code and data into memory as it processes them, so it
    /// doesn't need to provide anything after the `Module` is complete.
    /// The handle object that is returned can optionally be used to free
    /// allocated memory if required.
    type Product = SimpleJITMemoryHandle;

    /// Create a new `SimpleJITBackend`.
    fn new(builder: SimpleJITBuilder) -> Self {
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
        }
    }

    fn isa(&self) -> &dyn TargetIsa {
        &*self.isa
    }

    fn declare_function(&mut self, _id: FuncId, _name: &str, _linkage: Linkage) {
        // Nothing to do.
    }

    fn declare_data(
        &mut self,
        _id: DataId,
        _name: &str,
        _linkage: Linkage,
        _writable: bool,
        tls: bool,
        _align: Option<u8>,
    ) {
        assert!(!tls, "SimpleJIT doesn't yet support TLS");
        // Nothing to do.
    }

    fn define_function(
        &mut self,
        _id: FuncId,
        name: &str,
        ctx: &cranelift_codegen::Context,
        _namespace: &ModuleNamespace<Self>,
        code_size: u32,
    ) -> ModuleResult<Self::CompiledFunction> {
        let size = code_size as usize;
        let ptr = self
            .memory
            .code
            .allocate(size, EXECUTABLE_DATA_ALIGNMENT)
            .expect("TODO: handle OOM etc.");

        self.record_function_for_perf(ptr, size, name);

        let mut reloc_sink = SimpleJITRelocSink::new();
        // Ignore traps for now. For now, frontends should just avoid generating code
        // that traps.
        let mut trap_sink = NullTrapSink {};
        let mut stackmap_sink = SimpleJITStackmapSink::new();
        unsafe {
            ctx.emit_to_memory(
                &*self.isa,
                ptr,
                &mut reloc_sink,
                &mut trap_sink,
                &mut stackmap_sink,
            )
        };

        Ok(Self::CompiledFunction {
            code: ptr,
            size,
            relocs: reloc_sink.relocs,
        })
    }

    fn define_function_bytes(
        &mut self,
        _id: FuncId,
        name: &str,
        bytes: &[u8],
        _namespace: &ModuleNamespace<Self>,
        _traps: Vec<TrapSite>,
    ) -> ModuleResult<Self::CompiledFunction> {
        let size = bytes.len();
        let ptr = self
            .memory
            .code
            .allocate(size, EXECUTABLE_DATA_ALIGNMENT)
            .expect("TODO: handle OOM etc.");

        self.record_function_for_perf(ptr, size, name);

        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, size);
        }

        Ok(Self::CompiledFunction {
            code: ptr,
            size,
            relocs: vec![],
        })
    }

    fn define_data(
        &mut self,
        _id: DataId,
        _name: &str,
        writable: bool,
        tls: bool,
        align: Option<u8>,
        data: &DataContext,
        _namespace: &ModuleNamespace<Self>,
    ) -> ModuleResult<Self::CompiledData> {
        assert!(!tls, "SimpleJIT doesn't yet support TLS");

        let &DataDescription {
            ref init,
            ref function_decls,
            ref data_decls,
            ref function_relocs,
            ref data_relocs,
        } = data.description();

        let size = init.size();
        let storage = if writable {
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

        Ok(Self::CompiledData {
            storage,
            size,
            relocs,
        })
    }

    fn write_data_funcaddr(
        &mut self,
        _data: &mut Self::CompiledData,
        _offset: usize,
        _what: ir::FuncRef,
    ) {
        unimplemented!();
    }

    fn write_data_dataaddr(
        &mut self,
        _data: &mut Self::CompiledData,
        _offset: usize,
        _what: ir::GlobalValue,
        _usize: Addend,
    ) {
        unimplemented!();
    }

    fn finalize_function(
        &mut self,
        _id: FuncId,
        func: &Self::CompiledFunction,
        namespace: &ModuleNamespace<Self>,
    ) -> Self::FinalizedFunction {
        use std::ptr::write_unaligned;

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
            let base = self.get_definition(namespace, name);
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
        func.code
    }

    fn get_finalized_function(&self, func: &Self::CompiledFunction) -> Self::FinalizedFunction {
        func.code
    }

    fn finalize_data(
        &mut self,
        _id: DataId,
        data: &Self::CompiledData,
        namespace: &ModuleNamespace<Self>,
    ) -> Self::FinalizedData {
        use std::ptr::write_unaligned;

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
            let base = self.get_definition(namespace, name);
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
        (data.storage, data.size)
    }

    fn get_finalized_data(&self, data: &Self::CompiledData) -> Self::FinalizedData {
        (data.storage, data.size)
    }

    fn publish(&mut self) {
        // Now that we're done patching, prepare the memory for execution!
        self.memory.readonly.set_readonly();
        self.memory.code.set_readable_and_executable();
    }

    /// SimpleJIT emits code and data into memory as it processes them. This
    /// method performs no additional processing, but returns a handle which
    /// allows freeing the allocated memory. Otherwise said memory is leaked
    /// to enable safe handling of the resulting pointers.
    ///
    /// This method does not need to be called when access to the memory
    /// handle is not required.
    fn finish(self, _namespace: &ModuleNamespace<Self>) -> Self::Product {
        self.memory
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

impl SimpleJITMemoryHandle {
    /// Free memory allocated for code and data segments of compiled functions.
    ///
    /// # Safety
    ///
    /// Because this function invalidates any pointers retrived from the
    /// corresponding module, it should only be used when none of the functions
    /// from that module are currently executing and none of the`fn` pointers
    /// are called afterwards.
    pub unsafe fn free_memory(&mut self) {
        self.code.free_memory();
        self.readonly.free_memory();
        self.writable.free_memory();
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

struct SimpleJITStackmapSink {
    pub stackmaps: Vec<StackmapRecord>,
}

impl SimpleJITStackmapSink {
    pub fn new() -> Self {
        Self {
            stackmaps: Vec::new(),
        }
    }
}

impl StackmapSink for SimpleJITStackmapSink {
    fn add_stackmap(&mut self, offset: CodeOffset, stackmap: Stackmap) {
        self.stackmaps.push(StackmapRecord { offset, stackmap });
    }
}
