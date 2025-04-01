//! Encoding logic for VEX instructions.
//use super::evex::{Register, RegisterOrAmode};

use super::rex;
use super::XmmMem;
use crate::api::{AsReg, CodeSink, KnownOffsetTable, Registers};
use crate::mem::emit_modrm_sib_disp;
use crate::Amode;
use cranelift_assembler_x64_meta::dsl::{OpcodeMap, Vex};

/// We may need to include one or more legacy prefix bytes before the REX prefix.  This enum
/// covers only the small set of possibilities that we actually need.
#[derive(PartialEq)]
pub enum LegacyPrefix {
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

impl LegacyPrefix {
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

impl Default for LegacyPrefix {
    fn default() -> Self {
        Self::None
    }
}

pub struct VexInstruction<R: Registers> {
    pub length: VexVectorLength,
    pub prefix: LegacyPrefix,
    pub map: OpcodeMap,
    pub opcode: u8,
    pub w: bool,
    pub reg: u8,
    pub rm: XmmMem<R::ReadXmm, R::ReadGpr>,
    pub vvvv: Option<u8>,
    pub imm: Option<u8>,
}

impl<R: Registers> Default for VexInstruction<R> {
    fn default() -> Self {
        Self {
            length: VexVectorLength::default(),
            prefix: LegacyPrefix::None,
            map: OpcodeMap::None,
            opcode: 0x00,
            w: false,
            reg: 0x00,
            rm: XmmMem::default(),
            vvvv: None,
            imm: None,
        }
    }
}

impl<R: Registers> VexInstruction<R> {
    /// Construct a default VEX instruction.
    pub fn new(vex: Vex) -> Self {
        let mut vex_instruction = Self::default();
        vex_instruction.opcode = vex.opcodes.primary;
        vex_instruction
    }

    /// Set the length of the instruction.
    #[inline(always)]
    pub fn length(mut self, length: VexVectorLength) -> Self {
        self.length = length;
        self
    }

    /// Set the legacy prefix byte of the instruction: None | 66 | F2 | F3. VEX instructions
    /// pack these into the prefix, not as separate bytes.
    #[inline(always)]
    pub fn prefix(mut self, prefix: LegacyPrefix) -> Self {
        debug_assert!(
            prefix == LegacyPrefix::None
                || prefix == LegacyPrefix::_66
                || prefix == LegacyPrefix::_F2
                || prefix == LegacyPrefix::_F3
        );

        self.prefix = prefix;
        self
    }

    /// Set the opcode map byte of the instruction: None | 0F | 0F38 | 0F3A. VEX instructions pack
    /// these into the prefix, not as separate bytes.
    #[inline(always)]
    pub fn map(mut self, map: OpcodeMap) -> Self {
        self.map = map;
        self
    }

    /// Set the W bit, denoted by `.W1` or `.W0` in the instruction string.
    /// Typically used to indicate an instruction using 64 bits of an operand (e.g.
    /// 64 bit lanes). EVEX packs this bit in the EVEX prefix; previous encodings used the REX
    /// prefix.
    #[inline(always)]
    pub fn w(mut self, w: bool) -> Self {
        self.w = w;
        self
    }

    /// Set the instruction opcode byte.
    #[inline(always)]
    pub fn opcode(mut self, opcode: u8) -> Self {
        self.opcode = opcode;
        self
    }

    /// Set the register to use for the `reg` bits; many instructions use this as the write operand.
    #[inline(always)]
    pub fn reg(mut self, reg: impl AsReg) -> Self {
        self.reg = reg.enc();
        self
    }

    /// Some instructions use the ModRM.reg field as an opcode extension. This is usually denoted by
    /// a `/n` field in the manual.
    #[inline(always)]
    pub fn opcode_ext(mut self, n: u8) -> Self {
        self.reg = n;
        self
    }

    /// Set the register to use for the `rm` bits; many instructions use this
    /// as the "read from register/memory" operand. Setting this affects both
    /// the ModRM byte (`rm` section) and the VEX prefix (the extension bits
    /// for register encodings > 8).
    #[inline(always)]
    pub fn rm(mut self, reg: XmmMem<R::ReadXmm, R::ReadGpr>) -> Self {
        self.rm = reg.into();
        self
    }

    /// Set the `vvvv` register; some instructions allow using this as a second, non-destructive
    /// source register in 3-operand instructions (e.g. 2 read, 1 write).
    #[inline(always)]
    pub fn vvvv(mut self, reg: impl Into<u8>) -> Self {
        self.vvvv = Some(reg.into());
        self
    }

    /// Set the imm byte when used for a register. The reg bits are stored in `imm8[7:4]` with
    /// the lower bits unused. Overrides a previously set [Self::imm] field.
    #[inline(always)]
    pub fn imm_reg(mut self, reg: impl AsReg) -> Self {
        let reg: u8 = reg.enc();
        self.imm = Some((reg & 0xf) << 4);
        self
    }

    /// Set the imm byte.
    /// Overrides a previously set [Self::imm_reg] field.
    #[inline(always)]
    pub fn imm(mut self, imm: u8) -> Self {
        self.imm = Some(imm);
        self
    }

    /// The R bit in encoded format (inverted).
    #[inline(always)]
    fn r_bit(&self) -> u8 {
        (!(self.reg >> 3)) & 1
    }

    /// The X bit in encoded format (inverted).
    #[inline(always)]
    fn x_bit(&self) -> u8 {
        let reg = match &self.rm {
            XmmMem::Xmm(_xmm) => 0,
            XmmMem::Mem(Amode::ImmReg { .. }) => 0,
            XmmMem::Mem(Amode::ImmRegRegShift { index, .. }) => index.enc(),
            XmmMem::Mem(Amode::RipRelative { .. }) => 0,
        };

        !(reg >> 3) & 1
    }

    /// The B bit in encoded format (inverted).
    #[inline(always)]
    fn b_bit(&self) -> u8 {
        let reg = match &self.rm {
            XmmMem::Xmm(xmm) => (*xmm).enc(),
            XmmMem::Mem(Amode::ImmReg { base, .. }) => base.enc(),
            XmmMem::Mem(Amode::ImmRegRegShift { base, .. }) => base.enc(),
            XmmMem::Mem(Amode::RipRelative { .. }) => 0,
        };

        !(reg >> 3) & 1
    }

    /// Is the 2 byte prefix available for this instruction?
    /// We essentially just check if we need any of the bits that are only available
    /// in the 3 byte instruction
    #[inline(always)]
    fn use_2byte_prefix(&self) -> bool {
        // These bits are only represented on the 3 byte prefix, so their presence
        // implies the use of the 3 byte prefix
        self.b_bit() == 1 && self.x_bit() == 1 &&
        // The presence of W1 in the opcode column implies the opcode must be encoded using the
        // 3-byte form of the VEX prefix.
        self.w == false &&
        // The presence of 0F3A and 0F38 in the opcode column implies that opcode can only be
        // encoded by the three-byte form of VEX
        !(self.map == OpcodeMap::_0F3A || self.map == OpcodeMap::_0F38)
    }

    /// The last byte of the 2byte and 3byte prefixes is mostly the same, share the common
    /// encoding logic here.
    #[inline(always)]
    fn prefix_last_byte(&self) -> u8 {
        let vvvv = self.vvvv.map(|r| r.into()).unwrap_or(0x00);

        let mut byte = 0x00;
        byte |= self.prefix.bits();
        byte |= self.length.bits() << 2;
        byte |= ((!vvvv) & 0xF) << 3;
        byte
    }

    /// Encode the 2 byte prefix
    #[inline(always)]
    fn encode_2byte_prefix<CS: CodeSink + ?Sized>(&self, sink: &mut CS) {
        //  2 bytes:
        //    +-----+ +-------------------+
        //    | C5h | | R | vvvv | L | pp |
        //    +-----+ +-------------------+

        let last_byte = self.prefix_last_byte() | (self.r_bit() << 7);

        sink.put1(0xC5);
        sink.put1(last_byte);
    }

    /// Encode the 3 byte prefix
    #[inline(always)]
    fn encode_3byte_prefix<CS: CodeSink + ?Sized>(&self, sink: &mut CS) {
        //  3 bytes:
        //    +-----+ +--------------+ +-------------------+
        //    | C4h | | RXB | m-mmmm | | W | vvvv | L | pp |
        //    +-----+ +--------------+ +-------------------+
        let mut second_byte = 0x00;
        second_byte |= self.map.bits(); // m-mmmm field
        second_byte |= self.b_bit() << 5;
        second_byte |= self.x_bit() << 6;
        second_byte |= self.r_bit() << 7;

        let w_bit = self.w as u8;
        let last_byte = self.prefix_last_byte() | (w_bit << 7);

        sink.put1(0xC4);
        sink.put1(second_byte);
        sink.put1(last_byte);
    }

    /// Emit the VEX-encoded instruction to the provided buffer.
    pub fn encode(&self, sink: &mut impl CodeSink, off: &impl KnownOffsetTable) {
        // 2/3 byte prefix
        if self.use_2byte_prefix() {
            self.encode_2byte_prefix(sink);
        } else {
            self.encode_3byte_prefix(sink);
        }

        // 1 Byte Opcode
        sink.put1(self.opcode);

        match &self.rm {
            // Not all instructions use Reg as a reg, some use it as an extension
            // of the opcode.

            //RegisterOrAmode::Register(reg) => {
            XmmMem::Xmm(xmm) => {
                let rm: u8 = (*xmm).enc();
                sink.put1(rex::encode_modrm(3, self.reg & 7, rm & 7));
            }
            // For address-based modes reuse the logic from the `rex` module
            // for the modrm and trailing bytes since VEX uses the same
            // encoding.
            XmmMem::Mem(amode) => {
                let bytes_at_end = if self.imm.is_some() { 1 } else { 0 };
                emit_modrm_sib_disp(sink, off, self.reg & 7, amode, bytes_at_end, None);
            }
        }

        // Optional 1 Byte imm
        if let Some(imm) = self.imm {
            sink.put1(imm);
        }
    }
}

/// The VEX format allows choosing a vector length in the `L` bit.
pub enum VexVectorLength {
    V128,
    V256,
}

impl VexVectorLength {
    /// Encode the `L` bit.
    fn bits(&self) -> u8 {
        match self {
            Self::V128 => 0b0,
            Self::V256 => 0b1,
        }
    }
}

impl Default for VexVectorLength {
    fn default() -> Self {
        Self::V128
    }
}
