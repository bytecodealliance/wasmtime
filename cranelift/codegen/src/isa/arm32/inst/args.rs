//! arm32 addressing modes and immediate/operand helpers.

use crate::isa::arm32::inst::*;
use crate::machinst::{OperandVisitor, Reg};

pub use crate::isa::arm32::lower::isle::generated_code::{
    ALUOp, AMode, BitOp, CmpOp, Cond, ExtOp, LoadKind, ShiftOp, StoreKind,
};

/// A memory address resolved to a concrete base register and either an
/// immediate or register offset, ready for encoding.
pub(crate) enum ResolvedAMode {
    /// `[base, #offset]`
    Imm { base: Reg, offset: i32 },
    /// `[base, index]`
    Reg { base: Reg, index: Reg },
}

impl AMode {
    /// Collect the register operands referenced by this addressing mode.
    pub(crate) fn get_operands(&mut self, collector: &mut impl OperandVisitor) {
        // Only virtual registers are tracked; sp is implicit for the
        // stack-relative modes.
        fn use_if_virtual(collector: &mut impl OperandVisitor, reg: &mut Reg) {
            if reg.to_real_reg().is_none() {
                collector.reg_use(reg);
            }
        }
        match self {
            AMode::RegOffset { rn, .. } => use_if_virtual(collector, rn),
            AMode::RegReg { rn, rm } => {
                use_if_virtual(collector, rn);
                use_if_virtual(collector, rm);
            }
            AMode::SPOffset { .. } | AMode::SlotOffset { .. } | AMode::IncomingArg { .. } => {}
        }
    }

    /// Resolve this addressing mode against the current frame layout.
    pub(crate) fn resolve(&self, state: &EmitState) -> ResolvedAMode {
        match self {
            &AMode::RegOffset { rn, offset } => ResolvedAMode::Imm { base: rn, offset },
            &AMode::RegReg { rn, rm } => ResolvedAMode::Reg {
                base: rn,
                index: rm,
            },
            &AMode::SPOffset { offset } => ResolvedAMode::Imm {
                base: stack_reg(),
                offset: offset as i32,
            },
            &AMode::SlotOffset { offset } => {
                let fl = state.frame_layout();
                ResolvedAMode::Imm {
                    base: stack_reg(),
                    offset: (offset + i64::from(fl.outgoing_args_size)) as i32,
                }
            }
            &AMode::IncomingArg { offset } => {
                let fl = state.frame_layout();
                let sp_off = i64::from(fl.tail_args_size)
                    + i64::from(fl.setup_area_size)
                    + i64::from(fl.clobber_size)
                    + i64::from(fl.fixed_frame_storage_size)
                    + i64::from(fl.outgoing_args_size)
                    - offset;
                ResolvedAMode::Imm {
                    base: stack_reg(),
                    offset: sp_off as i32,
                }
            }
        }
    }

    /// Pretty-print this addressing mode.
    pub(crate) fn pretty_print(&self) -> String {
        match self {
            AMode::RegOffset { rn, offset } => {
                alloc::format!("[{}, #{}]", reg_name(*rn), offset)
            }
            AMode::RegReg { rn, rm } => {
                alloc::format!("[{}, {}]", reg_name(*rn), reg_name(*rm))
            }
            AMode::SPOffset { offset } => alloc::format!("[sp, #{offset}]"),
            AMode::SlotOffset { offset } => alloc::format!("[slot, #{offset}]"),
            AMode::IncomingArg { offset } => alloc::format!("[incoming_arg, #{offset}]"),
        }
    }
}

impl ALUOp {
    /// The 4-bit data-processing opcode for this operation.
    pub(crate) fn opcode(self) -> u32 {
        match self {
            ALUOp::And => 0b0000,
            ALUOp::Eor => 0b0001,
            ALUOp::Sub => 0b0010,
            ALUOp::Rsb => 0b0011,
            ALUOp::Add => 0b0100,
            ALUOp::Adc => 0b0101,
            ALUOp::Sbc => 0b0110,
            ALUOp::Orr => 0b1100,
            ALUOp::Bic => 0b1110,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            ALUOp::And => "and",
            ALUOp::Eor => "eor",
            ALUOp::Sub => "sub",
            ALUOp::Rsb => "rsb",
            ALUOp::Add => "add",
            ALUOp::Adc => "adc",
            ALUOp::Sbc => "sbc",
            ALUOp::Orr => "orr",
            ALUOp::Bic => "bic",
        }
    }
}

impl CmpOp {
    /// The 4-bit data-processing opcode (with the S bit set separately).
    pub(crate) fn opcode(self) -> u32 {
        match self {
            CmpOp::Tst => 0b1000,
            CmpOp::Teq => 0b1001,
            CmpOp::Cmp => 0b1010,
            CmpOp::Cmn => 0b1011,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            CmpOp::Tst => "tst",
            CmpOp::Teq => "teq",
            CmpOp::Cmp => "cmp",
            CmpOp::Cmn => "cmn",
        }
    }
}

impl BitOp {
    /// The full instruction word for `op rd, rm`, with `rd`/`rm` zeroed.
    pub(crate) fn template(self) -> u32 {
        match self {
            BitOp::Clz => 0x016f_0f10,
            BitOp::Rev => 0x06bf_0f30,
            BitOp::Rev16 => 0x06bf_0fb0,
            BitOp::Rbit => 0x06ff_0f30,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            BitOp::Clz => "clz",
            BitOp::Rev => "rev",
            BitOp::Rev16 => "rev16",
            BitOp::Rbit => "rbit",
        }
    }
}

impl ExtOp {
    /// The full instruction word for `op rd, rm` (rotation 0), with `rd`/`rm`
    /// zeroed.
    pub(crate) fn template(self) -> u32 {
        match self {
            ExtOp::Sxtb => 0x06af_0070,
            ExtOp::Sxth => 0x06bf_0070,
            ExtOp::Uxtb => 0x06ef_0070,
            ExtOp::Uxth => 0x06ff_0070,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            ExtOp::Sxtb => "sxtb",
            ExtOp::Sxth => "sxth",
            ExtOp::Uxtb => "uxtb",
            ExtOp::Uxth => "uxth",
        }
    }
}

impl ShiftOp {
    /// The 2-bit shift-type field used in a shifted operand2.
    pub(crate) fn bits(self) -> u32 {
        match self {
            ShiftOp::Lsl => 0b00,
            ShiftOp::Lsr => 0b01,
            ShiftOp::Asr => 0b10,
            ShiftOp::Ror => 0b11,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            ShiftOp::Lsl => "lsl",
            ShiftOp::Lsr => "lsr",
            ShiftOp::Asr => "asr",
            ShiftOp::Ror => "ror",
        }
    }
}

impl Cond {
    /// The 4-bit condition-code field.
    pub(crate) fn bits(self) -> u32 {
        match self {
            Cond::Eq => 0b0000,
            Cond::Ne => 0b0001,
            Cond::Hs => 0b0010,
            Cond::Lo => 0b0011,
            Cond::Mi => 0b0100,
            Cond::Pl => 0b0101,
            Cond::Vs => 0b0110,
            Cond::Vc => 0b0111,
            Cond::Hi => 0b1000,
            Cond::Ls => 0b1001,
            Cond::Ge => 0b1010,
            Cond::Lt => 0b1011,
            Cond::Gt => 0b1100,
            Cond::Le => 0b1101,
        }
    }

    /// The condition that is true exactly when `self` is false.
    pub(crate) fn invert(self) -> Cond {
        match self {
            Cond::Eq => Cond::Ne,
            Cond::Ne => Cond::Eq,
            Cond::Hs => Cond::Lo,
            Cond::Lo => Cond::Hs,
            Cond::Mi => Cond::Pl,
            Cond::Pl => Cond::Mi,
            Cond::Vs => Cond::Vc,
            Cond::Vc => Cond::Vs,
            Cond::Hi => Cond::Ls,
            Cond::Ls => Cond::Hi,
            Cond::Ge => Cond::Lt,
            Cond::Lt => Cond::Ge,
            Cond::Gt => Cond::Le,
            Cond::Le => Cond::Gt,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Cond::Eq => "eq",
            Cond::Ne => "ne",
            Cond::Hs => "hs",
            Cond::Lo => "lo",
            Cond::Mi => "mi",
            Cond::Pl => "pl",
            Cond::Vs => "vs",
            Cond::Vc => "vc",
            Cond::Hi => "hi",
            Cond::Ls => "ls",
            Cond::Ge => "ge",
            Cond::Lt => "lt",
            Cond::Gt => "gt",
            Cond::Le => "le",
        }
    }
}

impl LoadKind {
    pub(crate) fn mnemonic(self) -> &'static str {
        match self {
            LoadKind::Word => "ldr",
            LoadKind::UByte => "ldrb",
            LoadKind::SByte => "ldrsb",
            LoadKind::UHalf => "ldrh",
            LoadKind::SHalf => "ldrsh",
        }
    }
}

impl StoreKind {
    pub(crate) fn mnemonic(self) -> &'static str {
        match self {
            StoreKind::Word => "str",
            StoreKind::Byte => "strb",
            StoreKind::Half => "strh",
        }
    }
}

/// Decode a 12-bit rotated data-processing immediate back to its value, for
/// pretty-printing.
pub fn decode_rotated_imm(enc: u32) -> u32 {
    let rot = (enc >> 8) & 0xf;
    let imm8 = enc & 0xff;
    imm8.rotate_right(2 * rot)
}

/// Encode `val` as an A32 data-processing immediate: an 8-bit value rotated
/// right by an even amount. Returns the 12-bit encoded operand (`rot << 8 |
/// imm8`) if representable.
pub fn encode_rotated_imm(val: u32) -> Option<u32> {
    if val <= 0xff {
        return Some(val);
    }
    for rot in 1..16u32 {
        // The immediate is `imm8` rotated right by `2 * rot`; to recover imm8 we
        // rotate `val` left by the same amount.
        let rotated = val.rotate_left(2 * rot);
        if rotated <= 0xff {
            return Some((rot << 8) | rotated);
        }
    }
    None
}
