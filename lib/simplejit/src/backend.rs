//! Defines `SimpleJITBackend`.

use cranelift_codegen::binemit::{Addend, CodeOffset, NullTrapSink, Reloc, RelocSink};
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::{self, ir, settings};
use cranelift_module::{
    Backend, DataContext, DataDescription, Init, Linkage, ModuleNamespace, ModuleResult,
    Writability,
};
use cranelift_native;
use libc;
use memory::Memory;
use std::collections::HashMap;
use std::ffi::CString;
use std::ptr;
use target_lexicon::PointerWidth;
#[cfg(windows)]
use winapi;

/// A builder for `SimpleJITBackend`.
pub struct SimpleJITBuilder {
    isa: Box<TargetIsa>,
    symbols: HashMap<String, *const u8>,
}

impl SimpleJITBuilder {
    /// Create a new `SimpleJITBuilder`.
    pub fn new() -> Self {
        let (flag_builder, isa_builder) = cranelift_native::builders().unwrap_or_else(|_| {
            panic!("host machine is not a supported target");
        });
        let isa = isa_builder.finish(settings::Flags::new(flag_builder));
        Self::with_isa(isa)
    }

    /// Create a new `SimpleJITBuilder` with an arbitrary target. This is mainly
    /// useful for testing.
    ///
    /// SimpleJIT requires a `TargetIsa` configured for non-PIC.
    ///
    /// To create a `SimpleJITBuilder` for native use, use the `new` constructor
    /// instead.
    pub fn with_isa(isa: Box<TargetIsa>) -> Self {
        debug_assert!(!isa.flags().is_pic(), "SimpleJIT requires non-PIC code");
        let symbols = HashMap::new();
        Self { isa, symbols }
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
    pub fn symbol<'a, K>(&'a mut self, name: K, ptr: *const u8) -> &'a mut Self
    where
        K: Into<String>,
    {
        self.symbols.insert(name.into(), ptr);
        self
    }

    /// Define multiple symbols in the internal symbol table.
    ///
    /// Using this is equivalent to calling `symbol` on each element.
    pub fn symbols<'a, It, K>(&'a mut self, symbols: It) -> &'a mut Self
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
pub struct SimpleJITBackend {
    isa: Box<TargetIsa>,
    symbols: HashMap<String, *const u8>,
    code_memory: Memory,
    readonly_memory: Memory,
    writable_memory: Memory,
}

/// A record of a relocation to perform.
struct RelocRecord {
    offset: CodeOffset,
    reloc: Reloc,
    name: ir::ExternalName,
    addend: Addend,
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

impl SimpleJITBackend {
    fn lookup_symbol(&self, name: &str) -> *const u8 {
        match self.symbols.get(name) {
            Some(&ptr) => ptr,
            None => lookup_with_dlsym(name),
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
    /// to them.
    type FinalizedFunction = *const u8;
    type FinalizedData = (*mut u8, usize);

    /// SimpleJIT emits code and data into memory as it processes them, so it
    /// doesn't need to provide anything after the `Module` is complete.
    type Product = ();

    /// Create a new `SimpleJITBackend`.
    fn new(builder: SimpleJITBuilder) -> Self {
        Self {
            isa: builder.isa,
            symbols: builder.symbols,
            code_memory: Memory::new(),
            readonly_memory: Memory::new(),
            writable_memory: Memory::new(),
        }
    }

    fn isa(&self) -> &TargetIsa {
        &*self.isa
    }

    fn declare_function(&mut self, _name: &str, _linkage: Linkage) {
        // Nothing to do.
    }

    fn declare_data(&mut self, _name: &str, _linkage: Linkage, _writable: bool) {
        // Nothing to do.
    }

    fn define_function(
        &mut self,
        _name: &str,
        ctx: &cranelift_codegen::Context,
        _namespace: &ModuleNamespace<Self>,
        code_size: u32,
    ) -> ModuleResult<Self::CompiledFunction> {
        let size = code_size as usize;
        let ptr = self
            .code_memory
            .allocate(size)
            .expect("TODO: handle OOM etc.");
        let mut reloc_sink = SimpleJITRelocSink::new();
        // Ignore traps for now. For now, frontends should just avoid generating code
        // that traps.
        let mut trap_sink = NullTrapSink {};
        unsafe { ctx.emit_to_memory(&*self.isa, ptr, &mut reloc_sink, &mut trap_sink) };

        Ok(Self::CompiledFunction {
            code: ptr,
            size,
            relocs: reloc_sink.relocs,
        })
    }

    fn define_data(
        &mut self,
        _name: &str,
        data: &DataContext,
        _namespace: &ModuleNamespace<Self>,
    ) -> ModuleResult<Self::CompiledData> {
        let &DataDescription {
            writable,
            ref init,
            ref function_decls,
            ref data_decls,
            ref function_relocs,
            ref data_relocs,
        } = data.description();

        let size = init.size();
        let storage = match writable {
            Writability::Readonly => self
                .writable_memory
                .allocate(size)
                .expect("TODO: handle OOM etc."),
            Writability::Writable => self
                .writable_memory
                .allocate(size)
                .expect("TODO: handle OOM etc."),
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
            let base = if namespace.is_function(name) {
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
            };
            // TODO: Handle overflow.
            let what = unsafe { base.offset(addend as isize) };
            match reloc {
                Reloc::Abs4 => {
                    // TODO: Handle overflow.
                    #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
                    unsafe {
                        write_unaligned(at as *mut u32, what as u32)
                    };
                }
                Reloc::Abs8 => {
                    #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
                    unsafe {
                        write_unaligned(at as *mut u64, what as u64)
                    };
                }
                Reloc::X86PCRel4 | Reloc::X86CallPCRel4 => {
                    // TODO: Handle overflow.
                    let pcrel = ((what as isize) - (at as isize)) as i32;
                    #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
                    unsafe {
                        write_unaligned(at as *mut i32, pcrel)
                    };
                }
                Reloc::X86GOTPCRel4 | Reloc::X86CallPLTRel4 => panic!("unexpected PIC relocation"),
                _ => unimplemented!(),
            }
        }

        // Now that we're done patching, make the memory executable.
        self.code_memory.set_executable();
        func.code
    }

    fn finalize_data(
        &mut self,
        data: &Self::CompiledData,
        namespace: &ModuleNamespace<Self>,
    ) -> Self::FinalizedData {
        use std::ptr::write_unaligned;

        for record in &data.relocs {
            match *record {
                RelocRecord {
                    reloc,
                    offset,
                    ref name,
                    addend,
                } => {
                    let ptr = data.storage;
                    debug_assert!((offset as usize) < data.size);
                    let at = unsafe { ptr.offset(offset as isize) };
                    let base = if namespace.is_function(name) {
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
                    };
                    // TODO: Handle overflow.
                    let what = unsafe { base.offset(addend as isize) };
                    match reloc {
                        Reloc::Abs4 => {
                            // TODO: Handle overflow.
                            #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
                            unsafe {
                                write_unaligned(at as *mut u32, what as u32)
                            };
                        }
                        Reloc::Abs8 => {
                            #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
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
        }

        self.readonly_memory.set_readonly();
        (data.storage, data.size)
    }

    /// SimpleJIT emits code and data into memory as it processes them, so it
    /// doesn't need to provide anything after the `Module` is complete.
    fn finish(self) -> () {}
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
    fn reloc_ebb(&mut self, _offset: CodeOffset, _reloc: Reloc, _ebb_offset: CodeOffset) {
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

    fn reloc_jt(&mut self, _offset: CodeOffset, _reloc: Reloc, _jt: ir::JumpTable) {
        unimplemented!();
    }
}
