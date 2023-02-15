//! Assembler library implementation for Aarch64.

use super::{address::Address, regs};
use crate::{masm::OperandSize, reg::Reg};
use cranelift_codegen::{
    ir::{MemFlags, self},
    isa::{aarch64::inst::{
        self,
        emit::{EmitInfo, EmitState},
        ALUOp, AMode, ExtendOp, Imm12, Inst, PairAMode, CallIndInfo,
    }, CallConv},
    settings, Final, MachBuffer, MachBufferFinalized, MachInstEmit, Writable,
};
use regalloc2::PRegSet;

/// An Aarch64 instruction operand.
#[derive(Debug)]
pub(crate) enum Operand {
    /// Register.
    Reg(Reg),
    /// Memory address.
    Mem(Address),
    /// 64-bit signed immediate.
    Imm(i64),
}

// Conversions between winch-codegen aarch64 types and cranelift-codegen
// aarch64 types.

impl From<OperandSize> for inst::OperandSize {
    fn from(size: OperandSize) -> Self {
        match size {
            OperandSize::S32 => Self::Size32,
            OperandSize::S64 => Self::Size64,
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
    pub fn finalize(self) -> MachBufferFinalized<Final> {
        let stencil = self.buffer.finish();
        stencil.apply_base_srcloc(Default::default())
    }

    fn emit(&mut self, inst: Inst) {
        inst.emit(&[], &mut self.buffer, &self.emit_info, &mut self.emit_state);
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

    /// Move instruction combinations.
    pub fn mov(&mut self, src: Operand, dst: Operand, size: OperandSize) {
        match &(src, dst) {
            (Operand::Imm(imm), Operand::Reg(rd)) => {
                let scratch = regs::scratch();
                self.load_constant(*imm as u64, scratch);
                self.mov_rr(scratch, *rd, size);
            }
            (Operand::Reg(src), Operand::Reg(rd)) => {
                self.mov_rr(*src, *rd, size);
            }

            (src, dst) => panic!(
                "Invalid combination for mov: src = {:?}, dst = {:?}",
                src, dst
            ),
        }
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

    /// Add instruction combinations.
    pub fn add(&mut self, opm: Operand, opn: Operand, opd: Operand, size: OperandSize) {
        match &(opm, opn, opd) {
            (Operand::Imm(imm), Operand::Reg(rn), Operand::Reg(rd)) => {
                self.add_ir(*imm as u64, *rn, *rd, size);
            }
            (Operand::Reg(rm), Operand::Reg(rn), Operand::Reg(rd)) => {
                self.emit_alu_rrr_extend(ALUOp::Add, *rm, *rn, *rd, size);
            }
            (rm, rn, rd) => panic!(
                "Invalid combination for add: rm = {:?}, rn = {:?}, rd = {:?}",
                rm, rn, rd
            ),
        }
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

    /// Branch with a link to a register.
    pub fn blr(&mut self, rn: Reg) {
        self.emit(Inst::CallInd {
            info: Box::new(CallIndInfo {
                rn: rn.into(),
                uses: smallvec::smallvec![],
                defs: smallvec::smallvec![],
                clobbers: PRegSet::empty(),
                opcode: ir::Opcode::CallIndirect,
                caller_callconv: CallConv::SystemV,
                callee_callconv: CallConv::SystemV,
            }),
        });
    }

    /// Return instruction.
    pub fn ret(&mut self) {
        self.emit(Inst::Ret { rets: vec![] });
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
}
