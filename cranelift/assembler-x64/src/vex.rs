//! Encoding logic for VEX instructions.
use super::XmmMem;
use super::rex;
use crate::Amode;
use crate::api::{AsReg, CodeSink, KnownOffsetTable, Registers};
use crate::mem::emit_modrm_sib_disp;

/// Allows using the same opcode byte in different "opcode maps" to allow for more instruction
/// encodings. See appendix A in the Intel Software Developer's Manual, volume 2A, for more details.
#[derive(PartialEq)]
pub enum OpcodeMap {
    _0F,
    _0F38,
    _0F3A,
}

impl OpcodeMap {
    /// Normally the opcode map is specified as bytes in the instruction, but some x64 encoding
    /// formats pack this information as bits in a prefix (e.g. VEX / EVEX).
    pub fn bits(&self) -> u8 {
        match self {
            OpcodeMap::_0F => 0b01,
            OpcodeMap::_0F38 => 0b10,
            OpcodeMap::_0F3A => 0b11,
        }
    }
}

/// We may need to include one or more legacy prefix bytes before the REX prefix.  This enum
/// covers only the small set of possibilities that we actually need.
#[derive(PartialEq)]
pub enum VexPP {
    /// No prefix bytes.
    None,
    /// Operand Size Override -- here, denoting "16-bit operation".
    _66,
    /// REPNE, but no specific meaning here -- is just an opcode extension.
    _F2,
    /// REP/REPE, but no specific meaning here -- is just an opcode extension.
    _F3,
}

impl VexPP {
    /// Emit the legacy prefix as bits (e.g. for EVEX instructions).
    #[inline(always)]
    pub(crate) fn bits(&self) -> u8 {
        match self {
            Self::None => 0b00,
            Self::_66 => 0b01,
            Self::_F3 => 0b10,
            Self::_F2 => 0b11,
        }
    }
}

pub struct VexInstruction<R: Registers> {
    pub opcode: u8,
    pub length: VexVectorLength,
    pub prefix: VexPP,
    pub map: OpcodeMap,
    pub reg: u8,
    pub vvvv: Option<u8>,
    pub rm: Option<XmmMem<R::ReadXmm, R::ReadGpr>>,
    pub imm: Option<u8>,
    pub w: bool,
}

pub fn vex_instruction<R: Registers>(
    opcode: u8,
    length: VexVectorLength,
    prefix: VexPP,
    map: OpcodeMap,
    reg: u8,
    vvvv: Option<u8>,
    rm: Option<XmmMem<R::ReadXmm, R::ReadGpr>>,
    imm: Option<u8>,
) -> VexInstruction<R> {
    VexInstruction {
        opcode,
        length,
        prefix,
        map,
        reg,
        vvvv,
        rm,
        imm,
        w: false,
    }
}

impl<R: Registers> VexInstruction<R> {
    /// The R bit in encoded format (inverted).
    #[inline(always)]
    fn r_bit(&self) -> u8 {
        (!(self.reg >> 3)) & 1
    }

    /// The X bit in encoded format (inverted).
    #[inline(always)]
    fn x_bit(&self) -> u8 {
        let reg = match &self.rm {
            Some(XmmMem::Xmm(_xmm)) => 0,
            Some(XmmMem::Mem(Amode::ImmReg { .. })) => 0,
            Some(XmmMem::Mem(Amode::ImmRegRegShift { index, .. })) => index.enc(),
            Some(XmmMem::Mem(Amode::RipRelative { .. })) => 0,
            None => unreachable!("VEX encoding requires a valid rm operand"),
        };

        !(reg >> 3) & 1
    }

    /// The B bit in encoded format (inverted).
    #[inline(always)]
    fn b_bit(&self) -> u8 {
        let reg = match &self.rm {
            Some(XmmMem::Xmm(xmm)) => (*xmm).enc(),
            Some(XmmMem::Mem(Amode::ImmReg { base, .. })) => base.enc(),
            Some(XmmMem::Mem(Amode::ImmRegRegShift { base, .. })) => base.enc(),
            Some(XmmMem::Mem(Amode::RipRelative { .. })) => 0,
            None => unreachable!("VEX encoding requires a valid rm operand"),
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
        let vvvv = self.vvvv.unwrap_or(0x00);

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
            Some(XmmMem::Xmm(xmm)) => {
                let rm: u8 = (*xmm).enc();
                sink.put1(rex::encode_modrm(3, self.reg & 7, rm & 7));
            }
            // For address-based modes reuse the logic from the `rex` module
            // for the modrm and trailing bytes since VEX uses the same
            // encoding.
            Some(XmmMem::Mem(amode)) => {
                let bytes_at_end = if self.imm.is_some() { 1 } else { 0 };
                emit_modrm_sib_disp(sink, off, self.reg & 7, amode, bytes_at_end, None);
            }
            None => unreachable!("VEX encoding requires a valid rm operand"),
        }

        // Optional 1 Byte imm
        if let Some(imm) = self.imm {
            sink.put1(imm);
        }
    }
}

/// The VEX format allows choosing a vector length in the `L` bit.
pub enum VexVectorLength {
    _128,
}

impl VexVectorLength {
    /// Encode the `L` bit.
    fn bits(&self) -> u8 {
        match self {
            Self::_128 => 0b0,
        }
    }
}

impl Default for VexVectorLength {
    fn default() -> Self {
        Self::_128
    }
}
