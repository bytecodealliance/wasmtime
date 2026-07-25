use std::ptr;

use cranelift_codegen::binemit::{Addend, Reloc};
use cranelift_module::{ModuleError, ModuleReloc, ModuleRelocTarget, ModuleResult};

use crate::JITMemoryProvider;
use crate::memory::JITMemoryKind;

/// Size reserved per veneer. This is exactly the size of the largest veneer:
/// 16 bytes on AArch64 (`ldr` + `br` + 8-byte target address). The x86_64
/// veneer is 14 bytes (`jmp qword ptr [rip]` + 8-byte target address).
///
/// Veneers are placed directly after the compiled code, followed by the GOT
/// entries: `[code][veneers][GOT entries]`.
const VENEER_SIZE: usize = 16;

/// Size of a GOT entry: the absolute 8-byte address of a symbol.
const GOT_ENTRY_SIZE: usize = 8;

/// Reads a 32bit instruction at `iptr`, and writes it again after
/// being altered by `modifier`
unsafe fn modify_inst32(iptr: *mut u32, modifier: impl FnOnce(u32) -> u32) {
    let inst = iptr.read_unaligned();
    let new_inst = modifier(inst);
    iptr.write_unaligned(new_inst);
}

#[derive(Clone)]
pub(crate) struct CompiledBlob {
    ptr: *mut u8,
    size: usize,
    relocs: Vec<ModuleReloc>,
    veneer_count: usize,
    got_count: usize,
    #[cfg(feature = "wasmtime-unwinder")]
    wasmtime_exception_data: Option<Vec<u8>>,
}

unsafe impl Send for CompiledBlob {}

impl CompiledBlob {
    pub(crate) fn new(
        memory: &mut dyn JITMemoryProvider,
        data: &[u8],
        align: u64,
        relocs: Vec<ModuleReloc>,
        #[cfg(feature = "wasmtime-unwinder")] wasmtime_exception_data: Option<Vec<u8>>,
        kind: JITMemoryKind,
    ) -> ModuleResult<Self> {
        // Reserve a worst-case veneer slot for every branch relocation and a
        // GOT entry for every GOT-relative load in case its target is out of
        // range.
        let mut veneer_count = 0;
        let mut got_count = 0;
        for reloc in &relocs {
            match reloc.kind {
                Reloc::Arm64Call | Reloc::X86CallPCRel4 | Reloc::X86CallPLTRel4 => {
                    veneer_count += 1
                }
                Reloc::X86GOTPCRel4 => got_count += 1,
                _ => {}
            }
        }

        let mut alloc_size = data.len() + veneer_count * VENEER_SIZE;
        if got_count > 0 {
            alloc_size = alloc_size.next_multiple_of(GOT_ENTRY_SIZE) + got_count * GOT_ENTRY_SIZE;
        }

        let ptr = memory
            .allocate(alloc_size, align, kind)
            .map_err(|e| ModuleError::Allocation { err: e })?;

        unsafe {
            ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
        }

        Ok(CompiledBlob {
            ptr,
            size: data.len(),
            relocs,
            veneer_count,
            got_count,
            #[cfg(feature = "wasmtime-unwinder")]
            wasmtime_exception_data,
        })
    }

    pub(crate) fn new_zeroed(
        memory: &mut dyn JITMemoryProvider,
        size: usize,
        align: u64,
        relocs: Vec<ModuleReloc>,
        #[cfg(feature = "wasmtime-unwinder")] wasmtime_exception_data: Option<Vec<u8>>,
        kind: JITMemoryKind,
    ) -> ModuleResult<Self> {
        let ptr = memory
            .allocate(size, align, kind)
            .map_err(|e| ModuleError::Allocation { err: e })?;

        unsafe { ptr::write_bytes(ptr, 0, size) };

        Ok(CompiledBlob {
            ptr,
            size,
            relocs,
            veneer_count: 0,
            got_count: 0,
            #[cfg(feature = "wasmtime-unwinder")]
            wasmtime_exception_data,
        })
    }

    pub(crate) fn ptr(&self) -> *const u8 {
        self.ptr
    }

    pub(crate) fn size(&self) -> usize {
        self.size
    }

    #[cfg(feature = "wasmtime-unwinder")]
    pub(crate) fn wasmtime_exception_data(&self) -> Option<&[u8]> {
        self.wasmtime_exception_data.as_deref()
    }

    /// Offset of the GOT entries within the allocation: after the code and
    /// the veneers, aligned to the entry size.
    fn got_offset(&self) -> usize {
        (self.size + self.veneer_count * VENEER_SIZE).next_multiple_of(GOT_ENTRY_SIZE)
    }

    pub(crate) fn perform_relocations(
        &self,
        get_address: impl Fn(&ModuleRelocTarget) -> *const u8,
    ) {
        use std::ptr::write_unaligned;

        let mut next_veneer_idx = 0;
        let mut next_got_idx = 0;
        let relocation_target_addr = |name: &ModuleRelocTarget, addend: Addend| {
            let addend = isize::try_from(addend).unwrap();
            get_address(name)
                .expose_provenance()
                .checked_add_signed(addend)
                .unwrap()
        };

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
                    let what = relocation_target_addr(name, addend);
                    unsafe { write_unaligned(at as *mut u32, u32::try_from(what).unwrap()) };
                }
                Reloc::Abs8 => {
                    let what = relocation_target_addr(name, addend);
                    unsafe { write_unaligned(at as *mut u64, u64::try_from(what).unwrap()) };
                }
                Reloc::X86PCRel4 => {
                    let what = relocation_target_addr(name, addend);
                    let pcrel = i32::try_from((what as isize) - (at as isize)).unwrap();
                    unsafe { write_unaligned(at as *mut i32, pcrel) };
                }
                Reloc::X86CallPCRel4 | Reloc::X86CallPLTRel4 => {
                    // These relocations are applied to the 32-bit displacement of a `call`
                    // or `jmp` instruction, which is relative to the end of the
                    // instruction, i.e. 4 bytes past `at`. The addend (always -4 as
                    // emitted by the x64 backend) accounts for this, so the instruction
                    // transfers control to `what + 4`. The PLT form additionally permits
                    // resolution through a stub, which the veneer below provides.
                    let what = relocation_target_addr(name, addend);
                    if let Ok(pcrel) = i32::try_from((what as isize) - (at as isize)) {
                        unsafe { write_unaligned(at as *mut i32, pcrel) };
                    } else {
                        // The target is out of range for the 32-bit displacement, so
                        // redirect the call through a veneer at the end of the function,
                        // just like for `Arm64Call` below: `jmp qword ptr [rip]` followed
                        // by the absolute target address.
                        let veneer_idx = next_veneer_idx;
                        next_veneer_idx += 1;
                        assert!(veneer_idx < self.veneer_count);
                        let veneer =
                            unsafe { self.ptr.byte_add(self.size + veneer_idx * VENEER_SIZE) };

                        let target = u64::try_from(what.checked_add(4).unwrap()).unwrap();
                        unsafe {
                            ptr::copy_nonoverlapping([0xff, 0x25, 0, 0, 0, 0].as_ptr(), veneer, 6);
                            write_unaligned(veneer.byte_add(6).cast::<u64>(), target);
                        }

                        // Point the original instruction at the veneer instead; the
                        // displacement is again relative to the end of the 4-byte field.
                        let pcrel = i32::try_from((veneer as isize) - (at as isize) - 4).unwrap();
                        unsafe { write_unaligned(at as *mut i32, pcrel) };
                    }
                }
                Reloc::X86GOTPCRel4 => {
                    // This relocation is applied to the 32-bit displacement of a
                    // `mov reg, qword ptr [rip + disp]` instruction that loads the
                    // address of a symbol from its GOT entry. The addend (always -4 as
                    // emitted by the x64 backend) accounts for the displacement being
                    // relative to the end of the instruction, and any symbol offset is
                    // added by a separately emitted instruction, so the GOT entry holds
                    // the address of the symbol itself: `what + 4`.
                    let what = relocation_target_addr(name, addend);
                    let symbol_addr = u64::try_from(what.checked_add(4).unwrap()).unwrap();

                    // If the symbol is within displacement range, relax the load to a
                    // `lea` computing its address directly, the same way linkers relax
                    // `R_X86_64_REX_GOTPCRELX`. The instruction is expected to be a REX
                    // prefix, the `mov` opcode 0x8b, and a ModRM byte with mod=0b00 and
                    // r/m=0b101 (RIP-relative); only rewrite it if it matches.
                    let insn = (offset >= 3)
                        .then(|| unsafe { at.cast_const().sub(3).cast::<[u8; 3]>().read() });
                    let is_rip_relative_mov = insn.is_some_and(
                        |insn| matches!(insn, [0x48..=0x4f, 0x8b, modrm] if modrm & 0xc7 == 0x05),
                    );
                    let direct_pcrel = i32::try_from((what as isize) - (at as isize));
                    match (is_rip_relative_mov, direct_pcrel) {
                        (true, Ok(pcrel)) => unsafe {
                            at.sub(2).write(0x8d); // mov -> lea
                            write_unaligned(at as *mut i32, pcrel);
                        },
                        _ => {
                            // Resolve through a GOT entry at the end of the allocation,
                            // which is always in range of the load.
                            let got_idx = next_got_idx;
                            next_got_idx += 1;
                            assert!(got_idx < self.got_count);
                            let entry = unsafe {
                                self.ptr
                                    .byte_add(self.got_offset() + got_idx * GOT_ENTRY_SIZE)
                            };
                            unsafe { write_unaligned(entry.cast::<u64>(), symbol_addr) };

                            let pcrel =
                                i32::try_from((entry as isize) - (at as isize) - 4).unwrap();
                            unsafe { write_unaligned(at as *mut i32, pcrel) };
                        }
                    }
                }
                Reloc::S390xPCRel32Dbl | Reloc::S390xPLTRel32Dbl => {
                    let what = relocation_target_addr(name, addend);
                    let pcrel = i32::try_from(((what as isize) - (at as isize)) >> 1).unwrap();
                    unsafe { write_unaligned(at as *mut i32, pcrel) };
                }
                Reloc::Arm64Call => {
                    let what = relocation_target_addr(name, addend);
                    // The instruction is 32 bits long.
                    let iptr = at as *mut u32;

                    // The offset encoded in the `bl` instruction is the
                    // number of bytes divided by 4.
                    let diff = ((what as isize) - (at as isize)) >> 2;
                    // Sign propagating right shift disposes of the
                    // included bits, so the result is expected to be
                    // either all sign bits or 0 when in-range, depending
                    // on if the original value was negative or positive.
                    if (diff >> 25 == -1) || (diff >> 25 == 0) {
                        // The lower 26 bits of the `bl` instruction form the
                        // immediate offset argument.
                        let chop = 32 - 26;
                        let imm26 = (diff as u32) << chop >> chop;
                        unsafe { modify_inst32(iptr, |inst| inst | imm26) };
                    } else {
                        // If the target is out of range for a direct call, insert a veneer at the
                        // end of the function.
                        let veneer_idx = next_veneer_idx;
                        next_veneer_idx += 1;
                        assert!(veneer_idx < self.veneer_count);
                        let veneer =
                            unsafe { self.ptr.byte_add(self.size + veneer_idx * VENEER_SIZE) };

                        // Write the veneer
                        // x16 is reserved as scratch register to be used by veneers and PLT entries
                        unsafe {
                            write_unaligned(
                                veneer.cast::<u32>(),
                                0x58000050, // ldr x16, 0x8
                            );
                            write_unaligned(
                                veneer.byte_add(4).cast::<u32>(),
                                0xd61f0200, // br x16
                            );
                            write_unaligned(veneer.byte_add(8).cast::<u64>(), what as u64);
                        };

                        // Set the veneer as target of the call
                        let diff = ((veneer as isize) - (at as isize)) >> 2;
                        assert!((diff >> 25 == -1) || (diff >> 25 == 0));
                        let chop = 32 - 26;
                        let imm26 = (diff as u32) << chop >> chop;
                        unsafe { modify_inst32(iptr, |inst| inst | imm26) };
                    }
                }
                Reloc::Aarch64AdrGotPage21 => {
                    panic!("GOT relocation shouldn't be generated when !is_pic");
                }
                Reloc::Aarch64Ld64GotLo12Nc => {
                    panic!("GOT relocation shouldn't be generated when !is_pic");
                }
                Reloc::Aarch64AdrPrelPgHi21 => {
                    let what = relocation_target_addr(name, addend);
                    let get_page = |x| x & (!0xfff);
                    // NOTE: This should technically be i33 given that this relocation type allows
                    // a range from -4GB to +4GB, not -2GB to +2GB. But this doesn't really matter
                    // as the target is unlikely to be more than 2GB from the adrp instruction. We
                    // need to be careful to not cast to an unsigned int until after doing >> 12 to
                    // compute the upper 21bits of the pcrel address however as otherwise the top
                    // bit of the 33bit pcrel address would be forced 0 through zero extension
                    // instead of being sign extended as it should be.
                    let pcrel =
                        i32::try_from(get_page(what as isize) - get_page(at as isize)).unwrap();
                    let iptr = at as *mut u32;
                    let hi21 = (pcrel >> 12).cast_unsigned();
                    let lo = (hi21 & 0x3) << 29;
                    let hi = (hi21 & 0x1ffffc) << 3;
                    unsafe { modify_inst32(iptr, |inst| inst | lo | hi) };
                }
                Reloc::Aarch64AddAbsLo12Nc => {
                    let what = relocation_target_addr(name, addend);
                    let iptr = at as *mut u32;
                    let imm12 = (what as u32 & 0xfff) << 10;
                    unsafe { modify_inst32(iptr, |inst| inst | imm12) };
                }
                Reloc::RiscvCallPlt => {
                    // A R_RISCV_CALL_PLT relocation expects auipc+jalr instruction pair.
                    // It is the equivalent of two relocations:
                    // 1. R_RISCV_PCREL_HI20 on the `auipc`
                    // 2. R_RISCV_PCREL_LO12_I on the `jalr`

                    let what = relocation_target_addr(name, addend);
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
                    let what = relocation_target_addr(name, addend);
                    let pcrel = i32::try_from((what as isize) - (at as isize)).unwrap();
                    let at = at as *mut i32;
                    unsafe {
                        at.write_unaligned(at.read_unaligned().wrapping_add(pcrel));
                    }
                }

                // See <https://github.com/riscv-non-isa/riscv-elf-psabi-doc/blob/master/riscv-elf.adoc#pc-relative-symbol-addresses>
                // for why `0x800` is added here.
                Reloc::RiscvPCRelHi20 => {
                    let what = relocation_target_addr(name, addend);
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
