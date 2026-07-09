//! arm32 addressing modes and immediate/operand helpers.

use crate::isa::arm32::inst::*;
use crate::machinst::{OperandVisitor, Reg};

pub use crate::isa::arm32::lower::isle::generated_code::{
    ALUOp, AMode, AtomicSize, BarrierOp, BfxOp, BitOp, CmpOp, Cond, DspMul3Op, DspMul4Op,
    DspMulLOp, ExtAddOp, ExtOp, FpuOp2, FpuOp3, FpuSize, LoadKind, ParAluOp, PkhOp, QAluOp, SatOp,
    ShiftOp, StoreKind,
};

impl AtomicSize {
    /// The instruction-mnemonic suffix for this access width (`ldrex`,
    /// `ldrexb`, `ldrexh`, `ldrexd`).
    pub(crate) fn suffix(self) -> &'static str {
        match self {
            AtomicSize::Byte => "b",
            AtomicSize::Half => "h",
            AtomicSize::Word => "",
            AtomicSize::DWord => "d",
        }
    }

    /// The size field (bits 22:21) of an A32 exclusive/acquire-release
    /// instruction encoding.
    pub(crate) fn enc_bits(self) -> u32 {
        let sz = match self {
            AtomicSize::Word => 0b00,
            AtomicSize::DWord => 0b01,
            AtomicSize::Byte => 0b10,
            AtomicSize::Half => 0b11,
        };
        sz << 21
    }

    /// True for a sub-word (byte or halfword) access.
    pub(crate) fn is_subword(self) -> bool {
        matches!(self, AtomicSize::Byte | AtomicSize::Half)
    }

    /// The sign/zero-extend op that widens a sub-word value of this size to a
    /// full 32-bit register.
    pub(crate) fn extend_op(self, signed: bool) -> ExtOp {
        match (self, signed) {
            (AtomicSize::Byte, false) => ExtOp::Uxtb,
            (AtomicSize::Byte, true) => ExtOp::Sxtb,
            (AtomicSize::Half, false) => ExtOp::Uxth,
            (AtomicSize::Half, true) => ExtOp::Sxth,
            _ => unreachable!("extend_op is only valid for sub-word sizes"),
        }
    }
}

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
            BitOp::Revsh => 0x06ff_0fb0,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            BitOp::Clz => "clz",
            BitOp::Rev => "rev",
            BitOp::Rev16 => "rev16",
            BitOp::Rbit => "rbit",
            BitOp::Revsh => "revsh",
        }
    }
}

impl QAluOp {
    /// The instruction word for `op rd, rm, rn`, with the registers zeroed.
    pub(crate) fn template(self) -> u32 {
        match self {
            QAluOp::Qadd => 0x0100_0050,
            QAluOp::Qsub => 0x0120_0050,
            QAluOp::Qdadd => 0x0140_0050,
            QAluOp::Qdsub => 0x0160_0050,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            QAluOp::Qadd => "qadd",
            QAluOp::Qsub => "qsub",
            QAluOp::Qdadd => "qdadd",
            QAluOp::Qdsub => "qdsub",
        }
    }
}

impl SatOp {
    /// The instruction word for `op rd, #sat, rm` (no shift), registers zeroed.
    pub(crate) fn template(self) -> u32 {
        match self {
            SatOp::Ssat => 0x06a0_0010,
            SatOp::Usat => 0x06e0_0010,
            SatOp::Ssat16 => 0x06a0_0f30,
            SatOp::Usat16 => 0x06e0_0f30,
        }
    }

    /// Signed operations encode `sat_bits - 1`; unsigned encode `sat_bits`.
    pub(crate) fn is_signed(self) -> bool {
        matches!(self, SatOp::Ssat | SatOp::Ssat16)
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            SatOp::Ssat => "ssat",
            SatOp::Usat => "usat",
            SatOp::Ssat16 => "ssat16",
            SatOp::Usat16 => "usat16",
        }
    }
}

impl BfxOp {
    pub(crate) fn template(self) -> u32 {
        match self {
            BfxOp::Sbfx => 0x07a0_0050,
            BfxOp::Ubfx => 0x07e0_0050,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            BfxOp::Sbfx => "sbfx",
            BfxOp::Ubfx => "ubfx",
        }
    }
}

impl PkhOp {
    pub(crate) fn template(self) -> u32 {
        match self {
            PkhOp::Pkhbt => 0x0680_0010,
            PkhOp::Pkhtb => 0x0680_0050,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            PkhOp::Pkhbt => "pkhbt",
            PkhOp::Pkhtb => "pkhtb",
        }
    }
}

impl ParAluOp {
    /// The `(op1, op2)` fields of the parallel add/subtract encoding: `op1`
    /// (bits 22:20) selects the prefix (signedness/saturation) and `op2` (bits
    /// 7:5) selects the operation.
    fn parts(self) -> (u32, u32) {
        // Prefixes: S=1, Q=2, SH=3, U=5, UQ=6, UH=7.
        // Ops: ADD16=0, ASX=1, SAX=2, SUB16=3, ADD8=4, SUB8=7.
        match self {
            ParAluOp::Sadd16 => (1, 0),
            ParAluOp::Sasx => (1, 1),
            ParAluOp::Ssax => (1, 2),
            ParAluOp::Ssub16 => (1, 3),
            ParAluOp::Sadd8 => (1, 4),
            ParAluOp::Ssub8 => (1, 7),
            ParAluOp::Qadd16 => (2, 0),
            ParAluOp::Qasx => (2, 1),
            ParAluOp::Qsax => (2, 2),
            ParAluOp::Qsub16 => (2, 3),
            ParAluOp::Qadd8 => (2, 4),
            ParAluOp::Qsub8 => (2, 7),
            ParAluOp::Shadd16 => (3, 0),
            ParAluOp::Shasx => (3, 1),
            ParAluOp::Shsax => (3, 2),
            ParAluOp::Shsub16 => (3, 3),
            ParAluOp::Shadd8 => (3, 4),
            ParAluOp::Shsub8 => (3, 7),
            ParAluOp::Uadd16 => (5, 0),
            ParAluOp::Uasx => (5, 1),
            ParAluOp::Usax => (5, 2),
            ParAluOp::Usub16 => (5, 3),
            ParAluOp::Uadd8 => (5, 4),
            ParAluOp::Usub8 => (5, 7),
            ParAluOp::Uqadd16 => (6, 0),
            ParAluOp::Uqasx => (6, 1),
            ParAluOp::Uqsax => (6, 2),
            ParAluOp::Uqsub16 => (6, 3),
            ParAluOp::Uqadd8 => (6, 4),
            ParAluOp::Uqsub8 => (6, 7),
            ParAluOp::Uhadd16 => (7, 0),
            ParAluOp::Uhasx => (7, 1),
            ParAluOp::Uhsax => (7, 2),
            ParAluOp::Uhsub16 => (7, 3),
            ParAluOp::Uhadd8 => (7, 4),
            ParAluOp::Uhsub8 => (7, 7),
        }
    }

    /// The instruction word for `op rd, rn, rm`, with the registers zeroed.
    pub(crate) fn template(self) -> u32 {
        let (op1, op2) = self.parts();
        0x0600_0000 | (op1 << 20) | 0x0000_0f00 | (op2 << 5) | 0x10
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            ParAluOp::Sadd8 => "sadd8",
            ParAluOp::Sadd16 => "sadd16",
            ParAluOp::Ssub8 => "ssub8",
            ParAluOp::Ssub16 => "ssub16",
            ParAluOp::Sasx => "sasx",
            ParAluOp::Ssax => "ssax",
            ParAluOp::Qadd8 => "qadd8",
            ParAluOp::Qadd16 => "qadd16",
            ParAluOp::Qsub8 => "qsub8",
            ParAluOp::Qsub16 => "qsub16",
            ParAluOp::Qasx => "qasx",
            ParAluOp::Qsax => "qsax",
            ParAluOp::Shadd8 => "shadd8",
            ParAluOp::Shadd16 => "shadd16",
            ParAluOp::Shsub8 => "shsub8",
            ParAluOp::Shsub16 => "shsub16",
            ParAluOp::Shasx => "shasx",
            ParAluOp::Shsax => "shsax",
            ParAluOp::Uadd8 => "uadd8",
            ParAluOp::Uadd16 => "uadd16",
            ParAluOp::Usub8 => "usub8",
            ParAluOp::Usub16 => "usub16",
            ParAluOp::Uasx => "uasx",
            ParAluOp::Usax => "usax",
            ParAluOp::Uqadd8 => "uqadd8",
            ParAluOp::Uqadd16 => "uqadd16",
            ParAluOp::Uqsub8 => "uqsub8",
            ParAluOp::Uqsub16 => "uqsub16",
            ParAluOp::Uqasx => "uqasx",
            ParAluOp::Uqsax => "uqsax",
            ParAluOp::Uhadd8 => "uhadd8",
            ParAluOp::Uhadd16 => "uhadd16",
            ParAluOp::Uhsub8 => "uhsub8",
            ParAluOp::Uhsub16 => "uhsub16",
            ParAluOp::Uhasx => "uhasx",
            ParAluOp::Uhsax => "uhsax",
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
            ExtOp::Sxtb16 => 0x068f_0070,
            ExtOp::Uxtb16 => 0x06cf_0070,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            ExtOp::Sxtb => "sxtb",
            ExtOp::Sxth => "sxth",
            ExtOp::Uxtb => "uxtb",
            ExtOp::Uxth => "uxth",
            ExtOp::Sxtb16 => "sxtb16",
            ExtOp::Uxtb16 => "uxtb16",
        }
    }
}

impl ExtAddOp {
    /// The instruction word for `op rd, rn, rm` (rotation 0), registers zeroed.
    pub(crate) fn template(self) -> u32 {
        match self {
            ExtAddOp::Sxtab => 0x06a0_0070,
            ExtAddOp::Sxtah => 0x06b0_0070,
            ExtAddOp::Sxtab16 => 0x0680_0070,
            ExtAddOp::Uxtab => 0x06e0_0070,
            ExtAddOp::Uxtah => 0x06f0_0070,
            ExtAddOp::Uxtab16 => 0x06c0_0070,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            ExtAddOp::Sxtab => "sxtab",
            ExtAddOp::Sxtah => "sxtah",
            ExtAddOp::Sxtab16 => "sxtab16",
            ExtAddOp::Uxtab => "uxtab",
            ExtAddOp::Uxtah => "uxtah",
            ExtAddOp::Uxtab16 => "uxtab16",
        }
    }
}

impl DspMul3Op {
    /// The instruction word for `op rd, rn, rm`, with `rd`(19:16), `rm`(11:8),
    /// `rn`(3:0) zeroed. The half-select (M/N) and pre-select bits are baked in.
    pub(crate) fn template(self) -> u32 {
        match self {
            DspMul3Op::Smulbb => 0x0160_0080,
            DspMul3Op::Smulbt => 0x0160_00c0,
            DspMul3Op::Smultb => 0x0160_00a0,
            DspMul3Op::Smultt => 0x0160_00e0,
            DspMul3Op::Smulwb => 0x0120_00a0,
            DspMul3Op::Smulwt => 0x0120_00e0,
            DspMul3Op::Smmul => 0x0750_f010,
            DspMul3Op::Smuad => 0x0700_f010,
            DspMul3Op::Smusd => 0x0700_f050,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            DspMul3Op::Smulbb => "smulbb",
            DspMul3Op::Smulbt => "smulbt",
            DspMul3Op::Smultb => "smultb",
            DspMul3Op::Smultt => "smultt",
            DspMul3Op::Smulwb => "smulwb",
            DspMul3Op::Smulwt => "smulwt",
            DspMul3Op::Smmul => "smmul",
            DspMul3Op::Smuad => "smuad",
            DspMul3Op::Smusd => "smusd",
        }
    }
}

impl DspMul4Op {
    /// The instruction word for `op rd, rn, rm, ra`, with `rd`(19:16),
    /// `ra`(15:12), `rm`(11:8), `rn`(3:0) zeroed.
    pub(crate) fn template(self) -> u32 {
        match self {
            DspMul4Op::Smlabb => 0x0100_0080,
            DspMul4Op::Smlabt => 0x0100_00c0,
            DspMul4Op::Smlatb => 0x0100_00a0,
            DspMul4Op::Smlatt => 0x0100_00e0,
            DspMul4Op::Smlawb => 0x0120_0080,
            DspMul4Op::Smlawt => 0x0120_00c0,
            DspMul4Op::Smmla => 0x0750_0010,
            DspMul4Op::Smmls => 0x0750_00d0,
            DspMul4Op::Smlad => 0x0700_0010,
            DspMul4Op::Smlsd => 0x0700_0050,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            DspMul4Op::Smlabb => "smlabb",
            DspMul4Op::Smlabt => "smlabt",
            DspMul4Op::Smlatb => "smlatb",
            DspMul4Op::Smlatt => "smlatt",
            DspMul4Op::Smlawb => "smlawb",
            DspMul4Op::Smlawt => "smlawt",
            DspMul4Op::Smmla => "smmla",
            DspMul4Op::Smmls => "smmls",
            DspMul4Op::Smlad => "smlad",
            DspMul4Op::Smlsd => "smlsd",
        }
    }
}

impl DspMulLOp {
    /// The instruction word for `op rd_lo, rd_hi, rn, rm`, with `rd_hi`(19:16),
    /// `rd_lo`(15:12), `rm`(11:8), `rn`(3:0) zeroed.
    pub(crate) fn template(self) -> u32 {
        match self {
            DspMulLOp::Smlalbb => 0x0140_0080,
            DspMulLOp::Smlalbt => 0x0140_00c0,
            DspMulLOp::Smlaltb => 0x0140_00a0,
            DspMulLOp::Smlaltt => 0x0140_00e0,
            DspMulLOp::Smlald => 0x0740_0010,
            DspMulLOp::Smlsld => 0x0740_0050,
            DspMulLOp::Umaal => 0x0040_0090,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            DspMulLOp::Smlalbb => "smlalbb",
            DspMulLOp::Smlalbt => "smlalbt",
            DspMulLOp::Smlaltb => "smlaltb",
            DspMulLOp::Smlaltt => "smlaltt",
            DspMulLOp::Smlald => "smlald",
            DspMulLOp::Smlsld => "smlsld",
            DspMulLOp::Umaal => "umaal",
        }
    }
}

impl BarrierOp {
    /// The full (unconditional) instruction word, with the system option (SY).
    pub(crate) fn encoding(self) -> u32 {
        match self {
            BarrierOp::Dmb => 0xf57f_f05f,
            BarrierOp::Dsb => 0xf57f_f04f,
            BarrierOp::Isb => 0xf57f_f06f,
            BarrierOp::Clrex => 0xf57f_f01f,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            BarrierOp::Dmb => "dmb sy",
            BarrierOp::Dsb => "dsb sy",
            BarrierOp::Isb => "isb sy",
            BarrierOp::Clrex => "clrex",
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

impl FpuSize {
    /// The size bit (bit 8) that distinguishes F64 (set) from F32 (clear).
    pub(crate) fn size_bit(self) -> u32 {
        match self {
            FpuSize::F32 => 0,
            FpuSize::F64 => 0x100,
        }
    }

    pub(crate) fn suffix(self) -> &'static str {
        match self {
            FpuSize::F32 => "f32",
            FpuSize::F64 => "f64",
        }
    }
}

impl FpuOp3 {
    /// The F32 base word (`vadd.f32 s0, s0, s0`); F64 adds the size bit.
    pub(crate) fn base(self) -> u32 {
        match self {
            FpuOp3::Vadd => 0xee30_0a00,
            FpuOp3::Vsub => 0xee30_0a40,
            FpuOp3::Vmul => 0xee20_0a00,
            FpuOp3::Vdiv => 0xee80_0a00,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            FpuOp3::Vadd => "vadd",
            FpuOp3::Vsub => "vsub",
            FpuOp3::Vmul => "vmul",
            FpuOp3::Vdiv => "vdiv",
        }
    }
}

impl FpuOp2 {
    /// The F32 base word; F64 adds the size bit.
    pub(crate) fn base(self) -> u32 {
        match self {
            FpuOp2::Vmov => 0xeeb0_0a40,
            FpuOp2::Vneg => 0xeeb1_0a40,
            FpuOp2::Vabs => 0xeeb0_0ac0,
            FpuOp2::Vsqrt => 0xeeb1_0ac0,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            FpuOp2::Vmov => "vmov",
            FpuOp2::Vneg => "vneg",
            FpuOp2::Vabs => "vabs",
            FpuOp2::Vsqrt => "vsqrt",
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
