//! Defines `SimpleJITBackend`.

use cretonne_codegen::binemit::{Addend, CodeOffset, Reloc, RelocSink, NullTrapSink};
use cretonne_codegen::isa::TargetIsa;
use cretonne_codegen::result::CtonError;
use cretonne_codegen::{self, ir, settings};
use cretonne_module::{Backend, DataContext, Linkage, ModuleNamespace, Writability,
                      DataDescription, Init};
use cretonne_native;
use std::ffi::CString;
use std::ptr;
use libc;
use memory::Memory;

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

/// A `SimpleJITBackend` implements `Backend` and emits code and data into memory where it can be
/// directly called and accessed.
pub struct SimpleJITBackend {
    isa: Box<TargetIsa>,
    code_memory: Memory,
    readonly_memory: Memory,
    writable_memory: Memory,
}

impl SimpleJITBackend {
    /// Create a new `SimpleJITBackend`.
    pub fn new() -> Self {
        let (flag_builder, isa_builder) = cretonne_native::builders().unwrap_or_else(|_| {
            panic!("host machine is not a supported target");
        });
        let isa = isa_builder.finish(settings::Flags::new(&flag_builder));
        Self::with_isa(isa)
    }

    /// Create a new `SimpleJITBackend` with an arbitrary target. This is mainly
    /// useful for testing.
    ///
    /// SimpleJIT requires a `TargetIsa` configured for non-PIC.
    ///
    /// To create a `SimpleJITBackend` for native use, use the `new` constructor
    /// instead.
    pub fn with_isa(isa: Box<TargetIsa>) -> Self {
        debug_assert!(!isa.flags().is_pic(), "SimpleJIT requires non-PIC code");
        Self {
            isa,
            code_memory: Memory::new(),
            readonly_memory: Memory::new(),
            writable_memory: Memory::new(),
        }
    }
}

impl<'simple_jit_backend> Backend for SimpleJITBackend {
    type CompiledFunction = SimpleJITCompiledFunction;
    type CompiledData = SimpleJITCompiledData;

    type FinalizedFunction = *const u8;
    type FinalizedData = (*mut u8, usize);

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
        ctx: &cretonne_codegen::Context,
        _namespace: &ModuleNamespace<Self>,
        code_size: u32,
    ) -> Result<Self::CompiledFunction, CtonError> {
        let size = code_size as usize;
        let ptr = self.code_memory.allocate(size).expect(
            "TODO: handle OOM etc.",
        );
        let mut reloc_sink = SimpleJITRelocSink::new();
        // Ignore traps for now. For now, frontends should just avoid generating code
        // that traps.
        let mut trap_sink = NullTrapSink {};
        ctx.emit_to_memory(ptr, &mut reloc_sink, &mut trap_sink, &*self.isa);

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
    ) -> Result<Self::CompiledData, CtonError> {
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
            Writability::Readonly => {
                self.writable_memory.allocate(size).expect(
                    "TODO: handle OOM etc.",
                )
            }
            Writability::Writable => {
                self.writable_memory.allocate(size).expect(
                    "TODO: handle OOM etc.",
                )
            }
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

        let reloc = if self.isa.flags().is_64bit() {
            Reloc::Abs8
        } else {
            Reloc::Abs4
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
        _what: ir::GlobalVar,
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
                    None => lookup_with_dlsym(name_str),
                }
            } else {
                let (def, name_str, _writable) = namespace.get_data_definition(&name);
                match def {
                    Some(compiled) => compiled.storage,
                    None => lookup_with_dlsym(name_str),
                }
            };
            // TODO: Handle overflow.
            let what = unsafe { base.offset(addend as isize) };
            match reloc {
                Reloc::Abs4 => {
                    // TODO: Handle overflow.
                    #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
                    unsafe { write_unaligned(at as *mut u32, what as u32) };
                }
                Reloc::Abs8 => {
                    #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
                    unsafe { write_unaligned(at as *mut u64, what as u64) };
                }
                Reloc::X86PCRel4 => {
                    // TODO: Handle overflow.
                    let pcrel = ((what as isize) - (at as isize)) as i32;
                    #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
                    unsafe { write_unaligned(at as *mut i32, pcrel) };
                }
                Reloc::X86GOTPCRel4 |
                Reloc::X86PLTRel4 => panic!("unexpected PIC relocation"),
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
                            None => lookup_with_dlsym(name_str),
                        }
                    } else {
                        let (def, name_str, _writable) = namespace.get_data_definition(&name);
                        match def {
                            Some(compiled) => compiled.storage,
                            None => lookup_with_dlsym(name_str),
                        }
                    };
                    // TODO: Handle overflow.
                    let what = unsafe { base.offset(addend as isize) };
                    match reloc {
                        Reloc::Abs4 => {
                            // TODO: Handle overflow.
                            #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
                            unsafe { write_unaligned(at as *mut u32, what as u32) };
                        }
                        Reloc::Abs8 => {
                            #[cfg_attr(feature = "cargo-clippy", allow(cast_ptr_alignment))]
                            unsafe { write_unaligned(at as *mut u64, what as u64) };
                        }
                        Reloc::X86PCRel4 |
                        Reloc::X86GOTPCRel4 |
                        Reloc::X86PLTRel4 => panic!("unexpected text relocation in data"),
                        _ => unimplemented!(),
                    }
                }
            }
        }

        self.readonly_memory.set_readonly();
        (data.storage, data.size)
    }
}

fn lookup_with_dlsym(name: &str) -> *const u8 {
    let c_str = CString::new(name).unwrap();
    let c_str_ptr = c_str.as_ptr();
    let sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c_str_ptr) };
    if sym.is_null() {
        panic!("can't resolve symbol {}", name);
    }
    sym as *const u8
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
