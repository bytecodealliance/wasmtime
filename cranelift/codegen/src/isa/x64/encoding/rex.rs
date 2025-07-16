//! Encodes instructions in the standard x86 encoding mode. This is called
//! IA-32E mode in the Intel manuals but corresponds to the addition of the
//! REX-prefix format (hence the name of this module) that allowed encoding
//! instructions in both compatibility mode (32-bit instructions running on a
//! 64-bit OS) and in 64-bit mode (using the full 64-bit address space).
//!
//! For all of the routines that take both a memory-or-reg operand (sometimes
//! called "E" in the Intel documentation, see the Intel Developer's manual,
//! vol. 2, section A.2) and a reg-only operand ("G" in Intel-ese), the order is
//! always G first, then E. The term "enc" in the following means "hardware
//! register encoding number".

use super::ByteSink;
use crate::isa::x64::inst::args::Amode;
use crate::isa::x64::inst::{Inst, LabelUse, regs};
use crate::machinst::{MachBuffer, Reg, RegClass};

pub(crate) fn low8_will_sign_extend_to_32(x: u32) -> bool {
    let xs = x as i32;
    xs == ((xs << 24) >> 24)
}

/// Encode the ModR/M byte.
#[inline(always)]
pub fn encode_modrm(m0d: u8, enc_reg_g: u8, rm_e: u8) -> u8 {
    debug_assert!(m0d < 4);
    debug_assert!(enc_reg_g < 8);
    debug_assert!(rm_e < 8);
    ((m0d & 3) << 6) | ((enc_reg_g & 7) << 3) | (rm_e & 7)
}

#[inline(always)]
pub(crate) fn encode_sib(shift: u8, enc_index: u8, enc_base: u8) -> u8 {
    debug_assert!(shift < 4);
    debug_assert!(enc_index < 8);
    debug_assert!(enc_base < 8);
    ((shift & 3) << 6) | ((enc_index & 7) << 3) | (enc_base & 7)
}

/// Get the encoding number of a GPR.
#[inline(always)]
pub(crate) fn int_reg_enc(reg: impl Into<Reg>) -> u8 {
    let reg = reg.into();
    debug_assert!(reg.is_real(), "reg = {reg:?}");
    debug_assert_eq!(reg.class(), RegClass::Int);
    reg.to_real_reg().unwrap().hw_enc()
}

/// Allows using the same opcode byte in different "opcode maps" to allow for more instruction
/// encodings. See appendix A in the Intel Software Developer's Manual, volume 2A, for more details.
#[derive(PartialEq)]
pub enum OpcodeMap {
    None,
    _0F,
    _0F38,
    _0F3A,
}

impl OpcodeMap {
    /// Normally the opcode map is specified as bytes in the instruction, but some x64 encoding
    /// formats pack this information as bits in a prefix (e.g. VEX / EVEX).
    pub(crate) fn bits(&self) -> u8 {
        match self {
            OpcodeMap::None => 0b00,
            OpcodeMap::_0F => 0b01,
            OpcodeMap::_0F38 => 0b10,
            OpcodeMap::_0F3A => 0b11,
        }
    }
}

impl Default for OpcodeMap {
    fn default() -> Self {
        Self::None
    }
}

/// We may need to include one or more legacy prefix bytes before the REX prefix.  This enum
/// covers only the small set of possibilities that we actually need.
#[derive(PartialEq)]
pub enum LegacyPrefixes {
    /// No prefix bytes.
    None,
    /// Operand Size Override -- here, denoting "16-bit operation".
    _66,
    /// The Lock prefix.
    _F0,
    /// Operand size override and Lock.
    _66F0,
    /// REPNE, but no specific meaning here -- is just an opcode extension.
    _F2,
    /// REP/REPE, but no specific meaning here -- is just an opcode extension.
    _F3,
    /// Operand size override and same effect as F3.
    _66F3,
}

impl LegacyPrefixes {
    /// Emit the legacy prefix as bits (e.g. for EVEX instructions).
    #[inline(always)]
    pub(crate) fn bits(&self) -> u8 {
        match self {
            Self::None => 0b00,
            Self::_66 => 0b01,
            Self::_F3 => 0b10,
            Self::_F2 => 0b11,
            _ => panic!(
                "VEX and EVEX bits can only be extracted from single prefixes: None, 66, F3, F2"
            ),
        }
    }
}

impl Default for LegacyPrefixes {
    fn default() -> Self {
        Self::None
    }
}

pub(crate) fn emit_modrm_sib_disp(
    sink: &mut MachBuffer<Inst>,
    enc_g: u8,
    mem_e: &Amode,
    bytes_at_end: u8,
    evex_scaling: Option<i8>,
) {
    match *mem_e {
        Amode::ImmReg { simm32, base, .. } => {
            let enc_e = int_reg_enc(base);
            let mut imm = Imm::new(simm32, evex_scaling);

            // Most base registers allow for a single ModRM byte plus an
            // optional immediate. If rsp is the base register, however, then a
            // SIB byte must be used.
            let enc_e_low3 = enc_e & 7;
            if enc_e_low3 != regs::ENC_RSP {
                // If the base register is rbp and there's no offset then force
                // a 1-byte zero offset since otherwise the encoding would be
                // invalid.
                if enc_e_low3 == regs::ENC_RBP {
                    imm.force_immediate();
                }
                sink.put1(encode_modrm(imm.m0d(), enc_g & 7, enc_e & 7));
                imm.emit(sink);
            } else {
                // Displacement from RSP is encoded with a SIB byte where
                // the index and base are both encoded as RSP's encoding of
                // 0b100. This special encoding means that the index register
                // isn't used and the base is 0b100 with or without a
                // REX-encoded 4th bit (e.g. rsp or r12)
                sink.put1(encode_modrm(imm.m0d(), enc_g & 7, 0b100));
                sink.put1(0b00_100_100);
                imm.emit(sink);
            }
        }

        Amode::ImmRegRegShift {
            simm32,
            base: reg_base,
            index: reg_index,
            shift,
            ..
        } => {
            let enc_base = int_reg_enc(*reg_base);
            let enc_index = int_reg_enc(*reg_index);

            // Encoding of ModRM/SIB bytes don't allow the index register to
            // ever be rsp. Note, though, that the encoding of r12, whose three
            // lower bits match the encoding of rsp, is explicitly allowed with
            // REX bytes so only rsp is disallowed.
            assert!(enc_index != regs::ENC_RSP);

            // If the offset is zero then there is no immediate. Note, though,
            // that if the base register's lower three bits are `101` then an
            // offset must be present. This is a special case in the encoding of
            // the SIB byte and requires an explicit displacement with rbp/r13.
            let mut imm = Imm::new(simm32, evex_scaling);
            if enc_base & 7 == regs::ENC_RBP {
                imm.force_immediate();
            }

            // With the above determined encode the ModRM byte, then the SIB
            // byte, then any immediate as necessary.
            sink.put1(encode_modrm(imm.m0d(), enc_g & 7, 0b100));
            sink.put1(encode_sib(shift, enc_index & 7, enc_base & 7));
            imm.emit(sink);
        }

        Amode::RipRelative { ref target } => {
            // RIP-relative is mod=00, rm=101.
            sink.put1(encode_modrm(0b00, enc_g & 7, 0b101));

            let offset = sink.cur_offset();
            sink.use_label_at_offset(offset, *target, LabelUse::JmpRel32);
            // N.B.: some instructions (XmmRmRImm format for example)
            // have bytes *after* the RIP-relative offset. The
            // addressed location is relative to the end of the
            // instruction, but the relocation is nominally relative
            // to the end of the u32 field. So, to compensate for
            // this, we emit a negative extra offset in the u32 field
            // initially, and the relocation will add to it.
            sink.put4(-(i32::from(bytes_at_end)) as u32);
        }
    }
}

#[derive(Copy, Clone)]
enum Imm {
    None,
    Imm8(i8),
    Imm32(i32),
}

impl Imm {
    /// Classifies the 32-bit immediate `val` as how this can be encoded
    /// with ModRM/SIB bytes.
    ///
    /// For `evex_scaling` according to Section 2.7.5 of Intel's manual:
    ///
    /// > EVEX-encoded instructions always use a compressed displacement scheme
    /// > by multiplying disp8 in conjunction with a scaling factor N that is
    /// > determined based on the vector length, the value of EVEX.b bit
    /// > (embedded broadcast) and the input element size of the instruction
    ///
    /// The `evex_scaling` factor provided here is `Some(N)` for EVEX
    /// instructions.  This is taken into account where the `Imm` value
    /// contained is the raw byte offset.
    fn new(val: i32, evex_scaling: Option<i8>) -> Imm {
        if val == 0 {
            return Imm::None;
        }
        match evex_scaling {
            Some(scaling) => {
                if val % i32::from(scaling) == 0 {
                    let scaled = val / i32::from(scaling);
                    if low8_will_sign_extend_to_32(scaled as u32) {
                        return Imm::Imm8(scaled as i8);
                    }
                }
                Imm::Imm32(val)
            }
            None => match i8::try_from(val) {
                Ok(val) => Imm::Imm8(val),
                Err(_) => Imm::Imm32(val),
            },
        }
    }

    /// Forces `Imm::None` to become `Imm::Imm8(0)`, used for special cases
    /// where some base registers require an immediate.
    fn force_immediate(&mut self) {
        if let Imm::None = self {
            *self = Imm::Imm8(0);
        }
    }

    /// Returns the two "mod" bits present at the upper bits of the mod/rm
    /// byte.
    fn m0d(&self) -> u8 {
        match self {
            Imm::None => 0b00,
            Imm::Imm8(_) => 0b01,
            Imm::Imm32(_) => 0b10,
        }
    }

    fn emit<BS: ByteSink + ?Sized>(&self, sink: &mut BS) {
        match self {
            Imm::None => {}
            Imm::Imm8(n) => sink.put1(*n as u8),
            Imm::Imm32(n) => sink.put4(*n as u32),
        }
    }
}
