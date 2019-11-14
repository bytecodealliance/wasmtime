//! Provides a named interface to the `u16` Encoding bits.

use packed_struct::prelude::*;

/// Named interface to the `u16` Encoding bits, representing an opcode.
///
/// Cranelift requires each recipe to have a single encoding size in bytes.
/// X86 opcodes are variable length, so we use separate recipes for different
/// styles of opcodes and prefixes. The opcode format is indicated by the
/// recipe name prefix.
///
/// VEX/XOP and EVEX prefixes are not yet supported.
/// Encodings using any of these prefixes are represented by separate recipes.
///
/// The encoding bits are:
///
/// 0-7:   The opcode byte <op>.
/// 8-9:   pp, mandatory prefix:
///        00: none (Op*)
///        01: 66   (Mp*)
///        10: F3   (Mp*)
///        11: F2   (Mp*)
/// 10-11: mm, opcode map:
///        00: <op>        (Op1/Mp1)
///        01: 0F <op>     (Op2/Mp2)
///        10: 0F 38 <op>  (Op3/Mp3)
///        11: 0F 3A <op>  (Op3/Mp3)
/// 12-14  rrr, opcode bits for the ModR/M byte for certain opcodes.
/// 15:    REX.W bit (or VEX.W/E)
#[derive(Copy, Clone, PartialEq, PackedStruct)]
#[packed_struct(size_bytes = "2", bit_numbering = "lsb0")]
pub struct EncodingBits {
    /// Instruction opcode byte, without the prefix.
    #[packed_field(bits = "0:7")]
    pub opcode_byte: u8,

    /// Prefix kind for the instruction, as an enum.
    #[packed_field(bits = "8:11", ty = "enum")]
    pub prefix: OpcodePrefix,

    /// Bits for the ModR/M byte for certain opcodes.
    #[packed_field(bits = "12:14")]
    pub rrr: Integer<u8, packed_bits::Bits3>,

    /// REX.W bit (or VEX.W/E).
    #[packed_field(bits = "15")]
    pub rex_w: Integer<u8, packed_bits::Bits1>,
}

impl From<u16> for EncodingBits {
    fn from(bits: u16) -> EncodingBits {
        let bytes: [u8; 2] = [((bits >> 8) & 0xff) as u8, (bits & 0xff) as u8];
        EncodingBits::unpack(&bytes).expect("failed creating EncodingBits")
    }
}

impl EncodingBits {
    /// Constructs a new EncodingBits from parts.
    pub fn new(op_bytes: &[u8], rrr: u16, rex_w: u16) -> Self {
        EncodingBits {
            opcode_byte: op_bytes[op_bytes.len() - 1],
            prefix: OpcodePrefix::from_opcode(op_bytes),
            rrr: (rrr as u8).into(),
            rex_w: (rex_w as u8).into(),
        }
    }

    /// Returns the raw bits.
    #[inline]
    pub fn bits(self) -> u16 {
        let bytes: [u8; 2] = self.pack();
        ((bytes[0] as u16) << 8) | (bytes[1] as u16)
    }

    /// Extracts the PP bits of the OpcodePrefix.
    #[inline]
    pub fn pp(self) -> u8 {
        self.prefix.to_primitive() & 0x3
    }

    /// Extracts the MM bits of the OpcodePrefix.
    #[inline]
    pub fn mm(self) -> u8 {
        (self.prefix.to_primitive() >> 2) & 0x3
    }
}

/// Opcode prefix representation.
///
/// The prefix type occupies four of the EncodingBits.
#[allow(non_camel_case_types)]
#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PrimitiveEnum_u8)]
pub enum OpcodePrefix {
    Op1 = 0b0000,
    Mp1_66 = 0b0001,
    Mp1_f3 = 0b0010,
    Mp1_f2 = 0b0011,
    Op2_0f = 0b0100,
    Mp2_66_0f = 0b0101,
    Mp2_f3_0f = 0b0110,
    Mp2_f2_0f = 0b0111,
    Op3_0f_38 = 0b1000,
    Mp3_66_0f_38 = 0b1001,
    Mp3_f3_0f_38 = 0b1010,
    Mp3_f2_0f_38 = 0b1011,
    Op3_0f_3a = 0b1100,
    Mp3_66_0f_3a = 0b1101,
    Mp3_f3_0f_3a = 0b1110,
    Mp3_f2_0f_3a = 0b1111,
}

impl From<u8> for OpcodePrefix {
    fn from(n: u8) -> OpcodePrefix {
        OpcodePrefix::from_primitive(n).expect("invalid OpcodePrefix")
    }
}

impl OpcodePrefix {
    /// Extracts the OpcodePrefix from the opcode.
    pub fn from_opcode(op_bytes: &[u8]) -> OpcodePrefix {
        assert!(!op_bytes.is_empty(), "at least one opcode byte");

        let prefix_bytes = &op_bytes[..op_bytes.len() - 1];
        match prefix_bytes {
            [] => OpcodePrefix::Op1,
            [0x66] => OpcodePrefix::Mp1_66,
            [0xf3] => OpcodePrefix::Mp1_f3,
            [0xf2] => OpcodePrefix::Mp1_f2,
            [0x0f] => OpcodePrefix::Op2_0f,
            [0x66, 0x0f] => OpcodePrefix::Mp2_66_0f,
            [0xf3, 0x0f] => OpcodePrefix::Mp2_f3_0f,
            [0xf2, 0x0f] => OpcodePrefix::Mp2_f2_0f,
            [0x0f, 0x38] => OpcodePrefix::Op3_0f_38,
            [0x66, 0x0f, 0x38] => OpcodePrefix::Mp3_66_0f_38,
            [0xf3, 0x0f, 0x38] => OpcodePrefix::Mp3_f3_0f_38,
            [0xf2, 0x0f, 0x38] => OpcodePrefix::Mp3_f2_0f_38,
            [0x0f, 0x3a] => OpcodePrefix::Op3_0f_3a,
            [0x66, 0x0f, 0x3a] => OpcodePrefix::Mp3_66_0f_3a,
            [0xf3, 0x0f, 0x3a] => OpcodePrefix::Mp3_f3_0f_3a,
            [0xf2, 0x0f, 0x3a] => OpcodePrefix::Mp3_f2_0f_3a,
            _ => {
                panic!("unexpected opcode sequence: {:?}", op_bytes);
            }
        }
    }

    /// Returns the recipe name prefix.
    ///
    /// At the moment, each similar OpcodePrefix group is given its own Recipe.
    /// In order to distinguish them, this string is prefixed.
    pub fn recipe_name_prefix(self) -> &'static str {
        use OpcodePrefix::*;
        match self {
            Op1 => "Op1",
            Op2_0f => "Op2",
            Op3_0f_38 | Op3_0f_3a => "Op3",
            Mp1_66 | Mp1_f3 | Mp1_f2 => "Mp1",
            Mp2_66_0f | Mp2_f3_0f | Mp2_f2_0f => "Mp2",
            Mp3_66_0f_38 | Mp3_f3_0f_38 | Mp3_f2_0f_38 => "Mp3",
            Mp3_66_0f_3a | Mp3_f3_0f_3a | Mp3_f2_0f_3a => "Mp3",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function for prefix_roundtrip() to avoid long lines.
    fn test_roundtrip(p: OpcodePrefix) {
        assert_eq!(p, OpcodePrefix::from(p.to_primitive()));
    }

    /// Tests that to/from each opcode matches.
    #[test]
    fn prefix_roundtrip() {
        test_roundtrip(OpcodePrefix::Op1);
        test_roundtrip(OpcodePrefix::Mp1_66);
        test_roundtrip(OpcodePrefix::Mp1_f3);
        test_roundtrip(OpcodePrefix::Mp1_f2);
        test_roundtrip(OpcodePrefix::Op2_0f);
        test_roundtrip(OpcodePrefix::Mp2_66_0f);
        test_roundtrip(OpcodePrefix::Mp2_f3_0f);
        test_roundtrip(OpcodePrefix::Mp2_f2_0f);
        test_roundtrip(OpcodePrefix::Op3_0f_38);
        test_roundtrip(OpcodePrefix::Mp3_66_0f_38);
        test_roundtrip(OpcodePrefix::Mp3_f3_0f_38);
        test_roundtrip(OpcodePrefix::Mp3_f2_0f_38);
        test_roundtrip(OpcodePrefix::Op3_0f_3a);
        test_roundtrip(OpcodePrefix::Mp3_66_0f_3a);
        test_roundtrip(OpcodePrefix::Mp3_f3_0f_3a);
        test_roundtrip(OpcodePrefix::Mp3_f2_0f_3a);
    }

    /// Tests that the opcode_byte is the lower of the EncodingBits.
    #[test]
    fn encodingbits_opcode_byte() {
        let enc = EncodingBits::from(0x00ff);
        assert_eq!(enc.opcode_byte, 0xff);
        assert_eq!(enc.prefix.to_primitive(), 0x0);
        assert_eq!(u8::from(enc.rrr), 0x0);
        assert_eq!(u8::from(enc.rex_w), 0x0);

        let enc = EncodingBits::from(0x00cd);
        assert_eq!(enc.opcode_byte, 0xcd);
    }

    /// Tests that the OpcodePrefix is encoded correctly.
    #[test]
    fn encodingbits_prefix() {
        let enc = EncodingBits::from(0x0c00);
        assert_eq!(enc.opcode_byte, 0x00);
        assert_eq!(enc.prefix.to_primitive(), 0xc);
        assert_eq!(enc.prefix, OpcodePrefix::Op3_0f_3a);
        assert_eq!(u8::from(enc.rrr), 0x0);
        assert_eq!(u8::from(enc.rex_w), 0x0);
    }

    /// Tests that the REX.W bit is encoded correctly.
    #[test]
    fn encodingbits_rex_w() {
        let enc = EncodingBits::from(0x8000);
        assert_eq!(enc.opcode_byte, 0x00);
        assert_eq!(enc.prefix.to_primitive(), 0x0);
        assert_eq!(u8::from(enc.rrr), 0x0);
        assert_eq!(u8::from(enc.rex_w), 0x1);
    }

    /// Tests a round-trip of EncodingBits from/to a u16 (hardcoded endianness).
    #[test]
    fn encodingbits_roundtrip() {
        let bits: u16 = 0x1234;
        assert_eq!(EncodingBits::from(bits).bits(), bits);
    }
}
