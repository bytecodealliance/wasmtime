//! Encodes EVEX instructions. These instructions are those added by the AVX-512 extensions.
use super::rex::encode_modrm;
use super::CodeSink;
use cranelift_codegen_shared::isa::x86::EncodingBits;

/// Encode an EVEX instruction, including the prefixes, the instruction opcode, and the ModRM byte.
/// This EVEX encoding function only encodes the `reg` (operand 1), `vvvv` (operand 2), `rm`
/// (operand 3) form; other forms are possible (see section 2.6.2, Intel Software Development
/// Manual, volume 2A), requiring refactoring of this function or separate functions for each form
/// (e.g. as for the REX prefix).
pub fn encode_evex<CS: CodeSink + ?Sized>(
    enc: EncodingBits,
    reg: impl Into<Register>,
    vvvvv: impl Into<Register>,
    rm: impl Into<Register>,
    context: EvexContext,
    masking: EvexMasking,
    sink: &mut CS,
) {
    let reg = reg.into();
    let rm = rm.into();
    let vvvvv = vvvvv.into();

    // EVEX prefix.
    sink.put1(0x62);

    debug_assert!(enc.mm() < 0b100);
    let mut p0 = enc.mm() & 0b11;
    p0 |= evex2(rm, reg) << 4; // bits 3:2 are always unset
    sink.put1(p0);

    let mut p1 = enc.pp() | 0b100; // bit 2 is always set
    p1 |= (!(vvvvv.0) & 0b1111) << 3;
    p1 |= (enc.rex_w() & 0b1) << 7;
    sink.put1(p1);

    let mut p2 = masking.aaa_bits();
    p2 |= (!(vvvvv.0 >> 4) & 0b1) << 3;
    p2 |= context.bits() << 4;
    p2 |= masking.z_bit() << 7;
    sink.put1(p2);

    // Opcode.
    sink.put1(enc.opcode_byte());

    // ModR/M byte.
    sink.put1(encode_modrm(3, reg.0 & 7, rm.0 & 7))
}

/// Encode the RXBR' bits of the EVEX P0 byte. For an explanation of these bits, see section 2.6.1
/// in the Intel Software Development Manual, volume 2A. These bits can be used by different
/// addressing modes (see section 2.6.2), requiring different `vex*` functions than this one.
fn evex2(rm: Register, reg: Register) -> u8 {
    let b = !(rm.0 >> 3) & 1;
    let x = !(rm.0 >> 4) & 1;
    let r = !(reg.0 >> 3) & 1;
    let r_ = !(reg.0 >> 4) & 1;
    0x00 | r_ | (b << 1) | (x << 2) | (r << 3)
}

#[derive(Copy, Clone)]
pub struct Register(u8);
impl From<u8> for Register {
    fn from(reg: u8) -> Self {
        debug_assert!(reg < 16);
        Self(reg)
    }
}

/// Defines the EVEX context for the `L'`, `L`, and `b` bits (bits 6:4 of EVEX P2 byte). Table 2-36 in
/// section 2.6.10 (Intel Software Development Manual, volume 2A) describes how these bits can be
/// used together for certain classes of instructions; i.e., special care should be taken to ensure
/// that instructions use an applicable correct `EvexContext`. Table 2-39 contains cases where
/// opcodes can result in an #UD.
#[allow(dead_code)] // Rounding and broadcast modes are not yet used.
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

impl EvexContext {
    /// Construct an EVEX context for 128-bit SIMD instructions.
    pub fn v128() -> Self {
        Self::Other {
            length: EvexVectorLength::V128,
        }
    }

    /// Encode the `L'`, `L`, and `b` bits (bits 6:4 of EVEX P2 byte) for merging with the P2 byte.
    fn bits(&self) -> u8 {
        match self {
            Self::RoundingRegToRegFP { rc } => 0b001 | rc.bits() << 1,
            Self::NoRoundingFP { sae, length } => (*sae as u8) | length.bits() << 1,
            Self::MemoryOp { broadcast, length } => (*broadcast as u8) | length.bits() << 1,
            Self::Other { length } => length.bits() << 1,
        }
    }
}

/// The EVEX format allows choosing a vector length in the `L'` and `L` bits; see `EvexContext`.
#[allow(dead_code)] // Wider-length vectors are not yet used.
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

/// The EVEX format allows defining rounding control in the `L'` and `L` bits; see `EvexContext`.
#[allow(dead_code)] // Rounding controls are not yet used.
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
#[allow(dead_code)] // Masking is not yet used.
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
    fn z_bit(&self) -> u8 {
        match self {
            Self::None | Self::Merging { .. } => 0,
            Self::Zeroing { .. } => 1,
        }
    }

    /// Encode the `aaa` bits for merging with the P2 byte.
    fn aaa_bits(&self) -> u8 {
        match self {
            Self::None => 0b000,
            Self::Merging { k } | Self::Zeroing { k } => {
                debug_assert!(*k <= 7);
                *k
            }
        }
    }
}
