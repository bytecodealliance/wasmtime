//! ISLE integration glue code for arm32 lowering.

// Pull in the ISLE generated code.
pub mod generated_code;
use generated_code::MInst;

// Types that the generated ISLE code refers to via `use super::*`.
use crate::ir::condcodes::{FloatCC, IntCC};
use crate::ir::immediates::*;
use crate::ir::types::*;
use crate::ir::{
    BlockCall, ExternalName, Inst, InstructionData, MemFlags, Opcode, TrapCode, Value, ValueList,
};
use crate::isa::arm32::Arm32Backend;
use crate::isa::arm32::inst::{ALUOp, Cond, ShiftOp, encode_rotated_imm};
use crate::machinst::isle::*;
use crate::machinst::{
    ArgPair, CallArgList, CallInfo, CallRetList, InstOutput, Lower, MachInst, MachLabel, RetPair,
    Sig, TryCallInfo, VCodeConstant, VCodeConstantData, VCodeInst,
};
use alloc::boxed::Box;
use alloc::vec::Vec;
use regalloc2::PReg;

type VecArgPair = Vec<ArgPair>;
type VecRetPair = Vec<RetPair>;
type VecMachLabel = Vec<MachLabel>;
type BoxCallInfo = Box<CallInfo<ExternalName>>;
type BoxCallIndInfo = Box<CallInfo<Reg>>;

/// The ISLE lowering context for arm32.
pub(crate) struct Arm32IsleContext<'a, 'b, I, B>
where
    I: VCodeInst,
    B: LowerBackend,
{
    pub lower_ctx: &'a mut Lower<'b, I>,
    pub backend: &'a B,
}

impl<'a, 'b> Arm32IsleContext<'a, 'b, MInst, Arm32Backend> {
    fn new(lower_ctx: &'a mut Lower<'b, MInst>, backend: &'a Arm32Backend) -> Self {
        Self { lower_ctx, backend }
    }

    pub(crate) fn dfg(&self) -> &crate::ir::DataFlowGraph {
        &self.lower_ctx.f.dfg
    }

    /// Emit a single 32-bit shift `op rd, rm, #amount` (1 <= amount <= 31).
    fn shift_raw(&mut self, op: ShiftOp, rm: Reg, amount: u8) -> Reg {
        let rd = self.lower_ctx.alloc_tmp(I32).only_reg().unwrap();
        self.lower_ctx.emit(MInst::ShiftImm { op, rd, rm, amount });
        rd.to_reg()
    }

    /// Emit `orr rd, a, b`.
    fn orr_raw(&mut self, a: Reg, b: Reg) -> Reg {
        self.alu_raw(ALUOp::Orr, false, a, b)
    }

    /// Emit `op rd, rn, #imm12` (immediate already rotated-encoded).
    fn alu_imm_raw(&mut self, op: ALUOp, rn: Reg, imm12: u32) -> Reg {
        let rd = self.lower_ctx.alloc_tmp(I32).only_reg().unwrap();
        self.lower_ctx.emit(MInst::AluRRImm { op, rd, rn, imm12 });
        rd.to_reg()
    }

    /// `vmov r, s` — move an S register's bits into a GPR.
    fn mov_from_s_raw(&mut self, s: Reg) -> Reg {
        let rt = self.lower_ctx.alloc_tmp(I32).only_reg().unwrap();
        self.lower_ctx.emit(MInst::MovFromFpu32 { rt, rm: s });
        rt.to_reg()
    }

    /// `vmov s, r` — move a GPR's bits into an S register.
    fn mov_to_s_raw(&mut self, gpr: Reg) -> Reg {
        let rd = self.lower_ctx.alloc_tmp(F32).only_reg().unwrap();
        self.lower_ctx.emit(MInst::MovToFpu32 { rd, rt: gpr });
        rd.to_reg()
    }

    /// `vmov rlo, rhi, d` — move a D register's bits into two GPRs.
    fn mov_from_d_raw(&mut self, d: Reg) -> (Reg, Reg) {
        let lo = self.lower_ctx.alloc_tmp(I32).only_reg().unwrap();
        let hi = self.lower_ctx.alloc_tmp(I32).only_reg().unwrap();
        self.lower_ctx.emit(MInst::MovFromFpu64 {
            rt_lo: lo,
            rt_hi: hi,
            rm: d,
        });
        (lo.to_reg(), hi.to_reg())
    }

    /// `vmov d, rlo, rhi` — move two GPRs into a D register.
    fn mov_to_d_raw(&mut self, lo: Reg, hi: Reg) -> Reg {
        let rd = self.lower_ctx.alloc_tmp(F64).only_reg().unwrap();
        self.lower_ctx.emit(MInst::MovToFpu64 {
            rd,
            rt_lo: lo,
            rt_hi: hi,
        });
        rd.to_reg()
    }

    /// Emit `op{s} rd, rn, rm` into a fresh register (which is returned).
    fn alu_raw(&mut self, op: ALUOp, set_flags: bool, rn: Reg, rm: Reg) -> Reg {
        let rd = self.lower_ctx.alloc_tmp(I32).only_reg().unwrap();
        let inst = if set_flags {
            MInst::AluRRRFlags { op, rd, rn, rm }
        } else {
            MInst::AluRRR { op, rd, rn, rm }
        };
        self.lower_ctx.emit(inst);
        rd.to_reg()
    }

}

impl generated_code::Context for Arm32IsleContext<'_, '_, MInst, Arm32Backend> {
    isle_lower_prelude_methods!();

    fn emit(&mut self, inst: &MInst) -> Unit {
        self.lower_ctx.emit(inst.clone());
    }

    fn lower_br_table(&mut self, index: Reg, targets: &[MachLabel]) -> Unit {
        let tmp = self.lower_ctx.alloc_tmp(I32).only_reg().unwrap();
        self.lower_ctx.emit(MInst::BrTable {
            index,
            tmp,
            targets: targets.to_vec(),
        });
    }

    fn gen_call_info(
        &mut self,
        sig: Sig,
        dest: ExternalName,
        uses: CallArgList,
        defs: CallRetList,
        try_call_info: Option<TryCallInfo>,
        patchable: bool,
    ) -> BoxCallInfo {
        let stack_ret_space = self.lower_ctx.sigs()[sig].sized_stack_ret_space();
        let stack_arg_space = self.lower_ctx.sigs()[sig].sized_stack_arg_space();
        self.lower_ctx
            .abi_mut()
            .accumulate_outgoing_args_size(stack_ret_space + stack_arg_space);
        Box::new(
            self.lower_ctx
                .gen_call_info(sig, dest, uses, defs, try_call_info, patchable),
        )
    }

    fn gen_call_ind_info(
        &mut self,
        sig: Sig,
        dest: Reg,
        uses: CallArgList,
        defs: CallRetList,
        try_call_info: Option<TryCallInfo>,
    ) -> BoxCallIndInfo {
        let stack_ret_space = self.lower_ctx.sigs()[sig].sized_stack_ret_space();
        let stack_arg_space = self.lower_ctx.sigs()[sig].sized_stack_arg_space();
        self.lower_ctx
            .abi_mut()
            .accumulate_outgoing_args_size(stack_ret_space + stack_arg_space);
        Box::new(
            self.lower_ctx
                .gen_call_info(sig, dest, uses, defs, try_call_info, false),
        )
    }

    /// Materialize a 32-bit constant into a register with the shortest sequence.
    fn gen_constant(&mut self, val: u64) -> Reg {
        let val = val as u32;
        let rd = self.lower_ctx.alloc_tmp(I32).only_reg().unwrap();
        let inst = if let Some(imm12) = encode_rotated_imm(val) {
            MInst::MovRotImm { rd, imm12 }
        } else if let Some(imm12) = encode_rotated_imm(!val) {
            MInst::MvnRotImm { rd, imm12 }
        } else if val >> 16 == 0 {
            MInst::Movw {
                rd,
                imm16: val,
            }
        } else {
            MInst::MovImm {
                rd,
                imm: u64::from(val),
            }
        };
        self.lower_ctx.emit(inst);
        rd.to_reg()
    }

    /// Succeeds if the low 32 bits of `val` are encodable as a rotated imm12.
    fn u64_from_rotated_imm12(&mut self, val: u64) -> Option<u32> {
        encode_rotated_imm(val as u32)
    }

    /// The high 32 bits of a 64-bit constant.
    fn u64_high32(&mut self, val: u64) -> u64 {
        val >> 32
    }

    /// Zero-extend a u32 to u64.
    fn u32_to_u64(&mut self, val: u32) -> u64 {
        u64::from(val)
    }

    /// The offset of the high word of a 64-bit memory access.
    fn offset32_plus4(&mut self, offset: Offset32) -> i32 {
        i32::from(offset) + 4
    }

    /// Shift/rotate a 64-bit `(lo, hi)` pair by a constant amount, emitting the
    /// funnel-shift sequence. Only Lsl/Lsr/Asr are supported.
    fn gen_i64_shift_imm(&mut self, op: &ShiftOp, x: ValueRegs, amount: u64) -> ValueRegs {
        let n = (amount & 63) as u8;
        let xlo = x.regs()[0];
        let xhi = x.regs()[1];
        if n == 0 {
            return ValueRegs::two(xlo, xhi);
        }
        match *op {
            ShiftOp::Lsl => {
                if n < 32 {
                    let lo = self.shift_raw(ShiftOp::Lsl, xlo, n);
                    let hi_hi = self.shift_raw(ShiftOp::Lsl, xhi, n);
                    let hi_lo = self.shift_raw(ShiftOp::Lsr, xlo, 32 - n);
                    let hi = self.orr_raw(hi_hi, hi_lo);
                    ValueRegs::two(lo, hi)
                } else {
                    let zero = self.gen_constant(0);
                    let hi = if n == 32 {
                        xlo
                    } else {
                        self.shift_raw(ShiftOp::Lsl, xlo, n - 32)
                    };
                    ValueRegs::two(zero, hi)
                }
            }
            ShiftOp::Lsr => {
                if n < 32 {
                    let hi = self.shift_raw(ShiftOp::Lsr, xhi, n);
                    let lo_lo = self.shift_raw(ShiftOp::Lsr, xlo, n);
                    let lo_hi = self.shift_raw(ShiftOp::Lsl, xhi, 32 - n);
                    let lo = self.orr_raw(lo_lo, lo_hi);
                    ValueRegs::two(lo, hi)
                } else {
                    let zero = self.gen_constant(0);
                    let lo = if n == 32 {
                        xhi
                    } else {
                        self.shift_raw(ShiftOp::Lsr, xhi, n - 32)
                    };
                    ValueRegs::two(lo, zero)
                }
            }
            ShiftOp::Asr => {
                if n < 32 {
                    let hi = self.shift_raw(ShiftOp::Asr, xhi, n);
                    let lo_lo = self.shift_raw(ShiftOp::Lsr, xlo, n);
                    let lo_hi = self.shift_raw(ShiftOp::Lsl, xhi, 32 - n);
                    let lo = self.orr_raw(lo_lo, lo_hi);
                    ValueRegs::two(lo, hi)
                } else {
                    // The high word becomes all sign bits; the low word is the
                    // old high word arithmetically shifted by `n - 32`.
                    let hi = self.shift_raw(ShiftOp::Asr, xhi, 31);
                    let lo = if n == 32 {
                        xhi
                    } else {
                        self.shift_raw(ShiftOp::Asr, xhi, n - 32)
                    };
                    ValueRegs::two(lo, hi)
                }
            }
            ShiftOp::Ror => unimplemented!("arm32: 64-bit rotate is not implemented"),
        }
    }

    /// Shift/rotate by a constant, applying Cranelift's modulo-width semantics.
    /// A masked amount of zero becomes a plain register move.
    fn gen_shift_imm(&mut self, op: &ShiftOp, rm: Reg, amount: u64) -> Reg {
        let amount = (amount & 31) as u8;
        let rd = self.lower_ctx.alloc_tmp(I32).only_reg().unwrap();
        let inst = if amount == 0 {
            MInst::MovReg { rd, rm }
        } else {
            MInst::ShiftImm {
                op: *op,
                rd,
                rm,
                amount,
            }
        };
        self.lower_ctx.emit(inst);
        rd.to_reg()
    }

    /// Whether the hardware integer-divide instructions may be used.
    fn use_idiv(&mut self) -> bool {
        self.backend.isa_flags.has_idiv()
    }

    /// Emit the compare sequence for a 64-bit `icmp` and return the ARM
    /// condition code that tests its result.
    ///
    /// A 64-bit `subs`/`sbcs` sets N, V and C correctly for the full value but
    /// its Z flag only reflects the high word, so it's only usable for the
    /// conditions that don't depend on Z (lt/ge/lo/hs). The gt/le/hi/ls forms
    /// are obtained by swapping the operands, and eq/ne use a separate
    /// `eor`/`eor`/`orrs` reduction.
    fn lower_icmp_i64(&mut self, cc: &IntCC, a: ValueRegs, b: ValueRegs) -> Cond {
        let (alo, ahi) = (a.regs()[0], a.regs()[1]);
        let (blo, bhi) = (b.regs()[0], b.regs()[1]);
        match cc {
            IntCC::Equal | IntCC::NotEqual => {
                let tlo = self.alu_raw(ALUOp::Eor, false, alo, blo);
                let thi = self.alu_raw(ALUOp::Eor, false, ahi, bhi);
                // `orrs` sets Z iff both halves are equal, i.e. a == b.
                let _ = self.alu_raw(ALUOp::Orr, true, tlo, thi);
                if matches!(cc, IntCC::Equal) {
                    Cond::Eq
                } else {
                    Cond::Ne
                }
            }
            _ => {
                // (lo1, hi1) - (lo2, hi2), then test `cond`.
                let (lo1, hi1, lo2, hi2, cond) = match cc {
                    IntCC::SignedLessThan => (alo, ahi, blo, bhi, Cond::Lt),
                    IntCC::SignedGreaterThanOrEqual => (alo, ahi, blo, bhi, Cond::Ge),
                    IntCC::SignedGreaterThan => (blo, bhi, alo, ahi, Cond::Lt),
                    IntCC::SignedLessThanOrEqual => (blo, bhi, alo, ahi, Cond::Ge),
                    IntCC::UnsignedLessThan => (alo, ahi, blo, bhi, Cond::Lo),
                    IntCC::UnsignedGreaterThanOrEqual => (alo, ahi, blo, bhi, Cond::Hs),
                    IntCC::UnsignedGreaterThan => (blo, bhi, alo, ahi, Cond::Lo),
                    IntCC::UnsignedLessThanOrEqual => (blo, bhi, alo, ahi, Cond::Hs),
                    _ => unreachable!(),
                };
                let _ = self.alu_raw(ALUOp::Sub, true, lo1, lo2); // subs
                let _ = self.alu_raw(ALUOp::Sbc, true, hi1, hi2); // sbcs
                cond
            }
        }
    }

    /// Map a `FloatCC` to the ARM condition testing the flags left by a `vcmp`
    /// (`vmrs`). The `vcmp` sets N/Z/C/V per an IEEE ordered comparison, with an
    /// unordered result giving C=1, V=1.
    fn cond_from_floatcc(&mut self, cc: &FloatCC) -> Cond {
        match cc {
            FloatCC::Equal => Cond::Eq,
            FloatCC::NotEqual => Cond::Ne,
            FloatCC::LessThan => Cond::Mi,
            FloatCC::LessThanOrEqual => Cond::Ls,
            FloatCC::GreaterThan => Cond::Gt,
            FloatCC::GreaterThanOrEqual => Cond::Ge,
            FloatCC::Ordered => Cond::Vc,
            FloatCC::Unordered => Cond::Vs,
            FloatCC::UnorderedOrLessThan => Cond::Lt,
            FloatCC::UnorderedOrLessThanOrEqual => Cond::Le,
            FloatCC::UnorderedOrGreaterThan => Cond::Hi,
            FloatCC::UnorderedOrGreaterThanOrEqual => Cond::Hs,
            // These need a two-condition sequence and aren't handled yet.
            FloatCC::OrderedNotEqual | FloatCC::UnorderedOrEqual => {
                unimplemented!("arm32: fcmp condition {cc:?} not yet implemented")
            }
        }
    }

    /// `fcopysign` for f32: clear the sign bit of `a` and OR in `b`'s sign,
    /// working on the raw bits in GPRs.
    fn gen_copysign_f32(&mut self, a: Reg, b: Reg) -> Reg {
        let sign_mask = encode_rotated_imm(0x8000_0000).unwrap();
        let a_bits = self.mov_from_s_raw(a);
        let b_bits = self.mov_from_s_raw(b);
        let sign = self.alu_imm_raw(ALUOp::And, b_bits, sign_mask);
        let cleared = self.alu_imm_raw(ALUOp::Bic, a_bits, sign_mask);
        let res = self.alu_raw(ALUOp::Orr, false, cleared, sign);
        self.mov_to_s_raw(res)
    }

    /// `fcopysign` for f64: the sign bit lives in the high word.
    fn gen_copysign_f64(&mut self, a: Reg, b: Reg) -> Reg {
        let sign_mask = encode_rotated_imm(0x8000_0000).unwrap();
        let (a_lo, a_hi) = self.mov_from_d_raw(a);
        let (_b_lo, b_hi) = self.mov_from_d_raw(b);
        let sign = self.alu_imm_raw(ALUOp::And, b_hi, sign_mask);
        let cleared = self.alu_imm_raw(ALUOp::Bic, a_hi, sign_mask);
        let res_hi = self.alu_raw(ALUOp::Orr, false, cleared, sign);
        self.mov_to_d_raw(a_lo, res_hi)
    }

    fn cond_from_intcc(&mut self, cc: &IntCC) -> Cond {
        match cc {
            IntCC::Equal => Cond::Eq,
            IntCC::NotEqual => Cond::Ne,
            IntCC::SignedLessThan => Cond::Lt,
            IntCC::SignedGreaterThanOrEqual => Cond::Ge,
            IntCC::SignedGreaterThan => Cond::Gt,
            IntCC::SignedLessThanOrEqual => Cond::Le,
            IntCC::UnsignedLessThan => Cond::Lo,
            IntCC::UnsignedGreaterThanOrEqual => Cond::Hs,
            IntCC::UnsignedGreaterThan => Cond::Hi,
            IntCC::UnsignedLessThanOrEqual => Cond::Ls,
        }
    }
}

/// The main entry point for lowering with ISLE.
pub(crate) fn lower(
    lower_ctx: &mut Lower<MInst>,
    backend: &Arm32Backend,
    inst: Inst,
) -> Option<InstOutput> {
    let mut isle_ctx = Arm32IsleContext::new(lower_ctx, backend);
    generated_code::constructor_lower(&mut isle_ctx, inst)
}

/// The main entry point for branch lowering with ISLE.
pub(crate) fn lower_branch(
    lower_ctx: &mut Lower<MInst>,
    backend: &Arm32Backend,
    branch: Inst,
    targets: &[MachLabel],
) -> Option<()> {
    let mut isle_ctx = Arm32IsleContext::new(lower_ctx, backend);
    generated_code::constructor_lower_branch(&mut isle_ctx, branch, targets)
}
