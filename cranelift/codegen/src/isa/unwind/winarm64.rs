//! Windows Arm64 ABI unwind information.

use alloc::vec::Vec;
#[cfg(feature = "enable-serde")]
use serde_derive::{Deserialize, Serialize};

use crate::binemit::CodeOffset;
use crate::isa::unwind::UnwindInst;
use crate::result::CodegenResult;

use super::Writer;

/// The supported unwind codes for the Arm64 Windows ABI.
///
/// See: <https://learn.microsoft.com/en-us/cpp/build/arm64-exception-handling>
/// Only what is needed to describe the prologues generated by the Cranelift AArch64 ISA are represented here.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub(crate) enum UnwindCode {
    /// Save int register, or register pair.
    SaveReg {
        reg: u8,
        stack_offset: u16,
        is_pair: bool,
    },
    /// Save floating point register, or register pair.
    SaveFReg {
        reg: u8,
        stack_offset: u16,
        is_pair: bool,
    },
    /// Save frame-pointer register (X29) and LR register pair.
    SaveFpLrPair {
        stack_offset: u16,
    },
    // Small (<512b) stack allocation.
    AllocS {
        size: u16,
    },
    // Medium (<32Kb) stack allocation.
    AllocM {
        size: u16,
    },
    // Large (<256Mb) stack allocation.
    AllocL {
        size: u32,
    },
    /// PAC sign the LR register.
    PacSignLr,
    /// Set the frame-pointer register to the stack-pointer register.
    SetFp,
    /// Set the frame-pointer register to the stack-pointer register with an
    /// offset.
    AddFp {
        offset: u16,
    },
}

/// Represents Windows Arm64 unwind information.
///
/// For information about Windows Arm64 unwind info, see:
/// <https://learn.microsoft.com/en-us/cpp/build/arm64-exception-handling>
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct UnwindInfo {
    pub(crate) unwind_codes: Vec<UnwindCode>,
}

impl UnwindInfo {
    /// Calculate the number of words needed to encode the unwind codes.
    pub fn code_words(&self) -> u8 {
        let mut bytes = 0u16;
        for code in self.unwind_codes.iter() {
            let next_bytes = match code {
                UnwindCode::SaveFpLrPair { .. }
                | UnwindCode::AllocS { .. }
                | UnwindCode::PacSignLr
                | UnwindCode::SetFp => 1,
                UnwindCode::SaveReg { .. }
                | UnwindCode::SaveFReg { .. }
                | UnwindCode::AllocM { .. }
                | UnwindCode::AddFp { .. } => 2,
                UnwindCode::AllocL { .. } => 4,
            };
            bytes = bytes.checked_add(next_bytes).unwrap();
        }

        bytes.div_ceil(4).try_into().unwrap()
    }

    /// Emits the unwind information into the given mutable byte slice.
    ///
    /// This function will panic if the slice is not at least `emit_size` in length.
    pub fn emit(&self, buf: &mut [u8]) {
        fn encode_stack_offset<const BITS: u8>(stack_offset: u16) -> u16 {
            let encoded = (stack_offset / 8) - 1;
            assert!(encoded < (1 << BITS), "Stack offset too large");
            encoded
        }

        // NOTE: Unwind codes are written in big-endian!

        let mut writer = Writer::new(buf);
        for code in self.unwind_codes.iter().rev() {
            match code {
                &UnwindCode::SaveReg {
                    reg,
                    stack_offset,
                    is_pair,
                } => {
                    assert!(reg >= 19, "Can't save registers before X19");
                    let reg = u16::from(reg - 19);
                    let encoding = if is_pair {
                        let mut encoding = 0b11001100_00000000u16;
                        encoding |= reg << 6;
                        encoding |= encode_stack_offset::<6>(stack_offset);
                        encoding
                    } else {
                        let mut encoding = 0b11010100_00000000u16;
                        encoding |= reg << 5;
                        encoding |= encode_stack_offset::<5>(stack_offset);
                        encoding
                    };
                    writer.write_u16_be(encoding);
                }
                &UnwindCode::SaveFReg {
                    reg,
                    stack_offset,
                    is_pair,
                } => {
                    assert!(reg >= 8, "Can't save registers before D8");
                    let reg = u16::from(reg - 8);
                    let encoding = if is_pair {
                        let mut encoding = 0b11011010_00000000u16;
                        encoding |= reg << 6;
                        encoding |= encode_stack_offset::<6>(stack_offset);
                        encoding
                    } else {
                        let mut encoding = 0b11011110_00000000u16;
                        encoding |= reg << 5;
                        encoding |= encode_stack_offset::<5>(stack_offset);
                        encoding
                    };
                    writer.write_u16_be(encoding);
                }
                &UnwindCode::SaveFpLrPair { stack_offset } => {
                    if stack_offset == 0 {
                        writer.write_u8(0b01000000);
                    } else {
                        let encoding = 0b10000000u8
                            | u8::try_from(encode_stack_offset::<6>(stack_offset)).unwrap();
                        writer.write_u8(encoding);
                    }
                }
                &UnwindCode::AllocS { size } => {
                    // Size is measured in double 64-bit words.
                    let encoding = size / 16;
                    assert!(encoding < (1 << 5), "Stack alloc size too large");
                    // Tag is 0b000, so we don't need to encode that.
                    writer.write_u8(encoding.try_into().unwrap());
                }
                &UnwindCode::AllocM { size } => {
                    // Size is measured in double 64-bit words.
                    let mut encoding = size / 16;
                    assert!(encoding < (1 << 11), "Stack alloc size too large");
                    encoding |= 0b11000 << 11;
                    writer.write_u16_be(encoding);
                }
                &UnwindCode::AllocL { size } => {
                    // Size is measured in double 64-bit words.
                    let mut encoding = size / 16;
                    assert!(encoding < (1 << 24), "Stack alloc size too large");
                    encoding |= 0b11100000 << 24;
                    writer.write_u32_be(encoding);
                }
                UnwindCode::PacSignLr => {
                    writer.write_u8(0b11111100);
                }
                UnwindCode::SetFp => {
                    writer.write_u8(0b11100001);
                }
                &UnwindCode::AddFp { mut offset } => {
                    offset /= 8;
                    assert!(offset & !0xFF == 0, "Offset too large");
                    let encoding = (0b11100010 << 8) | offset;
                    writer.write_u16_be(encoding);
                }
            }
        }
    }
}

pub(crate) fn create_unwind_info_from_insts(
    insts: &[(CodeOffset, UnwindInst)],
) -> CodegenResult<UnwindInfo> {
    let mut unwind_codes = vec![];
    let mut last_stackalloc = None;
    let mut last_clobber_offset = None;
    for &(_, ref inst) in insts {
        match inst {
            &UnwindInst::PushFrameRegs { .. } => {
                unwind_codes.push(UnwindCode::SaveFpLrPair { stack_offset: 16 });
                unwind_codes.push(UnwindCode::SetFp);
            }
            &UnwindInst::DefineNewFrame {
                offset_downward_to_clobbers,
                ..
            } => {
                assert!(last_clobber_offset.is_none(), "More than one frame defined");
                last_clobber_offset = Some(offset_downward_to_clobbers);

                // If we've seen a stackalloc, then we were adjusting the stack
                // to make space for additional arguments, so encode that now.
                if let &Some(last_stackalloc) = &last_stackalloc {
                    assert!(last_stackalloc < (1u32 << 8) * 8);
                    unwind_codes.push(UnwindCode::AddFp {
                        offset: u16::try_from(last_stackalloc).unwrap(),
                    });
                    unwind_codes.push(UnwindCode::SaveFpLrPair { stack_offset: 0 });
                    unwind_codes.push(UnwindCode::SetFp);
                }
            }
            &UnwindInst::StackAlloc { size } => {
                last_stackalloc = Some(size);
                assert!(size % 16 == 0, "Size must be a multiple of 16");
                const SMALL_STACK_ALLOC_MAX: u32 = (1 << 5) * 16 - 1;
                const MEDIUM_STACK_ALLOC_MIN: u32 = SMALL_STACK_ALLOC_MAX + 1;
                const MEDIUM_STACK_ALLOC_MAX: u32 = (1 << 11) * 16 - 1;
                const LARGE_STACK_ALLOC_MIN: u32 = MEDIUM_STACK_ALLOC_MAX + 1;
                const LARGE_STACK_ALLOC_MAX: u32 = (1 << 24) * 16 - 1;
                match size {
                    0..=SMALL_STACK_ALLOC_MAX => unwind_codes.push(UnwindCode::AllocS {
                        size: size.try_into().unwrap(),
                    }),
                    MEDIUM_STACK_ALLOC_MIN..=MEDIUM_STACK_ALLOC_MAX => {
                        unwind_codes.push(UnwindCode::AllocM {
                            size: size.try_into().unwrap(),
                        })
                    }
                    LARGE_STACK_ALLOC_MIN..=LARGE_STACK_ALLOC_MAX => {
                        unwind_codes.push(UnwindCode::AllocL { size: size })
                    }
                    _ => panic!("Stack allocation size too large"),
                }
            }
            &UnwindInst::SaveReg {
                clobber_offset,
                reg,
            } => {
                // We're given the clobber offset, but we need to encode how far
                // the stack was adjusted, so calculate that based on the last
                // clobber offset we saw.
                let last_clobber_offset = last_clobber_offset.as_mut().expect("No frame defined");
                if *last_clobber_offset > clobber_offset {
                    let stack_offset = *last_clobber_offset - clobber_offset;
                    *last_clobber_offset = clobber_offset;

                    assert!(stack_offset % 8 == 0, "Offset must be a multiple of 8");
                    match reg.class() {
                        regalloc2::RegClass::Int => {
                            let reg = reg.hw_enc();
                            if reg < 19 {
                                panic!("Can't save registers before X19");
                            }
                            unwind_codes.push(UnwindCode::SaveReg {
                                reg,
                                stack_offset: stack_offset.try_into().unwrap(),
                                is_pair: false,
                            });
                        }
                        regalloc2::RegClass::Float => {
                            let reg = reg.hw_enc();
                            if reg < 8 {
                                panic!("Can't save registers before D8");
                            }
                            unwind_codes.push(UnwindCode::SaveFReg {
                                reg,
                                stack_offset: stack_offset.try_into().unwrap(),
                                is_pair: false,
                            });
                        }
                        regalloc2::RegClass::Vector => unreachable!(),
                    }
                } else {
                    // If we see a clobber offset within the last offset amount,
                    // then we're actually saving a pair of registers.
                    let last_unwind_code = unwind_codes.last_mut().unwrap();
                    match last_unwind_code {
                        UnwindCode::SaveReg { is_pair, .. } => {
                            assert_eq!(reg.class(), regalloc2::RegClass::Int);
                            assert!(!*is_pair);
                            *is_pair = true;
                        }
                        UnwindCode::SaveFReg { is_pair, .. } => {
                            assert_eq!(reg.class(), regalloc2::RegClass::Float);
                            assert!(!*is_pair);
                            *is_pair = true;
                        }
                        _ => unreachable!("Previous code should have been a register save"),
                    }
                }
            }
            &UnwindInst::Aarch64SetPointerAuth { return_addresses } => {
                assert!(
                    return_addresses,
                    "Windows doesn't support explicitly disabling return address signing"
                );
                unwind_codes.push(UnwindCode::PacSignLr);
            }
        }
    }

    Ok(UnwindInfo { unwind_codes })
}