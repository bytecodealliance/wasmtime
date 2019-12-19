//! Provides a named interface to the `u16` Encoding bits.

use std::ops::RangeInclusive;

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
#[derive(Copy, Clone, PartialEq)]
pub struct EncodingBits(u16);
const OPCODE: RangeInclusive<u16> = 0..=7;
const OPCODE_PREFIX: RangeInclusive<u16> = 8..=11; // Includes pp and mm.
const RRR: RangeInclusive<u16> = 12..=14;
const REX_W: RangeInclusive<u16> = 15..=15;

impl From<u16> for EncodingBits {
    fn from(bits: u16) -> Self {
        Self(bits)
    }
}

impl EncodingBits {
    /// Constructs a new EncodingBits from parts.
    pub fn new(op_bytes: &[u8], rrr: u16, rex_w: u16) -> Self {
        assert!(
            !op_bytes.is_empty(),
            "op_bytes must include at least one opcode byte"
        );
        let mut new = Self::from(0);
        let last_byte = op_bytes[op_bytes.len() - 1];
        new.write(OPCODE, last_byte as u16);
        let prefix: u8 = OpcodePrefix::from_opcode(op_bytes).into();
        new.write(OPCODE_PREFIX, prefix as u16);
        new.write(RRR, rrr);
        new.write(REX_W, rex_w);
        new
    }

    /// Returns a copy of the EncodingBits with the RRR bits set.
    #[inline]
    pub fn with_rrr(self, rrr: u8) -> Self {
        debug_assert_eq!(u8::from(self.rrr()), 0);
        let mut enc = self.clone();
        enc.write(RRR, rrr.into());
        enc
    }

    /// Returns a copy of the EncodingBits with the REX.W bit set.
    #[inline]
    pub fn with_rex_w(self) -> Self {
        debug_assert_eq!(self.rex_w(), 0);
        let mut enc = self.clone();
        enc.write(REX_W, 1);
        enc
    }

    /// Returns the raw bits.
    #[inline]
    pub fn bits(self) -> u16 {
        self.0
    }

    /// Convenience method for writing bits to specific range.
    #[inline]
    fn write(&mut self, range: RangeInclusive<u16>, value: u16) {
        assert!(ExactSizeIterator::len(&range) > 0);
        let size = range.end() - range.start() + 1; // Calculate the number of bits in the range.
        let mask = (1 << size) - 1; // Generate a bit mask.
        debug_assert!(
            value <= mask,
            "The written value should have fewer than {} bits.",
            size
        );
        let mask_complement = !(mask << *range.start()); // Create the bitwise complement for the clear mask.
        self.0 &= mask_complement; // Clear the bits in `range`.
        let value = (value & mask) << *range.start(); // Place the value in the correct location.
        self.0 |= value; // Modify the bits in `range`.
    }

    /// Convenience method for reading bits from a specific range.
    #[inline]
    fn read(self, range: RangeInclusive<u16>) -> u8 {
        assert!(ExactSizeIterator::len(&range) > 0);
        let size = range.end() - range.start() + 1; // Calculate the number of bits in the range.
        debug_assert!(size <= 8, "This structure expects ranges of at most 8 bits");
        let mask = (1 << size) - 1; // Generate a bit mask.
        ((self.0 >> *range.start()) & mask) as u8
    }

    /// Instruction opcode byte, without the prefix.
    #[inline]
    pub fn opcode_byte(self) -> u8 {
        self.read(OPCODE)
    }

    /// Prefix kind for the instruction, as an enum.
    #[inline]
    pub fn prefix(self) -> OpcodePrefix {
        OpcodePrefix::from(self.read(OPCODE_PREFIX))
    }

    /// Extracts the PP bits of the OpcodePrefix.
    #[inline]
    pub fn pp(self) -> u8 {
        self.prefix().to_primitive() & 0x3
    }

    /// Extracts the MM bits of the OpcodePrefix.
    #[inline]
    pub fn mm(self) -> u8 {
        (self.prefix().to_primitive() >> 2) & 0x3
    }

    /// Bits for the ModR/M byte for certain opcodes.
    #[inline]
    pub fn rrr(self) -> u8 {
        self.read(RRR)
    }

    /// REX.W bit (or VEX.W/E).
    #[inline]
    pub fn rex_w(self) -> u8 {
        self.read(REX_W)
    }
}

/// Opcode prefix representation.
///
/// The prefix type occupies four of the EncodingBits.
#[allow(non_camel_case_types)]
#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum OpcodePrefix {
    Op1,
    Mp1_66,
    Mp1_f3,
    Mp1_f2,
    Op2_0f,
    Mp2_66_0f,
    Mp2_f3_0f,
    Mp2_f2_0f,
    Op3_0f_38,
    Mp3_66_0f_38,
    Mp3_f3_0f_38,
    Mp3_f2_0f_38,
    Op3_0f_3a,
    Mp3_66_0f_3a,
    Mp3_f3_0f_3a,
    Mp3_f2_0f_3a,
}

impl From<u8> for OpcodePrefix {
    fn from(n: u8) -> Self {
        use OpcodePrefix::*;
        match n {
            0b0000 => Op1,
            0b0001 => Mp1_66,
            0b0010 => Mp1_f3,
            0b0011 => Mp1_f2,
            0b0100 => Op2_0f,
            0b0101 => Mp2_66_0f,
            0b0110 => Mp2_f3_0f,
            0b0111 => Mp2_f2_0f,
            0b1000 => Op3_0f_38,
            0b1001 => Mp3_66_0f_38,
            0b1010 => Mp3_f3_0f_38,
            0b1011 => Mp3_f2_0f_38,
            0b1100 => Op3_0f_3a,
            0b1101 => Mp3_66_0f_3a,
            0b1110 => Mp3_f3_0f_3a,
            0b1111 => Mp3_f2_0f_3a,
            _ => panic!("invalid opcode prefix"),
        }
    }
}

impl Into<u8> for OpcodePrefix {
    fn into(self) -> u8 {
        use OpcodePrefix::*;
        match self {
            Op1 => 0b0000,
            Mp1_66 => 0b0001,
            Mp1_f3 => 0b0010,
            Mp1_f2 => 0b0011,
            Op2_0f => 0b0100,
            Mp2_66_0f => 0b0101,
            Mp2_f3_0f => 0b0110,
            Mp2_f2_0f => 0b0111,
            Op3_0f_38 => 0b1000,
            Mp3_66_0f_38 => 0b1001,
            Mp3_f3_0f_38 => 0b1010,
            Mp3_f2_0f_38 => 0b1011,
            Op3_0f_3a => 0b1100,
            Mp3_66_0f_3a => 0b1101,
            Mp3_f3_0f_3a => 0b1110,
            Mp3_f2_0f_3a => 0b1111,
        }
    }
}

impl OpcodePrefix {
    /// Convert an opcode prefix to a `u8`; this is a convenience proxy for `Into<u8>`.
    fn to_primitive(self) -> u8 {
        self.into()
    }

    /// Extracts the OpcodePrefix from the opcode.
    pub fn from_opcode(op_bytes: &[u8]) -> Self {
        assert!(!op_bytes.is_empty(), "at least one opcode byte");

        let prefix_bytes = &op_bytes[..op_bytes.len() - 1];
        match prefix_bytes {
            [] => Self::Op1,
            [0x66] => Self::Mp1_66,
            [0xf3] => Self::Mp1_f3,
            [0xf2] => Self::Mp1_f2,
            [0x0f] => Self::Op2_0f,
            [0x66, 0x0f] => Self::Mp2_66_0f,
            [0xf3, 0x0f] => Self::Mp2_f3_0f,
            [0xf2, 0x0f] => Self::Mp2_f2_0f,
            [0x0f, 0x38] => Self::Op3_0f_38,
            [0x66, 0x0f, 0x38] => Self::Mp3_66_0f_38,
            [0xf3, 0x0f, 0x38] => Self::Mp3_f3_0f_38,
            [0xf2, 0x0f, 0x38] => Self::Mp3_f2_0f_38,
            [0x0f, 0x3a] => Self::Op3_0f_3a,
            [0x66, 0x0f, 0x3a] => Self::Mp3_66_0f_3a,
            [0xf3, 0x0f, 0x3a] => Self::Mp3_f3_0f_3a,
            [0xf2, 0x0f, 0x3a] => Self::Mp3_f2_0f_3a,
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

    #[test]
    fn prefix_to_name() {
        assert_eq!(OpcodePrefix::Op1.recipe_name_prefix(), "Op1");
        assert_eq!(OpcodePrefix::Op2_0f.recipe_name_prefix(), "Op2");
        assert_eq!(OpcodePrefix::Op3_0f_38.recipe_name_prefix(), "Op3");
        assert_eq!(OpcodePrefix::Mp1_66.recipe_name_prefix(), "Mp1");
        assert_eq!(OpcodePrefix::Mp2_66_0f.recipe_name_prefix(), "Mp2");
        assert_eq!(OpcodePrefix::Mp3_66_0f_3a.recipe_name_prefix(), "Mp3");
    }

    /// Tests that the opcode_byte is the lower of the EncodingBits.
    #[test]
    fn encodingbits_opcode_byte() {
        let enc = EncodingBits::from(0x00ff);
        assert_eq!(enc.opcode_byte(), 0xff);
        assert_eq!(enc.prefix().to_primitive(), 0x0);
        assert_eq!(enc.rrr(), 0x0);
        assert_eq!(enc.rex_w(), 0x0);

        let enc = EncodingBits::from(0x00cd);
        assert_eq!(enc.opcode_byte(), 0xcd);
    }

    /// Tests that the OpcodePrefix is encoded correctly.
    #[test]
    fn encodingbits_prefix() {
        let enc = EncodingBits::from(0x0c00);
        assert_eq!(enc.opcode_byte(), 0x00);
        assert_eq!(enc.prefix().to_primitive(), 0xc);
        assert_eq!(enc.prefix(), OpcodePrefix::Op3_0f_3a);
        assert_eq!(enc.rrr(), 0x0);
        assert_eq!(enc.rex_w(), 0x0);
    }

    /// Tests that the PP bits are encoded correctly.
    #[test]
    fn encodingbits_pp() {
        let enc = EncodingBits::from(0x0300);
        assert_eq!(enc.opcode_byte(), 0x0);
        assert_eq!(enc.pp(), 0x3);
        assert_eq!(enc.mm(), 0x0);
        assert_eq!(enc.rrr(), 0x0);
        assert_eq!(enc.rex_w(), 0x0);
    }

    /// Tests that the MM bits are encoded correctly.
    #[test]
    fn encodingbits_mm() {
        let enc = EncodingBits::from(0x0c00);
        assert_eq!(enc.opcode_byte(), 0x0);
        assert_eq!(enc.pp(), 0x00);
        assert_eq!(enc.mm(), 0x3);
        assert_eq!(enc.rrr(), 0x0);
        assert_eq!(enc.rex_w(), 0x0);
    }

    /// Tests that the ModR/M bits are encoded correctly.
    #[test]
    fn encodingbits_rrr() {
        let enc = EncodingBits::from(0x5000);
        assert_eq!(enc.opcode_byte(), 0x0);
        assert_eq!(enc.prefix().to_primitive(), 0x0);
        assert_eq!(enc.rrr(), 0x5);
        assert_eq!(enc.rex_w(), 0x0);
    }

    /// Tests that the REX.W bit is encoded correctly.
    #[test]
    fn encodingbits_rex_w() {
        let enc = EncodingBits::from(0x8000);
        assert_eq!(enc.opcode_byte(), 0x00);
        assert_eq!(enc.prefix().to_primitive(), 0x0);
        assert_eq!(enc.rrr(), 0x0);
        assert_eq!(enc.rex_w(), 0x1);
    }

    /// Tests setting and unsetting a bit using EncodingBits::write.
    #[test]
    fn encodingbits_flip() {
        let mut bits = EncodingBits::from(0);
        let range = 2..=2;

        bits.write(range.clone(), 1);
        assert_eq!(bits.bits(), 0b100);

        bits.write(range, 0);
        assert_eq!(bits.bits(), 0b000);
    }

    /// Tests a round-trip of EncodingBits from/to a u16 (hardcoded endianness).
    #[test]
    fn encodingbits_roundtrip() {
        let bits: u16 = 0x1234;
        assert_eq!(EncodingBits::from(bits).bits(), bits);
    }

    #[test]
    // I purposely want to divide the bits using the ranges defined above.
    #[allow(clippy::inconsistent_digit_grouping)]
    fn encodingbits_construction() {
        assert_eq!(
            EncodingBits::new(&[0x66, 0x40], 5, 1).bits(),
            0b1_101_0001_01000000 // 1 = rex_w, 101 = rrr, 0001 = prefix, 01000000 = opcode
        );
    }

    #[test]
    #[should_panic]
    fn encodingbits_panics_at_write_to_invalid_range() {
        EncodingBits::from(0).write(1..=0, 42);
    }

    #[test]
    #[should_panic]
    fn encodingbits_panics_at_read_to_invalid_range() {
        EncodingBits::from(0).read(1..=0);
    }
}
