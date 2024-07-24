//! Encodes EVEX instructions. These instructions are those added by the AVX-512 extensions. The
//! EVEX encoding requires a 4-byte prefix:
//!
//! Byte 0:  0x62
//!         ┌───┬───┬───┬───┬───┬───┬───┬───┐
//! Byte 1: │ R │ X │ B │ R'│ 0 │ 0 │ m │ m │
//!         ├───┼───┼───┼───┼───┼───┼───┼───┤
//! Byte 2: │ W │ v │ v │ v │ v │ 1 │ p │ p │
//!         ├───┼───┼───┼───┼───┼───┼───┼───┤
//! Byte 3: │ z │ L'│ L │ b │ V'│ a │ a │ a │
//!         └───┴───┴───┴───┴───┴───┴───┴───┘
//!
//! The prefix is then followed by the opcode byte, the ModR/M byte, and other optional suffixes
//! (e.g. SIB byte, displacements, immediates) based on the instruction (see section 2.6, Intel
//! Software Development Manual, volume 2A).

use super::rex::{self, LegacyPrefixes, OpcodeMap};
use crate::isa::x64::args::{Amode, Avx512TupleType};
use crate::isa::x64::inst::Inst;
use crate::MachBuffer;
use core::ops::RangeInclusive;

/// Constructs an EVEX-encoded instruction using a builder pattern. This approach makes it visually
/// easier to transform something the manual's syntax, `EVEX.256.66.0F38.W1 1F /r` to code:
/// `EvexInstruction::new().length(...).prefix(...).map(...).w(true).opcode(0x1F).reg(...).rm(...)`.
pub struct EvexInstruction {
    bits: u32,
    opcode: u8,
    reg: Register,
    rm: RegisterOrAmode,
    tuple_type: Option<Avx512TupleType>,
    imm: Option<u8>,
}

/// Because some of the bit flags in the EVEX prefix are reversed and users of `EvexInstruction` may
/// choose to skip setting fields, here we set some sane defaults. Note that:
/// - the first byte is always `0x62` but you will notice it at the end of the default `bits` value
///   implemented--remember the little-endian order
/// - some bits are always set to certain values: bits 10-11 to 0, bit 18 to 1
/// - the other bits set correspond to reversed bits: R, X, B, R' (byte 1), vvvv (byte 2), V' (byte
///   3).
///
/// See the `default_emission` test for what these defaults are equivalent to (e.g. using RAX,
/// unsetting the W bit, etc.)
impl Default for EvexInstruction {
    fn default() -> Self {
        Self {
            bits: 0x08_7C_F0_62,
            opcode: 0,
            reg: Register::default(),
            rm: RegisterOrAmode::Register(Register::default()),
            tuple_type: None,
            imm: None,
        }
    }
}

#[allow(non_upper_case_globals)] // This makes it easier to match the bit range names to the manual's names.
impl EvexInstruction {
    /// Construct a default EVEX instruction.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the length of the instruction . Note that there are sets of instructions (i.e. rounding,
    /// memory broadcast) that modify the same underlying bits--at some point (TODO) we can add a
    /// way to set those context bits and verify that both are not used (e.g. rounding AND length).
    /// For now, this method is very convenient.
    #[inline(always)]
    pub fn length(mut self, length: EvexVectorLength) -> Self {
        self.write(Self::LL, EvexContext::Other { length }.bits() as u32);
        self
    }

    /// Set the legacy prefix byte of the instruction: None | 66 | F0 | F2 | F3. EVEX instructions
    /// pack these into the prefix, not as separate bytes.
    #[inline(always)]
    pub fn prefix(mut self, prefix: LegacyPrefixes) -> Self {
        self.write(Self::pp, prefix.bits() as u32);
        self
    }

    /// Set the opcode map byte of the instruction: None | 0F | 0F38 | 0F3A. EVEX instructions pack
    /// these into the prefix, not as separate bytes.
    #[inline(always)]
    pub fn map(mut self, map: OpcodeMap) -> Self {
        self.write(Self::mm, map.bits() as u32);
        self
    }

    /// Set the W bit, typically used to indicate an instruction using 64 bits of an operand (e.g.
    /// 64 bit lanes). EVEX packs this bit in the EVEX prefix; previous encodings used the REX
    /// prefix.
    #[inline(always)]
    pub fn w(mut self, w: bool) -> Self {
        self.write(Self::W, w as u32);
        self
    }

    /// Set the instruction opcode byte.
    #[inline(always)]
    pub fn opcode(mut self, opcode: u8) -> Self {
        self.opcode = opcode;
        self
    }

    /// Set the "tuple type" which is used for 8-bit scaling when a memory
    /// operand is used.
    #[inline(always)]
    pub fn tuple_type(mut self, tt: Avx512TupleType) -> Self {
        self.tuple_type = Some(tt);
        self
    }

    /// Set the register to use for the `reg` bits; many instructions use this as the write operand.
    /// Setting this affects both the ModRM byte (`reg` section) and the EVEX prefix (the extension
    /// bits for register encodings > 8).
    #[inline(always)]
    pub fn reg(mut self, reg: impl Into<Register>) -> Self {
        self.reg = reg.into();
        let r = !(self.reg.0 >> 3) & 1;
        let r_ = !(self.reg.0 >> 4) & 1;
        self.write(Self::R, r as u32);
        self.write(Self::R_, r_ as u32);
        self
    }

    /// Set the mask to use. See section 2.6 in the Intel Software Developer's Manual, volume 2A for
    /// more details.
    #[allow(dead_code)]
    #[inline(always)]
    pub fn mask(mut self, mask: EvexMasking) -> Self {
        self.write(Self::aaa, mask.aaa_bits() as u32);
        self.write(Self::z, mask.z_bit() as u32);
        self
    }

    /// Set the `vvvvv` register; some instructions allow using this as a second, non-destructive
    /// source register in 3-operand instructions (e.g. 2 read, 1 write).
    #[allow(dead_code)]
    #[inline(always)]
    pub fn vvvvv(mut self, reg: impl Into<Register>) -> Self {
        let reg = reg.into();
        self.write(Self::vvvv, !(reg.0 as u32) & 0b1111);
        self.write(Self::V_, !(reg.0 as u32 >> 4) & 0b1);
        self
    }

    /// Set the register to use for the `rm` bits; many instructions use this
    /// as the "read from register/memory" operand. Setting this affects both
    /// the ModRM byte (`rm` section) and the EVEX prefix (the extension bits
    /// for register encodings > 8).
    #[inline(always)]
    pub fn rm(mut self, reg: impl Into<RegisterOrAmode>) -> Self {
        // NB: See Table 2-31. 32-Register Support in 64-bit Mode Using EVEX
        // with Embedded REX Bits
        self.rm = reg.into();
        let x = match &self.rm {
            RegisterOrAmode::Register(r) => r.0 >> 4,
            RegisterOrAmode::Amode(Amode::ImmRegRegShift { index, .. }) => {
                index.to_real_reg().unwrap().hw_enc() >> 3
            }

            // These two modes technically don't use the X bit, so leave it at
            // 0.
            RegisterOrAmode::Amode(Amode::ImmReg { .. }) => 0,
            RegisterOrAmode::Amode(Amode::RipRelative { .. }) => 0,
        };
        // The X bit is stored in an inverted format, so invert it here.
        self.write(Self::X, u32::from(!x & 1));

        let b = match &self.rm {
            RegisterOrAmode::Register(r) => r.0 >> 3,
            RegisterOrAmode::Amode(Amode::ImmReg { base, .. }) => {
                base.to_real_reg().unwrap().hw_enc() >> 3
            }
            RegisterOrAmode::Amode(Amode::ImmRegRegShift { base, .. }) => {
                base.to_real_reg().unwrap().hw_enc() >> 3
            }
            // The 4th bit of %rip is 0
            RegisterOrAmode::Amode(Amode::RipRelative { .. }) => 0,
        };
        // The B bit is stored in an inverted format, so invert it here.
        self.write(Self::B, u32::from(!b & 1));
        self
    }

    /// Set the imm byte.
    #[inline(always)]
    pub fn imm(mut self, imm: u8) -> Self {
        self.imm = Some(imm);
        self
    }

    /// Emit the EVEX-encoded instruction to the code sink:
    ///
    /// - the 4-byte EVEX prefix;
    /// - the opcode byte;
    /// - the ModR/M byte
    /// - SIB bytes, if necessary
    /// - an optional immediate, if necessary (not currently implemented)
    pub fn encode(&self, sink: &mut MachBuffer<Inst>) {
        if let RegisterOrAmode::Amode(amode) = &self.rm {
            if let Some(trap_code) = amode.get_flags().trap_code() {
                sink.add_trap(trap_code);
            }
        }
        sink.put4(self.bits);
        sink.put1(self.opcode);

        match &self.rm {
            RegisterOrAmode::Register(reg) => {
                let rm: u8 = (*reg).into();
                sink.put1(rex::encode_modrm(3, self.reg.0 & 7, rm & 7));
            }
            RegisterOrAmode::Amode(amode) => {
                let scaling = self.scaling_for_8bit_disp();

                let bytes_at_end = if self.imm.is_some() { 1 } else { 0 };
                rex::emit_modrm_sib_disp(sink, self.reg.0 & 7, amode, bytes_at_end, Some(scaling));
            }
        }
        if let Some(imm) = self.imm {
            sink.put1(imm);
        }
    }

    // In order to simplify the encoding of the various bit ranges in the prefix, we specify those
    // ranges according to the table below (extracted from the Intel Software Development Manual,
    // volume 2A). Remember that, because we pack the 4-byte prefix into a little-endian `u32`, this
    // chart should be read from right-to-left, top-to-bottom. Note also that we start ranges at bit
    // 8, leaving bits 0-7 for the mandatory `0x62`.
    //         ┌───┬───┬───┬───┬───┬───┬───┬───┐
    // Byte 1: │ R │ X │ B │ R'│ 0 │ 0 │ m │ m │
    //         ├───┼───┼───┼───┼───┼───┼───┼───┤
    // Byte 2: │ W │ v │ v │ v │ v │ 1 │ p │ p │
    //         ├───┼───┼───┼───┼───┼───┼───┼───┤
    // Byte 3: │ z │ L'│ L │ b │ V'│ a │ a │ a │
    //         └───┴───┴───┴───┴───┴───┴───┴───┘

    // Byte 1:
    const mm: RangeInclusive<u8> = 8..=9;
    const R_: RangeInclusive<u8> = 12..=12;
    const B: RangeInclusive<u8> = 13..=13;
    const X: RangeInclusive<u8> = 14..=14;
    const R: RangeInclusive<u8> = 15..=15;

    // Byte 2:
    const pp: RangeInclusive<u8> = 16..=17;
    const vvvv: RangeInclusive<u8> = 19..=22;
    const W: RangeInclusive<u8> = 23..=23;

    // Byte 3:
    const aaa: RangeInclusive<u8> = 24..=26;
    const V_: RangeInclusive<u8> = 27..=27;
    const b: RangeInclusive<u8> = 28..=28;
    const LL: RangeInclusive<u8> = 29..=30;
    const z: RangeInclusive<u8> = 31..=31;

    // A convenience method for writing the `value` bits to the given range in `self.bits`.
    #[inline]
    fn write(&mut self, range: RangeInclusive<u8>, value: u32) {
        assert!(ExactSizeIterator::len(&range) > 0);
        let size = range.end() - range.start() + 1; // Calculate the number of bits in the range.
        let mask: u32 = (1 << size) - 1; // Generate a bit mask.
        debug_assert!(
            value <= mask,
            "The written value should have fewer than {} bits.",
            size
        );
        let mask_complement = !(mask << *range.start()); // Create the bitwise complement for the clear mask.
        self.bits &= mask_complement; // Clear the bits in `range`; otherwise the OR below may allow previously-set bits to slip through.
        let value = value << *range.start(); // Place the value in the correct location (assumes `value <= mask`).
        self.bits |= value; // Modify the bits in `range`.
    }

    /// A convenience method for reading given range of bits in `self.bits`
    /// shifted to the LSB of the returned value..
    #[inline]
    fn read(&self, range: RangeInclusive<u8>) -> u32 {
        (self.bits >> range.start()) & ((1 << range.len()) - 1)
    }

    fn scaling_for_8bit_disp(&self) -> i8 {
        use Avx512TupleType::*;

        let vector_size_scaling = || match self.read(Self::LL) {
            0b00 => 16,
            0b01 => 32,
            0b10 => 64,
            _ => unreachable!(),
        };

        match self.tuple_type {
            Some(Full) => {
                if self.read(Self::b) == 1 {
                    if self.read(Self::W) == 0 {
                        4
                    } else {
                        8
                    }
                } else {
                    vector_size_scaling()
                }
            }
            Some(FullMem) => vector_size_scaling(),
            Some(Mem128) => 16,
            None => panic!("tuple type was not set"),
        }
    }
}

/// Describe the register index to use. This wrapper is a type-safe way to pass
/// around the registers defined in `inst/regs.rs`.
#[derive(Debug, Copy, Clone, Default)]
pub struct Register(u8);
impl From<u8> for Register {
    fn from(reg: u8) -> Self {
        debug_assert!(reg < 16);
        Self(reg)
    }
}
impl Into<u8> for Register {
    fn into(self) -> u8 {
        self.0
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub enum RegisterOrAmode {
    Register(Register),
    Amode(Amode),
}

impl From<u8> for RegisterOrAmode {
    fn from(reg: u8) -> Self {
        RegisterOrAmode::Register(reg.into())
    }
}

impl From<Amode> for RegisterOrAmode {
    fn from(amode: Amode) -> Self {
        RegisterOrAmode::Amode(amode)
    }
}

/// Defines the EVEX context for the `L'`, `L`, and `b` bits (bits 6:4 of EVEX P2 byte). Table 2-36 in
/// section 2.6.10 (Intel Software Development Manual, volume 2A) describes how these bits can be
/// used together for certain classes of instructions; i.e., special care should be taken to ensure
/// that instructions use an applicable correct `EvexContext`. Table 2-39 contains cases where
/// opcodes can result in an #UD.
#[allow(dead_code, missing_docs)] // Rounding and broadcast modes are not yet used.
pub enum EvexContext {
    RoundingRegToRegFP {
        rc: EvexRoundingControl,
    },
    NoRoundingFP {
        sae: bool,
        length: EvexVectorLength,
    },
    MemoryOp {
        broadcast: bool,
        length: EvexVectorLength,
    },
    Other {
        length: EvexVectorLength,
    },
}

impl Default for EvexContext {
    fn default() -> Self {
        Self::Other {
            length: EvexVectorLength::default(),
        }
    }
}

impl EvexContext {
    /// Encode the `L'`, `L`, and `b` bits (bits 6:4 of EVEX P2 byte) for merging with the P2 byte.
    pub fn bits(&self) -> u8 {
        match self {
            Self::RoundingRegToRegFP { rc } => 0b001 | rc.bits() << 1,
            Self::NoRoundingFP { sae, length } => (*sae as u8) | length.bits() << 1,
            Self::MemoryOp { broadcast, length } => (*broadcast as u8) | length.bits() << 1,
            Self::Other { length } => length.bits() << 1,
        }
    }
}

/// The EVEX format allows choosing a vector length in the `L'` and `L` bits; see `EvexContext`.
#[allow(dead_code, missing_docs)] // Wider-length vectors are not yet used.
pub enum EvexVectorLength {
    V128,
    V256,
    V512,
}

impl EvexVectorLength {
    /// Encode the `L'` and `L` bits for merging with the P2 byte.
    fn bits(&self) -> u8 {
        match self {
            Self::V128 => 0b00,
            Self::V256 => 0b01,
            Self::V512 => 0b10,
            // 0b11 is reserved (#UD).
        }
    }
}

impl Default for EvexVectorLength {
    fn default() -> Self {
        Self::V128
    }
}

/// The EVEX format allows defining rounding control in the `L'` and `L` bits; see `EvexContext`.
#[allow(dead_code, missing_docs)] // Rounding controls are not yet used.
pub enum EvexRoundingControl {
    RNE,
    RD,
    RU,
    RZ,
}

impl EvexRoundingControl {
    /// Encode the `L'` and `L` bits for merging with the P2 byte.
    fn bits(&self) -> u8 {
        match self {
            Self::RNE => 0b00,
            Self::RD => 0b01,
            Self::RU => 0b10,
            Self::RZ => 0b11,
        }
    }
}

/// Defines the EVEX masking behavior; masking support is described in section 2.6.4 of the Intel
/// Software Development Manual, volume 2A.
#[allow(dead_code, missing_docs)] // Masking is not yet used.
pub enum EvexMasking {
    None,
    Merging { k: u8 },
    Zeroing { k: u8 },
}

impl Default for EvexMasking {
    fn default() -> Self {
        EvexMasking::None
    }
}

impl EvexMasking {
    /// Encode the `z` bit for merging with the P2 byte.
    pub fn z_bit(&self) -> u8 {
        match self {
            Self::None | Self::Merging { .. } => 0,
            Self::Zeroing { .. } => 1,
        }
    }

    /// Encode the `aaa` bits for merging with the P2 byte.
    pub fn aaa_bits(&self) -> u8 {
        match self {
            Self::None => 0b000,
            Self::Merging { k } | Self::Zeroing { k } => {
                debug_assert!(*k <= 7);
                *k
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::MemFlags;
    use crate::isa::x64::args::Gpr;
    use crate::isa::x64::inst::regs;
    use std::vec::Vec;

    // As a sanity test, we verify that the output of `xed-asmparse-main 'vpabsq xmm0{k0},
    // xmm1'` matches this EVEX encoding machinery.
    #[test]
    fn vpabsq() {
        let mut tmp = MachBuffer::<Inst>::new();
        let tests: &[(crate::Reg, RegisterOrAmode, Vec<u8>)] = &[
            // vpabsq %xmm1, %xmm0
            (
                regs::xmm0(),
                regs::xmm1().to_real_reg().unwrap().hw_enc().into(),
                vec![0x62, 0xf2, 0xfd, 0x08, 0x1f, 0xc1],
            ),
            // vpabsq %xmm8, %xmm10
            (
                regs::xmm10(),
                regs::xmm8().to_real_reg().unwrap().hw_enc().into(),
                vec![0x62, 0x52, 0xfd, 0x08, 0x1f, 0xd0],
            ),
            // vpabsq %xmm15, %xmm3
            (
                regs::xmm3(),
                regs::xmm15().to_real_reg().unwrap().hw_enc().into(),
                vec![0x62, 0xd2, 0xfd, 0x08, 0x1f, 0xdf],
            ),
            // vpabsq (%rsi), %xmm12
            (
                regs::xmm12(),
                Amode::ImmReg {
                    simm32: 0,
                    base: regs::rsi(),
                    flags: MemFlags::trusted(),
                }
                .into(),
                vec![0x62, 0x72, 0xfd, 0x08, 0x1f, 0x26],
            ),
            // vpabsq 8(%r15), %xmm14
            (
                regs::xmm14(),
                Amode::ImmReg {
                    simm32: 8,
                    base: regs::r15(),
                    flags: MemFlags::trusted(),
                }
                .into(),
                vec![0x62, 0x52, 0xfd, 0x08, 0x1f, 0xb7, 0x08, 0x00, 0x00, 0x00],
            ),
            // vpabsq 16(%r15), %xmm14
            (
                regs::xmm14(),
                Amode::ImmReg {
                    simm32: 16,
                    base: regs::r15(),
                    flags: MemFlags::trusted(),
                }
                .into(),
                vec![0x62, 0x52, 0xfd, 0x08, 0x1f, 0x77, 0x01],
            ),
            // vpabsq 17(%rax), %xmm3
            (
                regs::xmm3(),
                Amode::ImmReg {
                    simm32: 17,
                    base: regs::rax(),
                    flags: MemFlags::trusted(),
                }
                .into(),
                vec![0x62, 0xf2, 0xfd, 0x08, 0x1f, 0x98, 0x11, 0x00, 0x00, 0x00],
            ),
            // vpabsq (%rbx, %rsi, 8), %xmm9
            (
                regs::xmm9(),
                Amode::ImmRegRegShift {
                    simm32: 0,
                    base: Gpr::unwrap_new(regs::rbx()),
                    index: Gpr::unwrap_new(regs::rsi()),
                    shift: 3,
                    flags: MemFlags::trusted(),
                }
                .into(),
                vec![0x62, 0x72, 0xfd, 0x08, 0x1f, 0x0c, 0xf3],
            ),
            // vpabsq 1(%r11, %rdi, 4), %xmm13
            (
                regs::xmm13(),
                Amode::ImmRegRegShift {
                    simm32: 1,
                    base: Gpr::unwrap_new(regs::r11()),
                    index: Gpr::unwrap_new(regs::rdi()),
                    shift: 2,
                    flags: MemFlags::trusted(),
                }
                .into(),
                vec![
                    0x62, 0x52, 0xfd, 0x08, 0x1f, 0xac, 0xbb, 0x01, 0x00, 0x00, 0x00,
                ],
            ),
            // vpabsq 128(%rsp, %r10, 2), %xmm5
            (
                regs::xmm5(),
                Amode::ImmRegRegShift {
                    simm32: 128,
                    base: Gpr::unwrap_new(regs::rsp()),
                    index: Gpr::unwrap_new(regs::r10()),
                    shift: 1,
                    flags: MemFlags::trusted(),
                }
                .into(),
                vec![0x62, 0xb2, 0xfd, 0x08, 0x1f, 0x6c, 0x54, 0x08],
            ),
            // vpabsq 112(%rbp, %r13, 1), %xmm6
            (
                regs::xmm6(),
                Amode::ImmRegRegShift {
                    simm32: 112,
                    base: Gpr::unwrap_new(regs::rbp()),
                    index: Gpr::unwrap_new(regs::r13()),
                    shift: 0,
                    flags: MemFlags::trusted(),
                }
                .into(),
                vec![0x62, 0xb2, 0xfd, 0x08, 0x1f, 0x74, 0x2d, 0x07],
            ),
            // vpabsq (%rbp, %r13, 1), %xmm7
            (
                regs::xmm7(),
                Amode::ImmRegRegShift {
                    simm32: 0,
                    base: Gpr::unwrap_new(regs::rbp()),
                    index: Gpr::unwrap_new(regs::r13()),
                    shift: 0,
                    flags: MemFlags::trusted(),
                }
                .into(),
                vec![0x62, 0xb2, 0xfd, 0x08, 0x1f, 0x7c, 0x2d, 0x00],
            ),
            // vpabsq 2032(%r12), %xmm8
            (
                regs::xmm8(),
                Amode::ImmReg {
                    simm32: 2032,
                    base: regs::r12(),
                    flags: MemFlags::trusted(),
                }
                .into(),
                vec![0x62, 0x52, 0xfd, 0x08, 0x1f, 0x44, 0x24, 0x7f],
            ),
            // vpabsq 2048(%r13), %xmm9
            (
                regs::xmm9(),
                Amode::ImmReg {
                    simm32: 2048,
                    base: regs::r13(),
                    flags: MemFlags::trusted(),
                }
                .into(),
                vec![0x62, 0x52, 0xfd, 0x08, 0x1f, 0x8d, 0x00, 0x08, 0x00, 0x00],
            ),
            // vpabsq -16(%r14), %xmm10
            (
                regs::xmm10(),
                Amode::ImmReg {
                    simm32: -16,
                    base: regs::r14(),
                    flags: MemFlags::trusted(),
                }
                .into(),
                vec![0x62, 0x52, 0xfd, 0x08, 0x1f, 0x56, 0xff],
            ),
            // vpabsq -5(%r15), %xmm11
            (
                regs::xmm11(),
                Amode::ImmReg {
                    simm32: -5,
                    base: regs::r15(),
                    flags: MemFlags::trusted(),
                }
                .into(),
                vec![0x62, 0x52, 0xfd, 0x08, 0x1f, 0x9f, 0xfb, 0xff, 0xff, 0xff],
            ),
            // vpabsq -2048(%rdx), %xmm12
            (
                regs::xmm12(),
                Amode::ImmReg {
                    simm32: -2048,
                    base: regs::rdx(),
                    flags: MemFlags::trusted(),
                }
                .into(),
                vec![0x62, 0x72, 0xfd, 0x08, 0x1f, 0x62, 0x80],
            ),
            // vpabsq -2064(%rsi), %xmm13
            (
                regs::xmm13(),
                Amode::ImmReg {
                    simm32: -2064,
                    base: regs::rsi(),
                    flags: MemFlags::trusted(),
                }
                .into(),
                vec![0x62, 0x72, 0xfd, 0x08, 0x1f, 0xae, 0xf0, 0xf7, 0xff, 0xff],
            ),
            // a: vpabsq a(%rip), %xmm14
            (
                regs::xmm14(),
                Amode::RipRelative {
                    target: tmp.get_label(),
                }
                .into(),
                vec![0x62, 0x72, 0xfd, 0x08, 0x1f, 0x35, 0xf6, 0xff, 0xff, 0xff],
            ),
        ];

        for (dst, src, encoding) in tests {
            let mut sink = MachBuffer::new();
            let label = sink.get_label();
            sink.bind_label(label, &mut Default::default());
            EvexInstruction::new()
                .prefix(LegacyPrefixes::_66)
                .map(OpcodeMap::_0F38)
                .w(true)
                .opcode(0x1F)
                .reg(dst.to_real_reg().unwrap().hw_enc())
                .rm(src.clone())
                .length(EvexVectorLength::V128)
                .tuple_type(Avx512TupleType::Full)
                .encode(&mut sink);
            let bytes0 = sink
                .finish(&Default::default(), &mut Default::default())
                .data;
            assert_eq!(
                bytes0.as_slice(),
                encoding.as_slice(),
                "dst={dst:?} src={src:?}"
            );
        }
    }

    /// Verify that the defaults are equivalent to an instruction with a `0x00` opcode using the
    /// "0" register (i.e. `rax`), with sane defaults for the various configurable parameters. This
    /// test is more interesting than it may appear because some of the parameters have flipped-bit
    /// representations (e.g. `vvvvv`) so emitting 0s as a default will not work.
    #[test]
    fn default_emission() {
        let mut sink = MachBuffer::new();
        EvexInstruction::new().encode(&mut sink);
        let bytes0 = sink
            .finish(&Default::default(), &mut Default::default())
            .data;

        let mut sink = MachBuffer::new();
        EvexInstruction::new()
            .length(EvexVectorLength::V128)
            .prefix(LegacyPrefixes::None)
            .map(OpcodeMap::None)
            .w(false)
            .opcode(0x00)
            .reg(regs::rax().to_real_reg().unwrap().hw_enc())
            .rm(regs::rax().to_real_reg().unwrap().hw_enc())
            .mask(EvexMasking::None)
            .encode(&mut sink);
        let bytes1 = sink
            .finish(&Default::default(), &mut Default::default())
            .data;

        assert_eq!(bytes0, bytes1);
    }
}
