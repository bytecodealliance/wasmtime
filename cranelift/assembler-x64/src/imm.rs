//! Immediate operands to instructions.

#![allow(clippy::module_name_repetitions)]
#![allow(unused_comparisons)] // Necessary to use maybe_print_hex! with `u*` values.
#![allow(clippy::cast_possible_wrap)] // Necessary to cast to `i*` for sign extension.

use crate::api::{KnownOffset, KnownOffsetTable};

/// This helper function prints the hexadecimal representation of the immediate
/// value, but only if the value is greater than or equal to 10. This is
/// necessary to match how Capstone pretty-prints immediate values.
macro_rules! maybe_print_hex {
    ($n:expr) => {
        if $n >= 0 && $n < 10 {
            format!("${:x}", $n)
        } else {
            format!("$0x{:x}", $n)
        }
    };
}

/// An 8-bit immediate operand.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(any(test, feature = "fuzz"), derive(arbitrary::Arbitrary))]
pub struct Imm8(u8);

impl Imm8 {
    #[must_use]
    pub fn new(value: u8) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn value(&self) -> u8 {
        self.0
    }

    pub fn encode(&self, sink: &mut impl CodeSink) {
        sink.put1(self.0);
    }

    #[must_use]
    pub fn to_string(&self, extend: Extension) -> String {
        use Extension::{None, SignExtendLong, SignExtendQuad, SignExtendWord, ZeroExtend};
        match extend {
            None => maybe_print_hex!(self.0),
            SignExtendWord => maybe_print_hex!(i16::from(self.0 as i8)),
            SignExtendLong => maybe_print_hex!(i32::from(self.0 as i8)),
            SignExtendQuad => maybe_print_hex!(i64::from(self.0 as i8)),
            ZeroExtend => maybe_print_hex!(u64::from(self.0)),
        }
    }
}

/// A 16-bit immediate operand.
#[derive(Clone, Debug)]
#[cfg_attr(any(test, feature = "fuzz"), derive(arbitrary::Arbitrary))]
pub struct Imm16(u16);

impl Imm16 {
    #[must_use]
    pub fn new(value: u16) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn value(&self) -> u16 {
        self.0
    }

    pub fn encode(&self, sink: &mut impl CodeSink) {
        sink.put2(self.0);
    }

    #[must_use]
    pub fn to_string(&self, extend: Extension) -> String {
        use Extension::{None, SignExtendLong, SignExtendQuad, SignExtendWord, ZeroExtend};
        match extend {
            None => maybe_print_hex!(self.0),
            SignExtendWord => maybe_print_hex!(self.0 as i16),
            SignExtendLong => maybe_print_hex!(i32::from(self.0 as i16)),
            SignExtendQuad => maybe_print_hex!(i64::from(self.0 as i16)),
            ZeroExtend => maybe_print_hex!(u64::from(self.0)),
        }
    }
}

/// A 32-bit immediate operand.
///
/// Note that, "in 64-bit mode, the typical size of immediate operands remains
/// 32 bits. When the operand size is 64 bits, the processor sign-extends all
/// immediates to 64 bits prior to their use" (Intel SDM Vol. 2, 2.2.1.5).
#[derive(Clone, Debug)]
#[cfg_attr(any(test, feature = "fuzz"), derive(arbitrary::Arbitrary))]
pub struct Imm32(u32);

impl Imm32 {
    #[must_use]
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn value(&self) -> u32 {
        self.0
    }

    pub fn encode(&self, sink: &mut impl CodeSink) {
        sink.put4(self.0);
    }

    #[must_use]
    pub fn to_string(&self, extend: Extension) -> String {
        use Extension::{None, SignExtendLong, SignExtendQuad, SignExtendWord, ZeroExtend};
        match extend {
            None => maybe_print_hex!(self.0),
            SignExtendWord => unreachable!("cannot sign extend a 32-bit value"),
            SignExtendLong => maybe_print_hex!(self.0 as i32),
            SignExtendQuad => maybe_print_hex!(i64::from(self.0 as i32)),
            ZeroExtend => maybe_print_hex!(u64::from(self.0)),
        }
    }
}

/// A 32-bit immediate like [`Imm32`], but with slightly different
/// pretty-printing.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(any(test, feature = "fuzz"), derive(arbitrary::Arbitrary))]
pub struct Simm32(i32);

impl Simm32 {
    #[must_use]
    pub fn new(value: i32) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn value(self) -> i32 {
        self.0
    }
}

impl From<i32> for Simm32 {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

impl std::fmt::LowerHex for Simm32 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.0 == 0 {
            return Ok(());
        }
        if self.0 < 0 {
            write!(f, "-")?;
        }
        if self.0 > 9 || self.0 < -9 {
            write!(f, "0x")?;
        }
        let abs = match self.0.checked_abs() {
            Some(i) => i,
            None => -2_147_483_648,
        };
        std::fmt::LowerHex::fmt(&abs, f)
    }
}

/// A [`Simm32`] immediate with an optional known offset.
///
/// Cranelift does not know certain offsets until emission time. To accommodate
/// Cranelift, this structure stores an optional [`KnownOffset`]. The following
/// happens immediately before emission:
/// - the [`KnownOffset`] is looked up, mapping it to an offset value
/// - the [`Simm32`] value is added to the offset value
#[derive(Clone, Debug)]
pub struct Simm32PlusKnownOffset {
    pub simm32: Simm32,
    pub offset: Option<KnownOffset>,
}

impl Simm32PlusKnownOffset {
    /// # Panics
    ///
    /// Panics if the sum of the immediate and the known offset value overflows.
    #[must_use]
    pub fn value(&self, offsets: &impl KnownOffsetTable) -> i32 {
        let known_offset = match self.offset {
            Some(offset) => offsets[offset],
            None => 0,
        };
        known_offset
            .checked_add(self.simm32.value())
            .expect("no wrapping")
    }
}

impl std::fmt::LowerHex for Simm32PlusKnownOffset {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(offset) = self.offset {
            write!(f, "<offset:{offset}>+")?;
        }
        std::fmt::LowerHex::fmt(&self.simm32, f)
    }
}

/// Define the ways an immediate may be sign- or zero-extended.
#[derive(Clone, Copy, Debug)]
pub enum Extension {
    None,
    SignExtendQuad,
    SignExtendLong,
    SignExtendWord,
    ZeroExtend,
}
