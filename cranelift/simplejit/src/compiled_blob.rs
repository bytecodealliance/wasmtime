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
            let at = unsafe { self.ptr.offset(offset as isize) };
            let base = get_definition(name);
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
}
