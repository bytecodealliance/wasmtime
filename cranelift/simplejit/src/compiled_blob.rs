use cranelift_codegen::binemit::Reloc;
use cranelift_codegen::ir::ExternalName;
use cranelift_module::RelocRecord;

#[derive(Clone)]
pub(crate) struct CompiledBlob {
    pub(crate) ptr: *mut u8,
    pub(crate) size: usize,
    pub(crate) relocs: Vec<RelocRecord>,
}

impl CompiledBlob {
    pub(crate) fn perform_relocations(&self, get_definition: impl Fn(&ExternalName) -> *const u8) {
        use std::ptr::write_unaligned;

        for &RelocRecord {
            reloc,
            offset,
            ref name,
            addend,
        } in &self.relocs
        {
            debug_assert!((offset as usize) < self.size);
            let at = unsafe { self.ptr.offset(isize::try_from(offset).unwrap()) };
            let base = get_definition(name);
            let what = unsafe { base.offset(isize::try_from(addend).unwrap()) };
            match reloc {
                Reloc::Abs4 => {
                    #[cfg_attr(feature = "cargo-clippy", allow(clippy::cast_ptr_alignment))]
                    unsafe {
                        write_unaligned(at as *mut u32, u32::try_from(what as usize).unwrap())
                    };
                }
                Reloc::Abs8 => {
                    #[cfg_attr(feature = "cargo-clippy", allow(clippy::cast_ptr_alignment))]
                    unsafe {
                        write_unaligned(at as *mut u64, u64::try_from(what as usize).unwrap())
                    };
                }
                Reloc::X86PCRel4 | Reloc::X86CallPCRel4 => {
                    let pcrel = i32::try_from((what as isize) - (at as isize)).unwrap();
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
}
