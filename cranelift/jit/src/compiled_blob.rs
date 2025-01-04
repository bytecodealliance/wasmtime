use cranelift_codegen::binemit::Reloc;
use cranelift_module::ModuleReloc;
use cranelift_module::ModuleRelocTarget;

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

unsafe impl Send for CompiledBlob {}

impl CompiledBlob {
    pub(crate) fn code(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.size) }
    }

    pub(crate) fn perform_relocations(
        &self,
        get_address: impl Fn(&ModuleRelocTarget) -> *const u8,
        get_got_entry: impl Fn(&ModuleRelocTarget) -> *const u8,
        get_plt_entry: impl Fn(&ModuleRelocTarget) -> *const u8,
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
                    unsafe {
                        write_unaligned(at as *mut u32, u32::try_from(what as usize).unwrap())
                    };
                }
                Reloc::Abs8 => {
                    let base = get_address(name);
                    let what = unsafe { base.offset(isize::try_from(addend).unwrap()) };
                    unsafe {
                        write_unaligned(at as *mut u64, u64::try_from(what as usize).unwrap())
                    };
                }
                Reloc::X86PCRel4 | Reloc::X86CallPCRel4 => {
                    let base = get_address(name);
                    let what = unsafe { base.offset(isize::try_from(addend).unwrap()) };
                    let pcrel = i32::try_from((what as isize) - (at as isize)).unwrap();
                    unsafe { write_unaligned(at as *mut i32, pcrel) };
                }
                Reloc::X86GOTPCRel4 => {
                    let base = get_got_entry(name);
                    let what = unsafe { base.offset(isize::try_from(addend).unwrap()) };
                    let pcrel = i32::try_from((what as isize) - (at as isize)).unwrap();
                    unsafe { write_unaligned(at as *mut i32, pcrel) };
                }
                Reloc::X86CallPLTRel4 => {
                    let base = get_plt_entry(name);
                    let what = unsafe { base.offset(isize::try_from(addend).unwrap()) };
                    let pcrel = i32::try_from((what as isize) - (at as isize)).unwrap();
                    unsafe { write_unaligned(at as *mut i32, pcrel) };
                }
                Reloc::S390xPCRel32Dbl | Reloc::S390xPLTRel32Dbl => {
                    let base = get_address(name);
                    let what = unsafe { base.offset(isize::try_from(addend).unwrap()) };
                    let pcrel = i32::try_from(((what as isize) - (at as isize)) >> 1).unwrap();
                    unsafe { write_unaligned(at as *mut i32, pcrel) };
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
                Reloc::Aarch64AdrGotPage21 => {
                    // Set the immediate value of an ADRP to bits [32:12] of X; check that –2^32 <= X < 2^32
                    assert_eq!(addend, 0, "addend affects the address looked up in get_got_entry, which is currently only called with a symbol");
                    let what = get_got_entry(name);
                    let what_page = (what as usize) & !0xfff;
                    let at_page = (at as usize) & !0xfff;
                    let pcrel = (what_page as isize).checked_sub(at_page as isize).unwrap();
                    assert!(
                        (-1 << 32) <= (pcrel as i64) && (pcrel as i64) < (1 << 32),
                        "can't reach GOT page with ±4GB `adrp` instruction"
                    );
                    let val = pcrel >> 12;

                    let immlo = ((val as u32) & 0b11) << 29;
                    let immhi = (((val as u32) >> 2) & &0x7ffff) << 5;
                    let mask = !((0x7ffff << 5) | (0b11 << 29));
                    unsafe { modify_inst32(at as *mut u32, |adrp| (adrp & mask) | immlo | immhi) };
                }
                Reloc::Aarch64Ld64GotLo12Nc => {
                    // Set the LD/ST immediate field to bits 11:3 of X. No overflow check; check that X&7 = 0
                    assert_eq!(addend, 0);
                    let base = get_got_entry(name);
                    let what = base as u32;
                    assert_eq!(what & 0b111, 0);
                    let val = what >> 3;
                    let imm9 = (val & 0x1ff) << 10;
                    let mask = !(0x1ff << 10);
                    unsafe { modify_inst32(at as *mut u32, |ldr| (ldr & mask) | imm9) };
                }
                Reloc::RiscvCallPlt => {
                    // A R_RISCV_CALL_PLT relocation expects auipc+jalr instruction pair.
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
