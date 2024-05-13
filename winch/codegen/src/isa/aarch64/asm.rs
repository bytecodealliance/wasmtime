//! Assembler library implementation for Aarch64.

use super::{address::Address, regs};
use crate::masm::RoundingMode;
use crate::{masm::OperandSize, reg::Reg};
use cranelift_codegen::isa::aarch64::inst::FPUOpRI::{UShr32, UShr64};
use cranelift_codegen::isa::aarch64::inst::{
    FPULeftShiftImm, FPUOp1, FPUOp2, FPUOpRI, FPUOpRIMod, FPURightShiftImm, FpuRoundMode,
    ScalarSize,
};
use cranelift_codegen::{
    ir::{MemFlags, SourceLoc},
    isa::aarch64::inst::{
        self,
        emit::{EmitInfo, EmitState},
        ALUOp, ALUOp3, AMode, ExtendOp, Imm12, Inst, PairAMode,
    },
    settings, Final, MachBuffer, MachBufferFinalized, MachInstEmit, MachInstEmitState, MachLabel,
    Writable,
};

impl From<OperandSize> for inst::OperandSize {
    fn from(size: OperandSize) -> Self {
        match size {
            OperandSize::S32 => Self::Size32,
            OperandSize::S64 => Self::Size64,
            s => panic!("Invalid operand size {:?}", s),
        }
    }
}
impl Into<ScalarSize> for OperandSize {
    fn into(self) -> ScalarSize {
        match self {
            OperandSize::S8 => ScalarSize::Size8,
            OperandSize::S16 => ScalarSize::Size16,
            OperandSize::S32 => ScalarSize::Size32,
            OperandSize::S64 => ScalarSize::Size64,
            OperandSize::S128 => ScalarSize::Size128,
        }
    }
}

/// Low level assembler implementation for Aarch64.
pub(crate) struct Assembler {
    /// The machine instruction buffer.
    buffer: MachBuffer<Inst>,
    /// Constant emission information.
    emit_info: EmitInfo,
    /// Emission state.
    emit_state: EmitState,
}

impl Assembler {
    /// Create a new Aarch64 assembler.
    pub fn new(shared_flags: settings::Flags) -> Self {
        Self {
            buffer: MachBuffer::<Inst>::new(),
            emit_state: Default::default(),
            emit_info: EmitInfo::new(shared_flags),
        }
    }
}

impl Assembler {
    /// Return the emitted code.
    pub fn finalize(mut self, loc: Option<SourceLoc>) -> MachBufferFinalized<Final> {
        let constants = Default::default();
        let stencil = self
            .buffer
            .finish(&constants, self.emit_state.ctrl_plane_mut());
        stencil.apply_base_srcloc(loc.unwrap_or_default())
    }

    fn emit(&mut self, inst: Inst) {
        inst.emit(&mut self.buffer, &self.emit_info, &mut self.emit_state);
    }

    /// Load a constant into a register.
    pub fn load_constant(&mut self, imm: u64, rd: Reg) {
        let writable = Writable::from_reg(rd.into());
        Inst::load_constant(writable, imm, &mut |_| writable)
            .into_iter()
            .for_each(|i| self.emit(i));
    }

    /// Store a pair of registers.
    pub fn stp(&mut self, xt1: Reg, xt2: Reg, addr: Address) {
        let mem: PairAMode = addr.try_into().unwrap();
        self.emit(Inst::StoreP64 {
            rt: xt1.into(),
            rt2: xt2.into(),
            mem,
            flags: MemFlags::trusted(),
        });
    }

    /// Store a register.
    pub fn str(&mut self, reg: Reg, addr: Address, size: OperandSize) {
        let mem: AMode = addr.try_into().unwrap();
        let flags = MemFlags::trusted();

        use OperandSize::*;
        let inst = match size {
            S64 => Inst::Store64 {
                rd: reg.into(),
                mem,
                flags,
            },
            S32 => Inst::Store32 {
                rd: reg.into(),
                mem,
                flags,
            },

            _ => unreachable!(),
        };

        self.emit(inst);
    }

    /// Load a register.
    pub fn ldr(&mut self, addr: Address, rd: Reg, size: OperandSize) {
        use OperandSize::*;
        let writable_reg = Writable::from_reg(rd.into());
        let mem: AMode = addr.try_into().unwrap();
        let flags = MemFlags::trusted();

        let inst = match size {
            S64 => Inst::ULoad64 {
                rd: writable_reg,
                mem,
                flags,
            },
            S32 => Inst::ULoad32 {
                rd: writable_reg,
                mem,
                flags,
            },
            _ => unreachable!(),
        };

        self.emit(inst);
    }

    /// Load a pair of registers.
    pub fn ldp(&mut self, xt1: Reg, xt2: Reg, addr: Address) {
        let writable_xt1 = Writable::from_reg(xt1.into());
        let writable_xt2 = Writable::from_reg(xt2.into());
        let mem = addr.try_into().unwrap();

        self.emit(Inst::LoadP64 {
            rt: writable_xt1,
            rt2: writable_xt2,
            mem,
            flags: MemFlags::trusted(),
        });
    }

    /// Register to register move.
    pub fn mov_rr(&mut self, rm: Reg, rd: Reg, size: OperandSize) {
        let writable_rd = Writable::from_reg(rd.into());
        self.emit(Inst::Mov {
            size: size.into(),
            rd: writable_rd,
            rm: rm.into(),
        });
    }

    /// Floating point register to register move.
    pub fn fmov_rr(&mut self, rn: Reg, rd: Reg, size: OperandSize) {
        let writable = Writable::from_reg(rd.into());
        let inst = match size {
            OperandSize::S32 => Inst::FpuMove32 {
                rd: writable,
                rn: rn.into(),
            },
            OperandSize::S64 => Inst::FpuMove64 {
                rd: writable,
                rn: rn.into(),
            },
            _ => unreachable!(),
        };

        self.emit(inst);
    }

    pub fn mov_to_fpu(&mut self, rn: Reg, rd: Reg, size: OperandSize) {
        let writable_rd = Writable::from_reg(rd.into());
        self.emit(Inst::MovToFpu {
            size: size.into(),
            rd: writable_rd,
            rn: rn.into(),
        });
    }

    /// Add with three registers.
    pub fn add_rrr(&mut self, rm: Reg, rn: Reg, rd: Reg, size: OperandSize) {
        self.emit_alu_rrr_extend(ALUOp::Add, rm, rn, rd, size);
    }

    /// Add immediate and register.
    pub fn add_ir(&mut self, imm: u64, rn: Reg, rd: Reg, size: OperandSize) {
        let alu_op = ALUOp::Add;
        if let Some(imm) = Imm12::maybe_from_u64(imm) {
            self.emit_alu_rri(alu_op, imm, rn, rd, size);
        } else {
            let scratch = regs::scratch();
            self.load_constant(imm, scratch);
            self.emit_alu_rrr_extend(alu_op, scratch, rn, rd, size);
        }
    }

    /// Substract with three registers.
    pub fn sub_rrr(&mut self, rm: Reg, rn: Reg, rd: Reg, size: OperandSize) {
        self.emit_alu_rrr_extend(ALUOp::Sub, rm, rn, rd, size);
    }

    /// Subtract immediate and register.
    pub fn sub_ir(&mut self, imm: u64, rn: Reg, rd: Reg, size: OperandSize) {
        let alu_op = ALUOp::Sub;
        if let Some(imm) = Imm12::maybe_from_u64(imm) {
            self.emit_alu_rri(alu_op, imm, rn, rd, size);
        } else {
            let scratch = regs::scratch();
            self.load_constant(imm, scratch);
            self.emit_alu_rrr_extend(alu_op, scratch, rn, rd, size);
        }
    }

    /// Multiply with three registers.
    pub fn mul_rrr(&mut self, rm: Reg, rn: Reg, rd: Reg, size: OperandSize) {
        self.emit_alu_rrrr(ALUOp3::MAdd, rm, rn, rd, regs::zero(), size);
    }

    /// Multiply immediate and register.
    pub fn mul_ir(&mut self, imm: u64, rn: Reg, rd: Reg, size: OperandSize) {
        let scratch = regs::scratch();
        self.load_constant(imm, scratch);
        self.emit_alu_rrrr(ALUOp3::MAdd, scratch, rn, rd, regs::zero(), size);
    }

    /// Float add with three registers.
    pub fn fadd_rrr(&mut self, rm: Reg, rn: Reg, rd: Reg, size: OperandSize) {
        self.emit_fpu_rrr(FPUOp2::Add, rm, rn, rd, size);
    }

    /// Float sub with three registers.
    pub fn fsub_rrr(&mut self, rm: Reg, rn: Reg, rd: Reg, size: OperandSize) {
        self.emit_fpu_rrr(FPUOp2::Sub, rm, rn, rd, size);
    }

    /// Float multiply with three registers.
    pub fn fmul_rrr(&mut self, rm: Reg, rn: Reg, rd: Reg, size: OperandSize) {
        self.emit_fpu_rrr(FPUOp2::Mul, rm, rn, rd, size);
    }

    /// Float division with three registers.
    pub fn fdiv_rrr(&mut self, rm: Reg, rn: Reg, rd: Reg, size: OperandSize) {
        self.emit_fpu_rrr(FPUOp2::Div, rm, rn, rd, size);
    }

    /// Float max with three registers.
    pub fn fmax_rrr(&mut self, rm: Reg, rn: Reg, rd: Reg, size: OperandSize) {
        self.emit_fpu_rrr(FPUOp2::Max, rm, rn, rd, size);
    }

    /// Float min with three registers.
    pub fn fmin_rrr(&mut self, rm: Reg, rn: Reg, rd: Reg, size: OperandSize) {
        self.emit_fpu_rrr(FPUOp2::Min, rm, rn, rd, size);
    }

    /// Float neg with two registers.
    pub fn fneg_rr(&mut self, rn: Reg, rd: Reg, size: OperandSize) {
        self.emit_fpu_rr(FPUOp1::Neg, rn, rd, size);
    }

    /// Float abs with two registers.
    pub fn fabs_rr(&mut self, rn: Reg, rd: Reg, size: OperandSize) {
        self.emit_fpu_rr(FPUOp1::Abs, rn, rd, size);
    }

    /// Float sqrt with two registers.
    pub fn fsqrt_rr(&mut self, rn: Reg, rd: Reg, size: OperandSize) {
        self.emit_fpu_rr(FPUOp1::Sqrt, rn, rd, size);
    }

    /// Float round (ceil, trunc, floor) with two registers.
    pub fn fround_rr(&mut self, rn: Reg, rd: Reg, mode: RoundingMode, size: OperandSize) {
        let fpu_mode = match (mode, size) {
            (RoundingMode::Nearest, OperandSize::S32) => FpuRoundMode::Nearest32,
            (RoundingMode::Up, OperandSize::S32) => FpuRoundMode::Plus32,
            (RoundingMode::Down, OperandSize::S32) => FpuRoundMode::Minus32,
            (RoundingMode::Zero, OperandSize::S32) => FpuRoundMode::Zero32,
            (RoundingMode::Nearest, OperandSize::S64) => FpuRoundMode::Nearest64,
            (RoundingMode::Up, OperandSize::S64) => FpuRoundMode::Plus64,
            (RoundingMode::Down, OperandSize::S64) => FpuRoundMode::Minus64,
            (RoundingMode::Zero, OperandSize::S64) => FpuRoundMode::Zero64,
            (m, o) => panic!("Invalid rounding mode or operand size {:?}, {:?}", m, o),
        };
        self.emit_fpu_round(fpu_mode, rn, rd)
    }

    /// Float unsigned shift right with two registers and an immediate.
    pub fn fushr_rri(&mut self, rn: Reg, rd: Reg, amount: u8, size: OperandSize) {
        let imm = FPURightShiftImm {
            amount: amount,
            lane_size_in_bits: size.num_bits(),
        };
        let ushr = match size {
            OperandSize::S32 => UShr32(imm),
            OperandSize::S64 => UShr64(imm),
            _ => unreachable!(),
        };
        self.emit_fpu_rri(ushr, rn, rd)
    }

    /// Float unsigned shift left and insert with three registers
    /// and an immediate.
    pub fn fsli_rri_mod(&mut self, ri: Reg, rn: Reg, rd: Reg, amount: u8, size: OperandSize) {
        let imm = FPULeftShiftImm {
            amount: amount,
            lane_size_in_bits: size.num_bits(),
        };
        let sli = match size {
            OperandSize::S32 => FPUOpRIMod::Sli32(imm),
            OperandSize::S64 => FPUOpRIMod::Sli64(imm),
            _ => unreachable!(),
        };
        self.emit_fpu_rri_mod(sli, ri, rn, rd)
    }

    /// Return instruction.
    pub fn ret(&mut self) {
        self.emit(Inst::Ret {});
    }

    // Helpers for ALU operations.

    fn emit_alu_rri(&mut self, op: ALUOp, imm: Imm12, rn: Reg, rd: Reg, size: OperandSize) {
        self.emit(Inst::AluRRImm12 {
            alu_op: op,
            size: size.into(),
            rd: Writable::from_reg(rd.into()),
            rn: rn.into(),
            imm12: imm,
        });
    }

    fn emit_alu_rrr_extend(&mut self, op: ALUOp, rm: Reg, rn: Reg, rd: Reg, size: OperandSize) {
        self.emit(Inst::AluRRRExtend {
            alu_op: op,
            size: size.into(),
            rd: Writable::from_reg(rd.into()),
            rn: rn.into(),
            rm: rm.into(),
            extendop: ExtendOp::UXTX,
        });
    }

    fn emit_alu_rrrr(&mut self, op: ALUOp3, rm: Reg, rn: Reg, rd: Reg, ra: Reg, size: OperandSize) {
        self.emit(Inst::AluRRRR {
            alu_op: op,
            size: size.into(),
            rd: Writable::from_reg(rd.into()),
            rn: rn.into(),
            rm: rm.into(),
            ra: ra.into(),
        });
    }

    fn emit_fpu_rrr(&mut self, op: FPUOp2, rm: Reg, rn: Reg, rd: Reg, size: OperandSize) {
        self.emit(Inst::FpuRRR {
            fpu_op: op,
            size: size.into(),
            rd: Writable::from_reg(rd.into()),
            rn: rn.into(),
            rm: rm.into(),
        });
    }

    fn emit_fpu_rri(&mut self, op: FPUOpRI, rn: Reg, rd: Reg) {
        self.emit(Inst::FpuRRI {
            fpu_op: op,
            rd: Writable::from_reg(rd.into()),
            rn: rn.into(),
        });
    }

    fn emit_fpu_rri_mod(&mut self, op: FPUOpRIMod, ri: Reg, rn: Reg, rd: Reg) {
        self.emit(Inst::FpuRRIMod {
            fpu_op: op,
            rd: Writable::from_reg(rd.into()),
            ri: ri.into(),
            rn: rn.into(),
        });
    }

    fn emit_fpu_rr(&mut self, op: FPUOp1, rn: Reg, rd: Reg, size: OperandSize) {
        self.emit(Inst::FpuRR {
            fpu_op: op,
            size: size.into(),
            rd: Writable::from_reg(rd.into()),
            rn: rn.into(),
        });
    }

    fn emit_fpu_round(&mut self, op: FpuRoundMode, rn: Reg, rd: Reg) {
        self.emit(Inst::FpuRound {
            op: op,
            rd: Writable::from_reg(rd.into()),
            rn: rn.into(),
        });
    }

    /// Get a label from the underlying machine code buffer.
    pub fn get_label(&mut self) -> MachLabel {
        self.buffer.get_label()
    }

    /// Get a mutable reference to underlying
    /// machine buffer.
    pub fn buffer_mut(&mut self) -> &mut MachBuffer<Inst> {
        &mut self.buffer
    }

    /// Get a reference to the underlying machine buffer.
    pub fn buffer(&self) -> &MachBuffer<Inst> {
        &self.buffer
    }
}
