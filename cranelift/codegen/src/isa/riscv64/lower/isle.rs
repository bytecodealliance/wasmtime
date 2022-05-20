//! ISLE integration glue code for riscv64 lowering.

// Pull in the ISLE generated code.
pub mod generated_code;

use self::generated_code::I128ArithmeticOP;

// Types that the generated ISLE code uses via `use super::*`.
use super::{writable_zero_reg, zero_reg, Inst as MInst};

use crate::isa::riscv64::settings::Flags as IsaFlags;
use crate::machinst::{isle::*, MachInst, SmallInstVec};
use crate::settings::Flags;

use crate::machinst::{VCodeConstant, VCodeConstantData};
use crate::{
    ir::{
        immediates::*, types::*, ExternalName, Inst, InstructionData, MemFlags, TrapCode, Value,
        ValueList,
    },
    isa::riscv64::inst::*,
    machinst::{InsnOutput, LowerCtx},
};

use std::boxed::Box;
use std::convert::TryFrom;
use std::vec::Vec;

use crate::machinst::Reg;

type BoxCallInfo = Box<CallInfo>;
type BoxCallIndInfo = Box<CallIndInfo>;
type VecMachLabel = Vec<MachLabel>;
type BoxExternalName = Box<ExternalName>;

/// The main entry point for lowering with ISLE.
pub(crate) fn lower<C>(
    lower_ctx: &mut C,
    flags: &Flags,
    isa_flags: &IsaFlags,
    outputs: &[InsnOutput],
    inst: Inst,
) -> Result<(), ()>
where
    C: LowerCtx<I = MInst>,
{
    lower_common(lower_ctx, flags, isa_flags, outputs, inst, |cx, insn| {
        generated_code::constructor_lower(cx, insn)
    })
}

impl<C> generated_code::Context for IsleContext<'_, C, Flags, IsaFlags, 6>
where
    C: LowerCtx<I = MInst>,
{
    isle_prelude_methods!();

    fn imm(&mut self, ty: Type, mut val: u64) -> Reg {
        /*
        Boolean types
        Boolean values are either true or false.

        The b1 type represents an abstract boolean value. It can only exist as an SSA value, and can't be directly stored in memory. It can, however, be converted into an integer with value 0 or 1 by the bint instruction (and converted back with icmp_imm with 0).

        Several larger boolean types are also defined, primarily to be used as SIMD element types. They can be stored in memory, and are represented as either all zero bits or all one bits.

        b1
        b8
        b16
        b32
        b64
        ///////////////////////////////////////////////////////////
        "represented as either all zero bits or all one bits."
        \\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\
                        */
        if ty.is_bool() && val != 0 {
            // need all be one
            val = !0;
        }
        let tmp = self.temp_writable_reg(ty);
        self.emit_list(&MInst::load_constant_u64(tmp, val));
        tmp.to_reg()
    }
    #[inline(always)]
    fn emit(&mut self, arg0: &MInst) -> Unit {
        self.lower_ctx.emit(arg0.clone());
    }
    #[inline(always)]
    fn imm12_from_u64(&mut self, arg0: u64) -> Option<Imm12> {
        Imm12::maybe_from_u64(arg0)
    }
    #[inline(always)]
    fn writable_zero_reg(&mut self) -> WritableReg {
        writable_zero_reg()
    }
    #[inline(always)]
    fn neg_imm12(&mut self, arg0: Imm12) -> Imm12 {
        -arg0
    }
    #[inline(always)]
    fn zero_reg(&mut self) -> Reg {
        zero_reg()
    }
    #[inline(always)]
    fn imm_from_bits(&mut self, val: u64) -> Imm12 {
        Imm12::maybe_from_u64(val).unwrap()
    }
    #[inline(always)]
    fn imm_from_neg_bits(&mut self, val: i64) -> Imm12 {
        Imm12::maybe_from_u64(val as u64).unwrap()
    }

    fn float_bnot(&mut self, ty: Type, r: Reg) -> Reg {
        let tmp_i = self.temp_writable_reg(I64);
        let inst = gen_move(tmp_i, I64, r, ty);
        self.emit(&inst);
        self.emit(&MInst::construct_bit_not(tmp_i, tmp_i.to_reg()));
        let tmp_f = self.temp_writable_reg(ty);
        let inst = gen_move(tmp_f, ty, tmp_i.to_reg(), I64);
        self.emit(&inst);
        tmp_f.to_reg()
    }
    fn bnot_128(&mut self, value: ValueRegs) -> ValueRegs {
        let tmp_hight = self.temp_writable_reg(I64);
        let tmp_low = self.temp_writable_reg(I64);
        let high = value.regs()[1];
        let low = value.regs()[0];
        self.emit(&MInst::construct_bit_not(tmp_hight, high));
        self.emit(&MInst::construct_bit_not(tmp_low, low));
        self.value_regs(tmp_low.to_reg(), tmp_hight.to_reg())
    }
    fn band_128(&mut self, x: ValueRegs, y: ValueRegs) -> ValueRegs {
        let tmp_hight = self.temp_writable_reg(I64);
        let tmp_low = self.temp_writable_reg(I64);
        self.emit(&MInst::AluRRR {
            alu_op: AluOPRRR::And,
            rd: tmp_hight,
            rs1: x.regs()[0],
            rs2: y.regs()[0],
        });
        self.emit(&MInst::AluRRR {
            alu_op: AluOPRRR::And,
            rd: tmp_hight,
            rs1: x.regs()[1],
            rs2: y.regs()[1],
        });
        self.value_regs(tmp_low.to_reg(), tmp_hight.to_reg())
    }

    fn i128_arithmetic(&mut self, op: &I128ArithmeticOP, x: ValueRegs, y: ValueRegs) -> ValueRegs {
        let mut dst = Vec::with_capacity(2);
        dst.push(self.temp_writable_reg(I64));
        dst.push(self.temp_writable_reg(I64));
        let t0 = self.temp_writable_reg(I64);
        let t1 = self.temp_writable_reg(I64);

        self.emit(&MInst::I128Arithmetic {
            op: *op,
            t0,
            t1,
            dst: dst.clone(),
            x,
            y,
        });
        self.value_regs(dst[0].to_reg(), dst[1].to_reg())
    }

    fn bit_reverse(&mut self, ty: Type, rs: Reg) -> Reg {
        // let tmp = self.temp_writable_reg(I64);
        // self.emit(&MInst::AluRRImm12 {
        //     alu_op: AluOPRRI::Rev8,
        //     rd: tmp,
        //     rs,
        //     imm12: AluOPRRI::Rev8.funct12(None),
        // });
        // if ty.bits() != 64 {
        //     let shift = 64 - ty.bits();
        //     self.emit(&MInst::AluRRImm12 {
        //         alu_op: AluOPRRI::Srli,
        //         rd: tmp,
        //         rs: tmp.to_reg(),
        //         imm12: Imm12::from_bits(shift as i16),
        //     });
        // }
        // tmp.to_reg()
        todo!()
    }
    fn r_clz(&mut self, ty: Type, val: ValueRegs) -> Reg {
        // if ty.bits() == 128 {
        // } else {
        //     let tmp = self.temp_writable_reg(I64);
        //     self.emit(&MInst::AluRRImm12 {});
        // }
        todo!()
    }

    fn r_ctz(&mut self, ty: Type, val: ValueRegs) -> Reg {
        let tmp1 = self.temp_writable_reg(I64);
        match ty.bits() {
            128 => {
                let tmp2 = self.temp_writable_reg(I64);
                let tmp3 = self.temp_writable_reg(I64);
                // first count lower trailing zeros
                let (op, imm12) = AluOPRRI::Ctz.funct12(None);
                self.emit(&MInst::AluRRImm12 {
                    alu_op: op,
                    rd: tmp1,
                    rs: val.regs()[0],
                    imm12,
                });
                // load constant 64

                self.emit(&MInst::AluRRImm12 {
                    alu_op: AluOPRRI::Ori,
                    rd: tmp3,
                    rs: zero_reg(),
                    imm12: Imm12::from_bits(64),
                });
                // if lower trailing zeros is less than 64 we know the upper 64-bit no need to count.
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Slt,
                    rd: tmp3,
                    rs1: tmp1.to_reg(),
                    rs2: tmp3.to_reg(),
                });
                // set high part lowest bit.
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Or,
                    rd: tmp2, /* if tmp2 == 0 we don't change the high part value and need to count ,otherwise
                              we set lowest bit to 1 , Ctz will return 0 which is the result we want.
                                        */
                    rs1: val.regs()[1],
                    rs2: tmp2.to_reg(),
                });
                // count hight parts
                let (op, imm12) = AluOPRRI::Ctz.funct12(None);
                self.emit(&MInst::AluRRImm12 {
                    alu_op: op,
                    rd: tmp2,
                    rs: tmp2.to_reg(),
                    imm12: imm12,
                });
                // add them togother
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Add,
                    rd: tmp1,
                    rs1: tmp1.to_reg(),
                    rs2: tmp2.to_reg(),
                });
            }
            64 => {
                let (op, imm12) = AluOPRRI::Ctz.funct12(None);
                self.emit(&MInst::AluRRImm12 {
                    alu_op: op,
                    rd: tmp1,
                    rs: val.regs()[0],
                    imm12,
                });
            }
            32 => {
                let (op, imm12) = AluOPRRI::Ctzw.funct12(None);
                self.emit(&MInst::AluRRImm12 {
                    alu_op: op,
                    rd: tmp1,
                    rs: val.regs()[0],
                    imm12: imm12,
                });
            }
            16 | 8 => {
                // first we must make sure all upper bit are 1
                // so we don't count extra zero.
                MInst::load_constant_u64(tmp1, if ty.bits() == 8 { !0xff } else { !0xffff })
                    .into_iter()
                    .for_each(|i| self.emit(&i));

                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Or,
                    rd: tmp1,
                    rs1: tmp1.to_reg(),
                    rs2: val.regs()[0],
                });
                let (op, imm12) = AluOPRRI::Ctz.funct12(None);
                self.emit(&MInst::AluRRImm12 {
                    alu_op: op,
                    rd: tmp1,
                    rs: val.regs()[0],
                    imm12,
                });
            }
            _ => unreachable!(),
        }
        tmp1.to_reg()
    }

    fn extend(&mut self, val: Reg, is_signed: bool, from_bits: u8, to_bits: u8) -> ValueRegs {
        if is_signed {
            if to_bits == 128 {
                let low = self.temp_writable_reg(I64);
                let tmp = self.temp_writable_reg(I64);
                let high = self.temp_writable_reg(I64);
                // extend the lower parts if need
                if from_bits != 64 {
                    self.emit(&MInst::Extend {
                        rd: tmp,
                        rn: val,
                        signed: is_signed,
                        from_bits,
                        to_bits: 64,
                    });
                } else {
                    self.emit(&MInst::gen_move(low, val, I64));
                }
                // extract the signed bit
                let (op, imm12) = AluOPRRI::Bexti.funct12(Some(from_bits - 1));
                self.emit(&MInst::AluRRImm12 {
                    alu_op: op,
                    rd: tmp,
                    rs: val,
                    imm12,
                });
                //pretend signed extend from b1->b64
                self.emit(&MInst::Extend {
                    rd: high,
                    rn: tmp.to_reg(),
                    signed: is_signed,
                    from_bits: 1,
                    to_bits: 64,
                });
                ValueRegs::two(low.to_reg(), high.to_reg())
            } else {
                let tmp = self.temp_writable_reg(I64);
                self.emit(&MInst::Extend {
                    rd: tmp,
                    rn: val,
                    signed: is_signed,
                    from_bits,
                    to_bits,
                });
                ValueRegs::one(tmp.to_reg())
            }
        } else {
            // this  is unsigned.
            let tmp = self.temp_writable_reg(I64);
            self.emit(&MInst::Extend {
                rd: tmp,
                rn: val,
                signed: is_signed,
                from_bits,
                to_bits,
            });
            if to_bits == 128 {
                let tmp2 = self.temp_writable_reg(I64);
                self.emit(&MInst::load_constant_imm12(tmp2, Imm12::from_bits(0)));
                ValueRegs::two(tmp.to_reg(), tmp2.to_reg())
            } else {
                ValueRegs::one(tmp.to_reg())
            }
        }
    }
}

impl<C> IsleContext<'_, C, Flags, IsaFlags, 6>
where
    C: LowerCtx<I = MInst>,
{
    #[inline(always)]
    fn emit_list(&mut self, list: &SmallInstVec<MInst>) {
        for i in list {
            self.lower_ctx.emit(i.clone());
        }
    }

    // i128 implemetation
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ir::types::B8X8;
}
