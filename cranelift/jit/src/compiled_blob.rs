use cranelift_codegen::binemit::Reloc;
use cranelift_module::ModuleExtName;
use cranelift_module::ModuleReloc;
use std::convert::TryFrom;

/// Reads a 32bit instruction at `iptr`, and writes it again after
/// being altered by `modifier`
unsafe fn modify_inst32(iptr: *mut u32, modifier: impl FnOnce(u32) -> u32) {
    let inst = iptr.read_unaligned();
    let new_inst = modifier(inst);
    iptr.write_unaligned(new_inst);
}

#[derive(Clone)]
pub(crate) struct CompiledBlob {
    pub(crate) ptr: *mut u8,
    pub(crate) size: usize,
    pub(crate) relocs: Vec<ModuleReloc>,
}

impl CompiledBlob {
    pub(crate) fn perform_relocations(
        &self,
        get_address: impl Fn(&ModuleExtName) -> *const u8,
        get_got_entry: impl Fn(&ModuleExtName) -> *const u8,
        get_plt_entry: impl Fn(&ModuleExtName) -> *const u8,
    ) {
        use std::ptr::write_unaligned;

        for &ModuleReloc {
            kind,
            offset,
            ref name,
            addend,
        } in &self.relocs
        {
            debug_assert!((offset as usize) < self.size);
            let at = unsafe { self.ptr.offset(isize::try_from(offset).unwrap()) };
            match kind {
                Reloc::Abs4 => {
                    let base = get_address(name);
                    let what = unsafe { base.offset(isize::try_from(addend).unwrap()) };
                    #[cfg_attr(feature = "cargo-clippy", allow(clippy::cast_ptr_alignment))]
                    unsafe {
                        write_unaligned(at as *mut u32, u32::try_from(what as usize).unwrap())
                    };
                }
                Reloc::Abs8 => {
                    let base = get_address(name);
                    let what = unsafe { base.offset(isize::try_from(addend).unwrap()) };
                    #[cfg_attr(feature = "cargo-clippy", allow(clippy::cast_ptr_alignment))]
                    unsafe {
                        write_unaligned(at as *mut u64, u64::try_from(what as usize).unwrap())
                    };
                }
                Reloc::X86PCRel4 | Reloc::X86CallPCRel4 => {
                    let base = get_address(name);
                    let what = unsafe { base.offset(isize::try_from(addend).unwrap()) };
                    let pcrel = i32::try_from((what as isize) - (at as isize)).unwrap();
                    #[cfg_attr(feature = "cargo-clippy", allow(clippy::cast_ptr_alignment))]
                    unsafe {
                        write_unaligned(at as *mut i32, pcrel)
                    };
                }
                Reloc::X86GOTPCRel4 => {
                    let base = get_got_entry(name);
                    let what = unsafe { base.offset(isize::try_from(addend).unwrap()) };
                    let pcrel = i32::try_from((what as isize) - (at as isize)).unwrap();
                    #[cfg_attr(feature = "cargo-clippy", allow(clippy::cast_ptr_alignment))]
                    unsafe {
                        write_unaligned(at as *mut i32, pcrel)
                    };
                }
                Reloc::X86CallPLTRel4 => {
                    let base = get_plt_entry(name);
                    let what = unsafe { base.offset(isize::try_from(addend).unwrap()) };
                    let pcrel = i32::try_from((what as isize) - (at as isize)).unwrap();
                    #[cfg_attr(feature = "cargo-clippy", allow(clippy::cast_ptr_alignment))]
                    unsafe {
                        write_unaligned(at as *mut i32, pcrel)
                    };
                }
                Reloc::S390xPCRel32Dbl | Reloc::S390xPLTRel32Dbl => {
                    let base = get_address(name);
                    let what = unsafe { base.offset(isize::try_from(addend).unwrap()) };
                    let pcrel = i32::try_from(((what as isize) - (at as isize)) >> 1).unwrap();
                    #[cfg_attr(feature = "cargo-clippy", allow(clippy::cast_ptr_alignment))]
                    unsafe {
                        write_unaligned(at as *mut i32, pcrel)
                    };
                }
                Reloc::Arm64Call => {
                    let base = get_address(name);
                    // The instruction is 32 bits long.
                    let iptr = at as *mut u32;
                    // The offset encoded in the `bl` instruction is the
                    // number of bytes divided by 4.
                    let diff = ((base as isize) - (at as isize)) >> 2;
                    // Sign propagating right shift disposes of the
                    // included bits, so the result is expected to be
                    // either all sign bits or 0, depending on if the original
                    // value was negative or positive.
                    assert!((diff >> 26 == -1) || (diff >> 26 == 0));
                    // The lower 26 bits of the `bl` instruction form the
                    // immediate offset argument.
                    let chop = 32 - 26;
                    let imm26 = (diff as u32) << chop >> chop;
                    unsafe { modify_inst32(iptr, |inst| inst | imm26) };
                }
                Reloc::RiscvCall => {
                    // A R_RISCV_CALL relocation expects auipc+jalr instruction pair.
                    // It is the equivalent of two relocations:
                    // 1. R_RISCV_PCREL_HI20 on the `auipc`
                    // 2. R_RISCV_PCREL_LO12_I on the `jalr`

                    let base = get_address(name);
                    let what = unsafe { base.offset(isize::try_from(addend).unwrap()) };
                    let pcrel = i32::try_from((what as isize) - (at as isize)).unwrap() as u32;

                    // See https://github.com/riscv-non-isa/riscv-elf-psabi-doc/blob/master/riscv-elf.adoc#pc-relative-symbol-addresses
                    // for a better explanation of the following code.
                    //
                    // Unlike the regular symbol relocations, here both "sub-relocations" point to the same address.
                    //
                    // `pcrel` is a signed value (+/- 2GiB range), when splitting it into two parts, we need to
                    // ensure that `hi20` is close enough to `pcrel` to be able to add `lo12` to it and still
                    // get a valid address.
                    //
                    // `lo12` is also a signed offset (+/- 2KiB range) relative to the `hi20` value.
                    //
                    // `hi20` should also be shifted right to be the "true" value. But we also need it
                    // left shifted for the `lo12` calculation and it also matches the instruction encoding.
                    let hi20 = pcrel.wrapping_add(0x800) & 0xFFFFF000;
                    let lo12 = pcrel.wrapping_sub(hi20) & 0xFFF;

                    unsafe {
                        // Do a R_RISCV_PCREL_HI20 on the `auipc`
                        let auipc_addr = at as *mut u32;
                        modify_inst32(auipc_addr, |auipc| (auipc & 0xFFF) | hi20);

                        // Do a R_RISCV_PCREL_LO12_I on the `jalr`
                        let jalr_addr = at.offset(4) as *mut u32;
                        modify_inst32(jalr_addr, |jalr| (jalr & 0xFFFFF) | (lo12 << 20));
                    }
                }
                _ => unimplemented!(),
            }
        }
    }
}
