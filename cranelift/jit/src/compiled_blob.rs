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
    #[cfg(feature = "wasmtime-unwinder")]
    pub(crate) exception_data: Option<Vec<u8>>,
}

unsafe impl Send for CompiledBlob {}

impl CompiledBlob {
    pub(crate) fn perform_relocations(
        &self,
        get_address: impl Fn(&ModuleRelocTarget) -> *const u8,
    ) {
        use std::ptr::write_unaligned;

        for (
            i,
            &ModuleReloc {
                kind,
                offset,
                ref name,
                addend,
            },
        ) in self.relocs.iter().enumerate()
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
                    panic!("GOT relocation shouldn't be generated when !is_pic");
                }
                Reloc::X86CallPLTRel4 => {
                    panic!("PLT relocation shouldn't be generated when !is_pic");
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
                    panic!("GOT relocation shouldn't be generated when !is_pic");
                }
                Reloc::Aarch64Ld64GotLo12Nc => {
                    panic!("GOT relocation shouldn't be generated when !is_pic");
                }
                Reloc::Aarch64AdrPrelPgHi21 => {
                    let base = get_address(name);
                    let what = unsafe { base.offset(isize::try_from(addend).unwrap()) };
                    let get_page = |x| x & (!0xfff);
                    let pcrel = i32::try_from(get_page(what as isize) - get_page(at as isize))
                        .unwrap()
                        .cast_unsigned();
                    let iptr = at as *mut u32;
                    let hi21 = pcrel >> 12;
                    let lo = (hi21 & 0x3) << 29;
                    let hi = (hi21 & 0x1ffffc) << 3;
                    unsafe { modify_inst32(iptr, |inst| inst | lo | hi) };
                }
                Reloc::Aarch64AddAbsLo12Nc => {
                    let base = get_address(name);
                    let what = unsafe { base.offset(isize::try_from(addend).unwrap()) };
                    let iptr = at as *mut u32;
                    let imm12 = (what.addr() as u32 & 0xfff) << 10;
                    unsafe { modify_inst32(iptr, |inst| inst | imm12) };
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
                Reloc::PulleyPcRel => {
                    let base = get_address(name);
                    let what = unsafe { base.offset(isize::try_from(addend).unwrap()) };
                    let pcrel = i32::try_from((what as isize) - (at as isize)).unwrap();
                    let at = at as *mut i32;
                    unsafe {
                        at.write_unaligned(at.read_unaligned().wrapping_add(pcrel));
                    }
                }

                // See <https://github.com/riscv-non-isa/riscv-elf-psabi-doc/blob/master/riscv-elf.adoc#pc-relative-symbol-addresses>
                // for why `0x800` is added here.
                Reloc::RiscvPCRelHi20 => {
                    let base = get_address(name);
                    let what = unsafe { base.offset(isize::try_from(addend).unwrap()) };
                    let pcrel = i32::try_from((what as isize) - (at as isize) + 0x800)
                        .unwrap()
                        .cast_unsigned();
                    let at = at as *mut u32;
                    unsafe {
                        modify_inst32(at, |i| i | (pcrel & 0xfffff000));
                    }
                }

                // The target of this relocation is the `auipc` preceding this
                // instruction which should be `RiscvPCRelHi20`, and the actual
                // target that we're relocating against is the target of that
                // relocation. Assume for now that the previous relocation is
                // the target of this relocation, and then use that.
                Reloc::RiscvPCRelLo12I => {
                    let prev_reloc = &self.relocs[i - 1];
                    assert_eq!(prev_reloc.kind, Reloc::RiscvPCRelHi20);
                    let lo_target = get_address(name);
                    let hi_address =
                        unsafe { self.ptr.offset(isize::try_from(prev_reloc.offset).unwrap()) };
                    assert_eq!(lo_target, hi_address);
                    let hi_target = get_address(&prev_reloc.name);
                    let pcrel = i32::try_from((hi_target as isize) - (hi_address as isize))
                        .unwrap()
                        .cast_unsigned();
                    let at = at as *mut u32;
                    unsafe {
                        modify_inst32(at, |i| i | ((pcrel & 0xfff) << 20));
                    }
                }

                other => unimplemented!("unimplemented reloc {other:?}"),
            }
        }
    }
}
