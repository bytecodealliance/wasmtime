//! Assembler library implementation for Aarch64.
use super::{address::Address, regs};
use crate::aarch64::regs::zero;
use crate::masm::{
    DivKind, Extend, ExtendKind, FloatCmpKind, IntCmpKind, RemKind, RoundingMode, ShiftKind,
    Signed, TruncKind,
};
use crate::CallingConvention;
use crate::{
    masm::OperandSize,
    reg::{writable, Reg, WritableReg},
};

use cranelift_codegen::isa::aarch64::inst::{ASIMDFPModImm, FpuToIntOp, UImm5, NZCV};
use cranelift_codegen::{
    ir::{ExternalName, LibCall, MemFlags, SourceLoc, TrapCode, UserExternalNameRef},
    isa::aarch64::inst::{
        self,
        emit::{EmitInfo, EmitState},
        ALUOp, ALUOp3, AMode, BitOp, BranchTarget, Cond, CondBrKind, ExtendOp, FPULeftShiftImm,
        FPUOp1, FPUOp2,
        FPUOpRI::{self, UShr32, UShr64},
        FPUOpRIMod, FPURightShiftImm, FpuRoundMode, Imm12, ImmLogic, ImmShift, Inst, IntToFpuOp,
        PairAMode, ScalarSize, VecLanesOp, VecMisc2, VectorSize,
    },
    settings, Final, MachBuffer, MachBufferFinalized, MachInst, MachInstEmit, MachInstEmitState,
    MachLabel, Writable,
};

impl From<OperandSize> for inst::OperandSize {
    fn from(size: OperandSize) -> Self {
        match size {
            OperandSize::S32 => Self::Size32,
            OperandSize::S64 => Self::Size64,
            s => panic!("Invalid operand size {s:?}"),
        }
    }
}

impl From<IntCmpKind> for Cond {
    fn from(value: IntCmpKind) -> Self {
        match value {
            IntCmpKind::Eq => Cond::Eq,
            IntCmpKind::Ne => Cond::Ne,
            IntCmpKind::LtS => Cond::Lt,
            IntCmpKind::LtU => Cond::Lo,
            IntCmpKind::GtS => Cond::Gt,
            IntCmpKind::GtU => Cond::Hi,
            IntCmpKind::LeS => Cond::Le,
            IntCmpKind::LeU => Cond::Ls,
            IntCmpKind::GeS => Cond::Ge,
            IntCmpKind::GeU => Cond::Hs,
        }
    }
}

impl From<FloatCmpKind> for Cond {
    fn from(value: FloatCmpKind) -> Self {
        match value {
            FloatCmpKind::Eq => Cond::Eq,
            FloatCmpKind::Ne => Cond::Ne,
            FloatCmpKind::Lt => Cond::Mi,
            FloatCmpKind::Gt => Cond::Gt,
            FloatCmpKind::Le => Cond::Ls,
            FloatCmpKind::Ge => Cond::Ge,
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
        self.emit_with_island(inst, Inst::worst_case_size());
    }

    fn emit_with_island(&mut self, inst: Inst, needed_space: u32) {
        if self.buffer.island_needed(needed_space) {
            let label = self.buffer.get_label();
            let jmp = Inst::Jump {
                dest: BranchTarget::Label(label),
            };
            jmp.emit(&mut self.buffer, &self.emit_info, &mut self.emit_state);
            self.buffer
                .emit_island(needed_space, self.emit_state.ctrl_plane_mut());
            self.buffer
                .bind_label(label, self.emit_state.ctrl_plane_mut());
        }
        inst.emit(&mut self.buffer, &self.emit_info, &mut self.emit_state);
    }

    /// Load a constant into a register.
    pub fn load_constant(&mut self, imm: u64, rd: WritableReg) {
        let writable = rd.map(Into::into);
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
        let inst = match (reg.is_int(), size) {
            (_, S8) => Inst::Store8 {
                rd: reg.into(),
                mem,
                flags,
            },
            (_, S16) => Inst::Store16 {
                rd: reg.into(),
                mem,
                flags,
            },
            (true, S32) => Inst::Store32 {
                rd: reg.into(),
                mem,
                flags,
            },
            (false, S32) => Inst::FpuStore32 {
                rd: reg.into(),
                mem,
                flags,
            },
            (true, S64) => Inst::Store64 {
                rd: reg.into(),
                mem,
                flags,
            },
            (false, S64) => Inst::FpuStore64 {
                rd: reg.into(),
                mem,
                flags,
            },
            (_, S128) => Inst::FpuStore128 {
                rd: reg.into(),
                mem,
                flags,
            },
        };

        self.emit(inst);
    }

    /// Load a signed register.
    pub fn sload(&mut self, addr: Address, rd: WritableReg, size: OperandSize, flags: MemFlags) {
        self.ldr(addr, rd, size, true, flags);
    }

    /// Load an unsigned register.
    pub fn uload(&mut self, addr: Address, rd: WritableReg, size: OperandSize, flags: MemFlags) {
        self.ldr(addr, rd, size, false, flags);
    }

    /// Load address into a register.
    fn ldr(
        &mut self,
        addr: Address,
        rd: WritableReg,
        size: OperandSize,
        signed: bool,
        flags: MemFlags,
    ) {
        use OperandSize::*;
        let writable_reg = rd.map(Into::into);
        let mem: AMode = addr.try_into().unwrap();

        let inst = match (rd.to_reg().is_int(), signed, size) {
            (_, false, S8) => Inst::ULoad8 {
                rd: writable_reg,
                mem,
                flags,
            },
            (_, true, S8) => Inst::SLoad8 {
                rd: writable_reg,
                mem,
                flags,
            },
            (_, false, S16) => Inst::ULoad16 {
                rd: writable_reg,
                mem,
                flags,
            },
            (_, true, S16) => Inst::SLoad16 {
                rd: writable_reg,
                mem,
                flags,
            },
            (true, false, S32) => Inst::ULoad32 {
                rd: writable_reg,
                mem,
                flags,
            },
            (false, _, S32) => Inst::FpuLoad32 {
                rd: writable_reg,
                mem,
                flags,
            },
            (true, true, S32) => Inst::SLoad32 {
                rd: writable_reg,
                mem,
                flags,
            },
            (true, _, S64) => Inst::ULoad64 {
                rd: writable_reg,
                mem,
                flags,
            },
            (false, _, S64) => Inst::FpuLoad64 {
                rd: writable_reg,
                mem,
                flags,
            },
            (_, _, S128) => Inst::FpuLoad128 {
                rd: writable_reg,
                mem,
                flags,
            },
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
    pub fn mov_rr(&mut self, rm: Reg, rd: WritableReg, size: OperandSize) {
        let writable_rd = rd.map(Into::into);
        self.emit(Inst::Mov {
            size: size.into(),
            rd: writable_rd,
            rm: rm.into(),
        });
    }

    /// Floating point register to register move.
    pub fn fmov_rr(&mut self, rn: Reg, rd: WritableReg, size: OperandSize) {
        let writable = rd.map(Into::into);
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

    pub fn mov_to_fpu(&mut self, rn: Reg, rd: WritableReg, size: OperandSize) {
        let writable_rd = rd.map(Into::into);
        self.emit(Inst::MovToFpu {
            size: size.into(),
            rd: writable_rd,
            rn: rn.into(),
        });
    }

    pub fn mov_from_vec(&mut self, rn: Reg, rd: WritableReg, idx: u8, size: OperandSize) {
        self.emit(Inst::MovFromVec {
            rd: rd.map(Into::into),
            rn: rn.into(),
            idx,
            size: size.into(),
        });
    }

    /// Add with three registers.
    pub fn add_rrr(&mut self, rm: Reg, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.alu_rrr_extend(ALUOp::Add, rm, rn, rd, size);
    }

    /// Add immediate and register.
    pub fn add_ir(&mut self, imm: u64, rn: Reg, rd: WritableReg, size: OperandSize) {
        let alu_op = ALUOp::Add;
        if let Some(imm) = Imm12::maybe_from_u64(imm) {
            self.alu_rri(alu_op, imm, rn, rd, size);
        } else {
            let scratch = regs::scratch();
            self.load_constant(imm, writable!(scratch));
            self.alu_rrr_extend(alu_op, scratch, rn, rd, size);
        }
    }

    /// Add across Vector.
    pub fn addv(&mut self, rn: Reg, rd: WritableReg, size: VectorSize) {
        self.emit(Inst::VecLanes {
            op: VecLanesOp::Addv,
            rd: rd.map(Into::into),
            rn: rn.into(),
            size,
        });
    }

    /// Subtract with three registers.
    pub fn sub_rrr(&mut self, rm: Reg, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.alu_rrr_extend(ALUOp::Sub, rm, rn, rd, size);
    }

    /// Subtract immediate and register.
    pub fn sub_ir(&mut self, imm: u64, rn: Reg, rd: WritableReg, size: OperandSize) {
        let alu_op = ALUOp::Sub;
        if let Some(imm) = Imm12::maybe_from_u64(imm) {
            self.alu_rri(alu_op, imm, rn, rd, size);
        } else {
            let scratch = regs::scratch();
            self.load_constant(imm, writable!(scratch));
            self.alu_rrr_extend(alu_op, scratch, rn, rd, size);
        }
    }

    /// Subtract with three registers, setting flags.
    pub fn subs_rrr(&mut self, rm: Reg, rn: Reg, size: OperandSize) {
        self.alu_rrr_extend(ALUOp::SubS, rm, rn, writable!(regs::zero()), size);
    }

    /// Subtract immediate and register, setting flags.
    pub fn subs_ir(&mut self, imm: u64, rn: Reg, size: OperandSize) {
        let alu_op = ALUOp::SubS;
        if let Some(imm) = Imm12::maybe_from_u64(imm) {
            self.alu_rri(alu_op, imm, rn, writable!(regs::zero()), size);
        } else {
            let scratch = regs::scratch();
            self.load_constant(imm, writable!(scratch));
            self.alu_rrr_extend(alu_op, scratch, rn, writable!(regs::zero()), size);
        }
    }

    /// Multiply with three registers.
    pub fn mul_rrr(&mut self, rm: Reg, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.alu_rrrr(ALUOp3::MAdd, rm, rn, rd, regs::zero(), size);
    }

    /// Multiply immediate and register.
    pub fn mul_ir(&mut self, imm: u64, rn: Reg, rd: WritableReg, size: OperandSize) {
        let scratch = regs::scratch();
        self.load_constant(imm, writable!(scratch));
        self.alu_rrrr(ALUOp3::MAdd, scratch, rn, rd, regs::zero(), size);
    }

    /// Signed/unsigned division with three registers.
    pub fn div_rrr(
        &mut self,
        divisor: Reg,
        dividend: Reg,
        dest: Writable<Reg>,
        kind: DivKind,
        size: OperandSize,
    ) {
        // Check for division by 0.
        self.trapz(divisor, TrapCode::INTEGER_DIVISION_BY_ZERO, size);

        // check for overflow
        if kind == DivKind::Signed {
            // Check for divisor overflow.
            self.alu_rri(
                ALUOp::AddS,
                Imm12::maybe_from_u64(1).expect("1 to fit in 12 bits"),
                divisor,
                writable!(zero()),
                size,
            );

            // Check if the dividend is 1.
            self.emit(Inst::CCmpImm {
                size: size.into(),
                rn: dividend.into(),
                imm: UImm5::maybe_from_u8(1).expect("1 fits in 5 bits"),
                nzcv: NZCV::new(false, false, false, false),
                cond: Cond::Eq,
            });

            // Finally, trap if the previous operation overflowed.
            self.trapif(Cond::Vs, TrapCode::INTEGER_OVERFLOW);
        }

        // `cranelift-codegen` doesn't support emitting sdiv for anything but I64,
        // we therefore sign-extend the operand.
        // see: https://github.com/bytecodealliance/wasmtime/issues/9766
        let size = if size == OperandSize::S32 && kind == DivKind::Signed {
            self.extend(
                divisor,
                writable!(divisor),
                ExtendKind::Signed(Extend::<Signed>::I64Extend32),
            );
            self.extend(
                dividend,
                writable!(dividend),
                ExtendKind::Signed(Extend::<Signed>::I64Extend32),
            );
            OperandSize::S64
        } else {
            size
        };

        let op = match kind {
            DivKind::Signed => ALUOp::SDiv,
            DivKind::Unsigned => ALUOp::UDiv,
        };

        self.alu_rrr(op, divisor, dividend, dest.map(Into::into), size);
    }

    /// Signed/unsigned remainder operation with three registers.
    pub fn rem_rrr(
        &mut self,
        divisor: Reg,
        dividend: Reg,
        dest: Writable<Reg>,
        kind: RemKind,
        size: OperandSize,
    ) {
        // Check for division by 0
        self.trapz(divisor, TrapCode::INTEGER_DIVISION_BY_ZERO, size);

        // `cranelift-codegen` doesn't support emitting sdiv for anything but I64,
        // we therefore sign-extend the operand.
        // see: https://github.com/bytecodealliance/wasmtime/issues/9766
        let size = if size == OperandSize::S32 && kind.is_signed() {
            self.extend(
                divisor,
                writable!(divisor),
                ExtendKind::Signed(Extend::<Signed>::I64Extend32),
            );
            self.extend(
                dividend,
                writable!(dividend),
                ExtendKind::Signed(Extend::<Signed>::I64Extend32),
            );
            OperandSize::S64
        } else {
            size
        };

        let op = match kind {
            RemKind::Signed => ALUOp::SDiv,
            RemKind::Unsigned => ALUOp::UDiv,
        };

        let scratch = regs::scratch();
        self.alu_rrr(op, divisor, dividend, writable!(scratch.into()), size);

        self.alu_rrrr(
            ALUOp3::MSub,
            scratch,
            divisor,
            dest.map(Into::into),
            dividend,
            size,
        );
    }

    /// And with three registers.
    pub fn and_rrr(&mut self, rm: Reg, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.alu_rrr(ALUOp::And, rm, rn, rd, size);
    }

    /// And immediate and register.
    pub fn and_ir(&mut self, imm: u64, rn: Reg, rd: WritableReg, size: OperandSize) {
        let alu_op = ALUOp::And;
        let cl_size: inst::OperandSize = size.into();
        if let Some(imm) = ImmLogic::maybe_from_u64(imm, cl_size.to_ty()) {
            self.alu_rri_logic(alu_op, imm, rn, rd, size);
        } else {
            let scratch = regs::scratch();
            self.load_constant(imm, writable!(scratch));
            self.alu_rrr(alu_op, scratch, rn, rd, size);
        }
    }

    /// Or with three registers.
    pub fn or_rrr(&mut self, rm: Reg, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.alu_rrr(ALUOp::Orr, rm, rn, rd, size);
    }

    /// Or immediate and register.
    pub fn or_ir(&mut self, imm: u64, rn: Reg, rd: WritableReg, size: OperandSize) {
        let alu_op = ALUOp::Orr;
        let cl_size: inst::OperandSize = size.into();
        if let Some(imm) = ImmLogic::maybe_from_u64(imm, cl_size.to_ty()) {
            self.alu_rri_logic(alu_op, imm, rn, rd, size);
        } else {
            let scratch = regs::scratch();
            self.load_constant(imm, writable!(scratch));
            self.alu_rrr(alu_op, scratch, rn, rd, size);
        }
    }

    /// Xor with three registers.
    pub fn xor_rrr(&mut self, rm: Reg, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.alu_rrr(ALUOp::Eor, rm, rn, rd, size);
    }

    /// Xor immediate and register.
    pub fn xor_ir(&mut self, imm: u64, rn: Reg, rd: WritableReg, size: OperandSize) {
        let alu_op = ALUOp::Eor;
        let cl_size: inst::OperandSize = size.into();
        if let Some(imm) = ImmLogic::maybe_from_u64(imm, cl_size.to_ty()) {
            self.alu_rri_logic(alu_op, imm, rn, rd, size);
        } else {
            let scratch = regs::scratch();
            self.load_constant(imm, writable!(scratch));
            self.alu_rrr(alu_op, scratch, rn, rd, size);
        }
    }

    /// Shift with three registers.
    pub fn shift_rrr(
        &mut self,
        rm: Reg,
        rn: Reg,
        rd: WritableReg,
        kind: ShiftKind,
        size: OperandSize,
    ) {
        let shift_op = self.shift_kind_to_alu_op(kind, rm, size);
        self.alu_rrr(shift_op, rm, rn, rd, size);
    }

    /// Shift immediate and register.
    pub fn shift_ir(
        &mut self,
        imm: u64,
        rn: Reg,
        rd: WritableReg,
        kind: ShiftKind,
        size: OperandSize,
    ) {
        let shift_op = self.shift_kind_to_alu_op(kind, rn, size);

        if let Some(imm) = ImmShift::maybe_from_u64(imm) {
            self.alu_rri_shift(shift_op, imm, rn, rd, size);
        } else {
            let scratch = regs::scratch();
            self.load_constant(imm, writable!(scratch));
            self.alu_rrr(shift_op, scratch, rn, rd, size);
        }
    }

    /// Count Leading Zeros.
    pub fn clz(&mut self, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.bit_rr(BitOp::Clz, rn, rd, size);
    }

    /// Reverse Bits reverses the bit order in a register.
    pub fn rbit(&mut self, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.bit_rr(BitOp::RBit, rn, rd, size);
    }

    /// Float add with three registers.
    pub fn fadd_rrr(&mut self, rm: Reg, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.fpu_rrr(FPUOp2::Add, rm, rn, rd, size);
    }

    /// Float sub with three registers.
    pub fn fsub_rrr(&mut self, rm: Reg, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.fpu_rrr(FPUOp2::Sub, rm, rn, rd, size);
    }

    /// Float multiply with three registers.
    pub fn fmul_rrr(&mut self, rm: Reg, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.fpu_rrr(FPUOp2::Mul, rm, rn, rd, size);
    }

    /// Float division with three registers.
    pub fn fdiv_rrr(&mut self, rm: Reg, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.fpu_rrr(FPUOp2::Div, rm, rn, rd, size);
    }

    /// Float max with three registers.
    pub fn fmax_rrr(&mut self, rm: Reg, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.fpu_rrr(FPUOp2::Max, rm, rn, rd, size);
    }

    /// Float min with three registers.
    pub fn fmin_rrr(&mut self, rm: Reg, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.fpu_rrr(FPUOp2::Min, rm, rn, rd, size);
    }

    /// Float neg with two registers.
    pub fn fneg_rr(&mut self, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.fpu_rr(FPUOp1::Neg, rn, rd, size);
    }

    /// Float abs with two registers.
    pub fn fabs_rr(&mut self, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.fpu_rr(FPUOp1::Abs, rn, rd, size);
    }

    /// Float sqrt with two registers.
    pub fn fsqrt_rr(&mut self, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.fpu_rr(FPUOp1::Sqrt, rn, rd, size);
    }

    /// Float round (ceil, trunc, floor) with two registers.
    pub fn fround_rr(&mut self, rn: Reg, rd: WritableReg, mode: RoundingMode, size: OperandSize) {
        let fpu_mode = match (mode, size) {
            (RoundingMode::Nearest, OperandSize::S32) => FpuRoundMode::Nearest32,
            (RoundingMode::Up, OperandSize::S32) => FpuRoundMode::Plus32,
            (RoundingMode::Down, OperandSize::S32) => FpuRoundMode::Minus32,
            (RoundingMode::Zero, OperandSize::S32) => FpuRoundMode::Zero32,
            (RoundingMode::Nearest, OperandSize::S64) => FpuRoundMode::Nearest64,
            (RoundingMode::Up, OperandSize::S64) => FpuRoundMode::Plus64,
            (RoundingMode::Down, OperandSize::S64) => FpuRoundMode::Minus64,
            (RoundingMode::Zero, OperandSize::S64) => FpuRoundMode::Zero64,
            (m, o) => panic!("Invalid rounding mode or operand size {m:?}, {o:?}"),
        };
        self.fpu_round(fpu_mode, rn, rd)
    }

    /// Float unsigned shift right with two registers and an immediate.
    pub fn fushr_rri(&mut self, rn: Reg, rd: WritableReg, amount: u8, size: OperandSize) {
        let imm = FPURightShiftImm {
            amount,
            lane_size_in_bits: size.num_bits(),
        };
        let ushr = match size {
            OperandSize::S32 => UShr32(imm),
            OperandSize::S64 => UShr64(imm),
            _ => unreachable!(),
        };
        self.fpu_rri(ushr, rn, rd)
    }

    /// Float unsigned shift left and insert with three registers
    /// and an immediate.
    pub fn fsli_rri_mod(
        &mut self,
        ri: Reg,
        rn: Reg,
        rd: WritableReg,
        amount: u8,
        size: OperandSize,
    ) {
        let imm = FPULeftShiftImm {
            amount,
            lane_size_in_bits: size.num_bits(),
        };
        let sli = match size {
            OperandSize::S32 => FPUOpRIMod::Sli32(imm),
            OperandSize::S64 => FPUOpRIMod::Sli64(imm),
            _ => unreachable!(),
        };
        self.fpu_rri_mod(sli, ri, rn, rd)
    }

    /// Float compare.
    pub fn fcmp(&mut self, rm: Reg, rn: Reg, size: OperandSize) {
        self.emit(Inst::FpuCmp {
            size: size.into(),
            rn: rn.into(),
            rm: rm.into(),
        })
    }

    /// Convert an signed integer to a float.
    pub fn cvt_sint_to_float(
        &mut self,
        rn: Reg,
        rd: WritableReg,
        src_size: OperandSize,
        dst_size: OperandSize,
    ) {
        let op = match (src_size, dst_size) {
            (OperandSize::S32, OperandSize::S32) => IntToFpuOp::I32ToF32,
            (OperandSize::S64, OperandSize::S32) => IntToFpuOp::I64ToF32,
            (OperandSize::S32, OperandSize::S64) => IntToFpuOp::I32ToF64,
            (OperandSize::S64, OperandSize::S64) => IntToFpuOp::I64ToF64,
            _ => unreachable!(),
        };

        self.emit(Inst::IntToFpu {
            op,
            rd: rd.map(Into::into),
            rn: rn.into(),
        });
    }

    /// Convert an unsigned integer to a float.
    pub fn cvt_uint_to_float(
        &mut self,
        rn: Reg,
        rd: WritableReg,
        src_size: OperandSize,
        dst_size: OperandSize,
    ) {
        let op = match (src_size, dst_size) {
            (OperandSize::S32, OperandSize::S32) => IntToFpuOp::U32ToF32,
            (OperandSize::S64, OperandSize::S32) => IntToFpuOp::U64ToF32,
            (OperandSize::S32, OperandSize::S64) => IntToFpuOp::U32ToF64,
            (OperandSize::S64, OperandSize::S64) => IntToFpuOp::U64ToF64,
            _ => unreachable!(),
        };

        self.emit(Inst::IntToFpu {
            op,
            rd: rd.map(Into::into),
            rn: rn.into(),
        });
    }

    /// Change precision of float.
    pub fn cvt_float_to_float(
        &mut self,
        rn: Reg,
        rd: WritableReg,
        src_size: OperandSize,
        dst_size: OperandSize,
    ) {
        let (fpu_op, size) = match (src_size, dst_size) {
            (OperandSize::S32, OperandSize::S64) => (FPUOp1::Cvt32To64, ScalarSize::Size32),
            (OperandSize::S64, OperandSize::S32) => (FPUOp1::Cvt64To32, ScalarSize::Size64),
            _ => unimplemented!(),
        };
        self.emit(Inst::FpuRR {
            fpu_op,
            size,
            rd: rd.map(Into::into),
            rn: rn.into(),
        });
    }

    /// Return instruction.
    pub fn ret(&mut self) {
        self.emit(Inst::Ret {});
    }

    /// An unconditional branch.
    pub fn jmp(&mut self, target: MachLabel) {
        self.emit(Inst::Jump {
            dest: BranchTarget::Label(target),
        });
    }

    /// A conditional branch.
    pub fn jmp_if(&mut self, kind: Cond, taken: MachLabel) {
        self.emit(Inst::CondBr {
            taken: BranchTarget::Label(taken),
            not_taken: BranchTarget::ResolvedOffset(4),
            kind: CondBrKind::Cond(kind),
        });
    }

    /// Emits a jump table sequence.
    pub fn jmp_table(
        &mut self,
        targets: &[MachLabel],
        default: MachLabel,
        index: Reg,
        tmp1: Reg,
        tmp2: Reg,
    ) {
        self.emit_with_island(
            Inst::JTSequence {
                default,
                targets: Box::new(targets.to_vec()),
                ridx: index.into(),
                rtmp1: Writable::from_reg(tmp1.into()),
                rtmp2: Writable::from_reg(tmp2.into()),
            },
            // number of bytes needed for the jumptable sequence:
            // 4 bytes per instruction, with 8 instructions base + the size of
            // the jumptable more.
            (4 * (8 + targets.len())).try_into().unwrap(),
        );
    }

    /// Conditional Set sets the destination register to 1 if the condition
    /// is true, and otherwise sets it to 0.
    pub fn cset(&mut self, rd: WritableReg, cond: Cond) {
        self.emit(Inst::CSet {
            rd: rd.map(Into::into),
            cond,
        });
    }

    // If the condition is true, Conditional Select writes rm to rd. If the condition is false,
    // it writes rn to rd
    pub fn csel(&mut self, rm: Reg, rn: Reg, rd: WritableReg, cond: Cond) {
        self.emit(Inst::CSel {
            rd: rd.map(Into::into),
            rm: rm.into(),
            rn: rn.into(),
            cond,
        });
    }

    // Population Count per byte.
    pub fn cnt(&mut self, rd: WritableReg) {
        self.emit(Inst::VecMisc {
            op: VecMisc2::Cnt,
            rd: rd.map(Into::into),
            rn: rd.to_reg().into(),
            size: VectorSize::Size8x8,
        });
    }

    pub fn extend(&mut self, rn: Reg, rd: WritableReg, kind: ExtendKind) {
        self.emit(Inst::Extend {
            rd: rd.map(Into::into),
            rn: rn.into(),
            signed: kind.signed(),
            from_bits: kind.from_bits(),
            to_bits: kind.to_bits(),
        })
    }

    /// Bitwise AND (shifted register), setting flags.
    pub fn ands_rr(&mut self, rn: Reg, rm: Reg, size: OperandSize) {
        self.alu_rrr(ALUOp::AndS, rm, rn, writable!(regs::zero()), size);
    }

    /// Permanently Undefined.
    pub fn udf(&mut self, code: TrapCode) {
        self.emit(Inst::Udf { trap_code: code });
    }

    /// Conditional trap.
    pub fn trapif(&mut self, cc: Cond, code: TrapCode) {
        self.emit(Inst::TrapIf {
            kind: CondBrKind::Cond(cc),
            trap_code: code,
        });
    }

    /// Trap if `rn` is zero.
    pub fn trapz(&mut self, rn: Reg, code: TrapCode, size: OperandSize) {
        self.emit(Inst::TrapIf {
            kind: CondBrKind::Zero(rn.into(), size.into()),
            trap_code: code,
        });
    }

    // Helpers for ALU operations.

    fn alu_rri(&mut self, op: ALUOp, imm: Imm12, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.emit(Inst::AluRRImm12 {
            alu_op: op,
            size: size.into(),
            rd: rd.map(Into::into),
            rn: rn.into(),
            imm12: imm,
        });
    }

    fn alu_rri_logic(
        &mut self,
        op: ALUOp,
        imm: ImmLogic,
        rn: Reg,
        rd: WritableReg,
        size: OperandSize,
    ) {
        self.emit(Inst::AluRRImmLogic {
            alu_op: op,
            size: size.into(),
            rd: rd.map(Into::into),
            rn: rn.into(),
            imml: imm,
        });
    }

    fn alu_rri_shift(
        &mut self,
        op: ALUOp,
        imm: ImmShift,
        rn: Reg,
        rd: WritableReg,
        size: OperandSize,
    ) {
        self.emit(Inst::AluRRImmShift {
            alu_op: op,
            size: size.into(),
            rd: rd.map(Into::into),
            rn: rn.into(),
            immshift: imm,
        });
    }

    fn alu_rrr(&mut self, op: ALUOp, rm: Reg, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.emit(Inst::AluRRR {
            alu_op: op,
            size: size.into(),
            rd: rd.map(Into::into),
            rn: rn.into(),
            rm: rm.into(),
        });
    }

    fn alu_rrr_extend(&mut self, op: ALUOp, rm: Reg, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.emit(Inst::AluRRRExtend {
            alu_op: op,
            size: size.into(),
            rd: rd.map(Into::into),
            rn: rn.into(),
            rm: rm.into(),
            extendop: ExtendOp::UXTX,
        });
    }

    fn alu_rrrr(
        &mut self,
        op: ALUOp3,
        rm: Reg,
        rn: Reg,
        rd: WritableReg,
        ra: Reg,
        size: OperandSize,
    ) {
        self.emit(Inst::AluRRRR {
            alu_op: op,
            size: size.into(),
            rd: rd.map(Into::into),
            rn: rn.into(),
            rm: rm.into(),
            ra: ra.into(),
        });
    }

    fn fpu_rrr(&mut self, op: FPUOp2, rm: Reg, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.emit(Inst::FpuRRR {
            fpu_op: op,
            size: size.into(),
            rd: rd.map(Into::into),
            rn: rn.into(),
            rm: rm.into(),
        });
    }

    fn fpu_rri(&mut self, op: FPUOpRI, rn: Reg, rd: WritableReg) {
        self.emit(Inst::FpuRRI {
            fpu_op: op,
            rd: rd.map(Into::into),
            rn: rn.into(),
        });
    }

    fn fpu_rri_mod(&mut self, op: FPUOpRIMod, ri: Reg, rn: Reg, rd: WritableReg) {
        self.emit(Inst::FpuRRIMod {
            fpu_op: op,
            rd: rd.map(Into::into),
            ri: ri.into(),
            rn: rn.into(),
        });
    }

    fn fpu_rr(&mut self, op: FPUOp1, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.emit(Inst::FpuRR {
            fpu_op: op,
            size: size.into(),
            rd: rd.map(Into::into),
            rn: rn.into(),
        });
    }

    fn fpu_round(&mut self, op: FpuRoundMode, rn: Reg, rd: WritableReg) {
        self.emit(Inst::FpuRound {
            op: op,
            rd: rd.map(Into::into),
            rn: rn.into(),
        });
    }

    fn bit_rr(&mut self, op: BitOp, rn: Reg, rd: WritableReg, size: OperandSize) {
        self.emit(Inst::BitRR {
            op,
            size: size.into(),
            rd: rd.map(Into::into),
            rn: rn.into(),
        });
    }

    // Convert ShiftKind to ALUOp. If kind == Rotl, then emulate it by emitting
    // the negation of the given reg r, and returns ALUOp::Extr (an alias for
    // `ror` the rotate-right instruction)
    fn shift_kind_to_alu_op(&mut self, kind: ShiftKind, r: Reg, size: OperandSize) -> ALUOp {
        match kind {
            ShiftKind::Shl => ALUOp::Lsl,
            ShiftKind::ShrS => ALUOp::Asr,
            ShiftKind::ShrU => ALUOp::Lsr,
            ShiftKind::Rotr => ALUOp::Extr,
            ShiftKind::Rotl => {
                // neg(r) is sub(zero, r).
                self.alu_rrr(ALUOp::Sub, regs::zero(), r, writable!(r), size);
                ALUOp::Extr
            }
        }
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

    /// Emit a direct call to a function defined locally and
    /// referenced to by `name`.
    pub fn call_with_name(&mut self, name: UserExternalNameRef, call_conv: CallingConvention) {
        self.emit(Inst::Call {
            info: Box::new(cranelift_codegen::CallInfo::empty(
                ExternalName::user(name),
                call_conv.into(),
            )),
        })
    }

    /// Emit an indirect call to a function whose address is
    /// stored the `callee` register.
    pub fn call_with_reg(&mut self, callee: Reg, call_conv: CallingConvention) {
        self.emit(Inst::CallInd {
            info: Box::new(cranelift_codegen::CallInfo::empty(
                callee.into(),
                call_conv.into(),
            )),
        })
    }

    /// Emit a call to a well-known libcall.
    /// `dst` is used as a scratch register to hold the address of the libcall function.
    pub fn call_with_lib(&mut self, lib: LibCall, dst: Reg, call_conv: CallingConvention) {
        let name = ExternalName::LibCall(lib);
        self.emit(Inst::LoadExtName {
            rd: writable!(dst.into()),
            name: name.into(),
            offset: 0,
        });
        self.call_with_reg(dst, call_conv)
    }

    /// Load the min value for an integer of size out_size, as a floating-point
    /// of size `in-size`, into register `rd`.
    fn min_fp_value(
        &mut self,
        signed: bool,
        in_size: OperandSize,
        out_size: OperandSize,
        rd: Writable<Reg>,
    ) {
        use OperandSize::*;

        match in_size {
            S32 => {
                let min = match (signed, out_size) {
                    (true, S8) => i8::MIN as f32 - 1.,
                    (true, S16) => i16::MIN as f32 - 1.,
                    (true, S32) => i32::MIN as f32, // I32_MIN - 1 isn't precisely representable as a f32.
                    (true, S64) => i64::MIN as f32, // I64_MIN - 1 isn't precisely representable as a f32.

                    (false, _) => -1.,

                    (_, S128) => {
                        unimplemented!("floating point conversion to 128bit are not supported")
                    }
                };

                self.load_const_fp(min.to_bits() as u64, rd, in_size);
            }
            S64 => {
                let min = match (signed, out_size) {
                    (true, S8) => i8::MIN as f64 - 1.,
                    (true, S16) => i16::MIN as f64 - 1.,
                    (true, S32) => i32::MIN as f64 - 1.,
                    (true, S64) => i64::MIN as f64,

                    (false, _) => -1.,

                    (_, S128) => {
                        unimplemented!("floating point conversion to 128bit are not supported")
                    }
                };

                self.load_const_fp(min.to_bits(), rd, in_size);
            }
            s => unreachable!("unsupported floating-point size: {}bit", s.num_bits()),
        }
    }

    /// Load the max value for an integer of size out_size, as a floating-point
    /// of size `in_size`, into register `rd`.
    fn max_fp_value(
        &mut self,
        signed: bool,
        in_size: OperandSize,
        out_size: OperandSize,
        rd: Writable<Reg>,
    ) {
        use OperandSize::*;

        match in_size {
            S32 => {
                let max = match (signed, out_size) {
                    (true, S8) => i8::MAX as f32 + 1.,
                    (true, S16) => i16::MAX as f32 + 1.,
                    (true, S32) => i32::MAX as f32 + 1.,
                    (true, S64) => (i64::MAX as u64 + 1) as f32,

                    (false, S8) => u8::MAX as f32 + 1.,
                    (false, S16) => u16::MAX as f32 + 1.,
                    (false, S32) => u32::MAX as f32 + 1.,
                    (false, S64) => (u64::MAX as u128 + 1) as f32,

                    (_, S128) => {
                        unimplemented!("floating point conversion to 128bit are not supported")
                    }
                };

                self.load_const_fp(max.to_bits() as u64, rd, in_size);
            }
            S64 => {
                let max = match (signed, out_size) {
                    (true, S8) => i8::MAX as f64 + 1.,
                    (true, S16) => i16::MAX as f64 + 1.,
                    (true, S32) => i32::MAX as f64 + 1.,
                    (true, S64) => (i64::MAX as u64 + 1) as f64,

                    (false, S8) => u8::MAX as f64 + 1.,
                    (false, S16) => u16::MAX as f64 + 1.,
                    (false, S32) => u32::MAX as f64 + 1.,
                    (false, S64) => (u64::MAX as u128 + 1) as f64,

                    (_, S128) => {
                        unimplemented!("floating point conversion to 128bit are not supported")
                    }
                };

                self.load_const_fp(max.to_bits(), rd, in_size);
            }
            s => unreachable!("unsupported floating-point size: {}bit", s.num_bits()),
        }
    }

    /// Load the floating point number encoded in `n` of size `size`, into `rd`.
    fn load_const_fp(&mut self, n: u64, rd: Writable<Reg>, size: OperandSize) {
        // Check if we can load `n` directly, otherwise, load it into a tmp register, as an
        // integer, and then move that to `rd`.
        match ASIMDFPModImm::maybe_from_u64(n, size.into()) {
            Some(imm) => {
                self.emit(Inst::FpuMoveFPImm {
                    rd: rd.map(Into::into),
                    imm,
                    size: size.into(),
                });
            }
            None => {
                let tmp = regs::scratch();
                self.load_constant(n, Writable::from_reg(tmp));
                self.mov_to_fpu(tmp, rd, size)
            }
        }
    }

    /// Emit instructions to check if the value in `rn` is NaN.
    fn check_nan(&mut self, rn: Reg, size: OperandSize) {
        self.fcmp(rn, rn, size);
        self.trapif(Cond::Vs, TrapCode::BAD_CONVERSION_TO_INTEGER);
    }

    /// Convert the floating point of size `src_size` stored in `src`, into a integer of size
    /// `dst_size`, storing the result in `dst`.
    pub fn fpu_to_int(
        &mut self,
        dst: Writable<Reg>,
        src: Reg,
        src_size: OperandSize,
        dst_size: OperandSize,
        kind: TruncKind,
        signed: bool,
    ) {
        if kind.is_unchecked() {
            // Confusingly, when `kind` is `Unchecked` is when we actually need to perform the checks:
            // - check if fp is NaN
            // - check bounds
            self.check_nan(src, src_size);

            let tmp_reg = writable!(regs::float_scratch());
            self.min_fp_value(signed, src_size, dst_size, tmp_reg);
            self.fcmp(src, tmp_reg.to_reg(), src_size);
            self.trapif(Cond::Le, TrapCode::INTEGER_OVERFLOW);

            self.max_fp_value(signed, src_size, dst_size, tmp_reg);
            self.fcmp(src, tmp_reg.to_reg(), src_size);
            self.trapif(Cond::Ge, TrapCode::INTEGER_OVERFLOW);
        }

        self.cvt_fpu_to_int(dst, src, src_size, dst_size, signed)
    }

    /// Select and emit the appropriate `fcvt*` instruction
    pub fn cvt_fpu_to_int(
        &mut self,
        dst: Writable<Reg>,
        src: Reg,
        src_size: OperandSize,
        dst_size: OperandSize,
        signed: bool,
    ) {
        let op = match (src_size, dst_size, signed) {
            (OperandSize::S32, OperandSize::S32, false) => FpuToIntOp::F32ToU32,
            (OperandSize::S32, OperandSize::S32, true) => FpuToIntOp::F32ToI32,
            (OperandSize::S32, OperandSize::S64, false) => FpuToIntOp::F32ToU64,
            (OperandSize::S32, OperandSize::S64, true) => FpuToIntOp::F32ToI64,
            (OperandSize::S64, OperandSize::S32, false) => FpuToIntOp::F64ToU32,
            (OperandSize::S64, OperandSize::S32, true) => FpuToIntOp::F64ToI32,
            (OperandSize::S64, OperandSize::S64, false) => FpuToIntOp::F64ToU64,
            (OperandSize::S64, OperandSize::S64, true) => FpuToIntOp::F64ToI64,
            (fsize, int_size, signed) => unimplemented!(
                "unsupported conversion: f{} to {}{}",
                fsize.num_bits(),
                if signed { "i" } else { "u" },
                int_size.num_bits(),
            ),
        };

        self.emit(Inst::FpuToInt {
            op,
            rd: dst.map(Into::into),
            rn: src.into(),
        });
    }
}
