//! Encoding logic for REX instructions.

#![allow(clippy::bool_to_int_with_if)]

use crate::api::CodeSink;

pub(crate) fn low8_will_sign_extend_to_32(x: u32) -> bool {
    #[allow(clippy::cast_possible_wrap)]
    let xs = x as i32;
    xs == ((xs << 24) >> 24)
}

/// Encode the ModR/M byte.
#[inline]
pub fn encode_modrm(m0d: u8, enc_reg_g: u8, rm_e: u8) -> u8 {
    debug_assert!(m0d < 4);
    debug_assert!(enc_reg_g < 8);
    debug_assert!(rm_e < 8);
    ((m0d & 3) << 6) | ((enc_reg_g & 7) << 3) | (rm_e & 7)
}

/// Encode the SIB byte (scale-index-base).
#[inline]
pub fn encode_sib(scale: u8, enc_index: u8, enc_base: u8) -> u8 {
    debug_assert!(scale < 4);
    debug_assert!(enc_index < 8);
    debug_assert!(enc_base < 8);
    ((scale & 3) << 6) | ((enc_index & 7) << 3) | (enc_base & 7)
}

/// Write a suitable number of bits from an imm64 to the sink.
#[allow(clippy::cast_possible_truncation)]
pub fn emit_simm(sink: &mut impl CodeSink, size: u8, simm32: u32) {
    match size {
        8 | 4 => sink.put4(simm32),
        2 => sink.put2(simm32 as u16),
        1 => sink.put1(simm32 as u8),
        _ => unreachable!(),
    }
}

/// A small bit field to record a REX prefix specification:
/// - bit 0 set to 1 indicates REX.W must be 0 (cleared).
/// - bit 1 set to 1 indicates the REX prefix must always be emitted.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct RexFlags(u8);

impl RexFlags {
    /// By default, set the W field, and don't always emit.
    #[inline]
    #[must_use]
    pub fn set_w() -> Self {
        Self(0)
    }

    /// Creates a new REX prefix for which the REX.W bit will be cleared.
    #[inline]
    #[must_use]
    pub fn clear_w() -> Self {
        Self(1)
    }

    /// True if 64-bit operands are used.
    #[inline]
    #[must_use]
    pub fn must_clear_w(self) -> bool {
        (self.0 & 1) != 0
    }

    /// Require that the REX prefix is emitted.
    #[inline]
    pub fn always_emit(&mut self) -> &mut Self {
        self.0 |= 2;
        self
    }

    /// True if the REX prefix must always be emitted.
    #[inline]
    #[must_use]
    pub fn must_always_emit(self) -> bool {
        (self.0 & 2) != 0
    }

    /// Force emission of the REX byte if the register is: `rsp`, `rbp`, `rsi`,
    /// `rdi`.
    pub fn always_emit_if_8bit_needed(&mut self, enc: u8) {
        if (4..=7).contains(&enc) {
            self.always_emit();
        }
    }

    /// Emit a unary instruction.
    #[inline]
    pub fn emit_one_op(self, sink: &mut impl CodeSink, enc_e: u8) {
        // Register Operand coded in Opcode Byte
        // REX.R and REX.X unused
        // REX.B == 1 accesses r8-r15
        let w = if self.must_clear_w() { 0 } else { 1 };
        let r = 0;
        let x = 0;
        let b = (enc_e >> 3) & 1;
        let rex = 0x40 | (w << 3) | (r << 2) | (x << 1) | b;
        if rex != 0x40 || self.must_always_emit() {
            sink.put1(rex);
        }
    }

    /// Emit a binary instruction.
    #[inline]
    pub fn emit_two_op(self, sink: &mut impl CodeSink, enc_g: u8, enc_e: u8) {
        let w = if self.must_clear_w() { 0 } else { 1 };
        let r = (enc_g >> 3) & 1;
        let x = 0;
        let b = (enc_e >> 3) & 1;
        let rex = 0x40 | (w << 3) | (r << 2) | (x << 1) | b;
        if rex != 0x40 || self.must_always_emit() {
            sink.put1(rex);
        }
    }

    /// Emit a ternary instruction.
    #[inline]
    pub fn emit_three_op(self, sink: &mut impl CodeSink, enc_g: u8, enc_index: u8, enc_base: u8) {
        let w = if self.must_clear_w() { 0 } else { 1 };
        let r = (enc_g >> 3) & 1;
        let x = (enc_index >> 3) & 1;
        let b = (enc_base >> 3) & 1;
        let rex = 0x40 | (w << 3) | (r << 2) | (x << 1) | b;
        if rex != 0x40 || self.must_always_emit() {
            sink.put1(rex);
        }
    }
}

#[derive(Copy, Clone)]
#[allow(missing_docs)]
pub enum Imm {
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
    pub fn new(val: i32, evex_scaling: Option<i8>) -> Imm {
        if val == 0 {
            return Imm::None;
        }
        match evex_scaling {
            Some(scaling) => {
                if val % i32::from(scaling) == 0 {
                    let scaled = val / i32::from(scaling);
                    #[allow(clippy::cast_sign_loss)]
                    if low8_will_sign_extend_to_32(scaled as u32) {
                        #[allow(clippy::cast_possible_truncation)]
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
    pub fn force_immediate(&mut self) {
        if let Imm::None = self {
            *self = Imm::Imm8(0);
        }
    }

    /// Returns the two "mod" bits present at the upper bits of the mod/rm
    /// byte.
    pub fn m0d(self) -> u8 {
        match self {
            Imm::None => 0b00,
            Imm::Imm8(_) => 0b01,
            Imm::Imm32(_) => 0b10,
        }
    }

    /// Emit the truncated immediate into the code sink.
    #[allow(clippy::cast_sign_loss)]
    pub fn emit(self, sink: &mut impl CodeSink) {
        match self {
            Imm::None => {}
            Imm::Imm8(n) => sink.put1(n as u8),
            Imm::Imm32(n) => sink.put4(n as u32),
        }
    }
}
