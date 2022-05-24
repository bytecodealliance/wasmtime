//! ISLE integration glue code for riscv64 lowering.

// Pull in the ISLE generated code.
pub mod generated_code;

use alloc::borrow::ToOwned;

use self::generated_code::I128OP;

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

    fn i128_arithmetic(&mut self, op: &I128OP, x: ValueRegs, y: ValueRegs) -> ValueRegs {
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

    fn lower_bit_reverse(&mut self, ty: Type, rs: Reg) -> Reg {
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
    fn lower_clz(&mut self, ty: Type, val: ValueRegs) -> Reg {
        assert!(ty.is_int());
        let tmp = self.temp_writable_reg(I64);
        if ty != I128 {
            let rs = val.regs()[0];
            match ty.bits() {
                64 => {
                    let (op, imm12) = AluOPRRI::Clz.funct12(None);
                    self.emit(&MInst::AluRRImm12 {
                        alu_op: op,
                        rd: tmp,
                        rs,
                        imm12,
                    });
                }
                32 => {
                    let (op, imm12) = AluOPRRI::Clzw.funct12(None);
                    self.emit(&MInst::AluRRImm12 {
                        alu_op: op,
                        rd: tmp,
                        rs,
                        imm12,
                    });
                }
                16 | 8 => {
                    let rs = generated_code::constructor_narrow_int(self, ty, rs).unwrap();
                    let (op, imm12) = AluOPRRI::Clzw.funct12(None);
                    self.emit(&MInst::AluRRImm12 {
                        alu_op: op,
                        rd: tmp,
                        rs,
                        imm12,
                    });
                    self.emit(&MInst::AluRRImm12 {
                        alu_op: AluOPRRI::Addi,
                        rd: tmp,
                        rs: tmp.to_reg(),
                        imm12: Imm12::from_bits(-(32 - ty.bits() as i16)),
                    })
                }

                _ => unreachable!(),
            }
        } else {
            unimplemented!()
        }
        tmp.to_reg()
    }

    fn lower_ctz(&mut self, ty: Type, val: ValueRegs) -> ValueRegs {
        let rd = self.temp_writable_reg(I64);
        let (op, imm12) = AluOPRRI::Ctz.funct12(None);
        match ty.bits() {
            128 => {
                let tmp_high = self.temp_writable_reg(I64);
                // first count lower trailing zeros

                self.emit(&MInst::AluRRImm12 {
                    alu_op: op,
                    rd,
                    rs: val.regs()[0],
                    imm12,
                });
                let tmp_src_high = self.temp_writable_reg(I64);
                // load constant 64
                self.emit(&MInst::AluRRImm12 {
                    alu_op: AluOPRRI::Ori,
                    rd: tmp_src_high,
                    rs: zero_reg(),
                    imm12: Imm12::from_bits(64),
                });
                // if lower trailing zeros is less than 64 we know the upper 64-bit no need to count.
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Slt,
                    rd: tmp_src_high,
                    rs1: rd.to_reg(),
                    rs2: tmp_src_high.to_reg(),
                });
                // set high part lowest bit.
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Or,
                    rd: tmp_high, /* if tmp2 == 0 we don't change the high part value and need to count ,otherwise
                                  we set lowest bit to 1 , Ctz will return 0 which is the result we want.
                                            */
                    rs1: val.regs()[1],
                    rs2: tmp_src_high.to_reg(),
                });
                // count hight parts

                self.emit(&MInst::AluRRImm12 {
                    alu_op: op,
                    rd: tmp_high,
                    rs: tmp_high.to_reg(),
                    imm12: imm12,
                });
                // add them togother
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Add,
                    rd,
                    rs1: rd.to_reg(),
                    rs2: tmp_high.to_reg(),
                });

                /*

                    todo why return 128-bit value?????
                */
                let r = self.temp_writable_reg(I64);
                self.emit(&MInst::load_constant_imm12(r, Imm12::from_bits(0)));
                return ValueRegs::two(rd.to_reg(), r.to_reg());
            }
            64 => {
                self.emit(&MInst::AluRRImm12 {
                    alu_op: op,
                    rd,
                    rs: val.regs()[0],
                    imm12,
                });
            }
            32 => {
                self.emit(&MInst::AluRRImm12 {
                    alu_op: op,
                    rd,
                    rs: val.regs()[0],
                    imm12: imm12,
                });
            }
            16 | 8 => {
                // first we must make sure all upper bit are 1
                // so we don't count extra zero.
                MInst::load_constant_u64(rd, if ty.bits() == 8 { !0xff } else { !0xffff })
                    .into_iter()
                    .for_each(|i| self.emit(&i));

                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Or,
                    rd,
                    rs1: rd.to_reg(),
                    rs2: val.regs()[0],
                });
                let (op, imm12) = AluOPRRI::Ctz.funct12(None);
                self.emit(&MInst::AluRRImm12 {
                    alu_op: op,
                    rd,
                    rs: val.regs()[0],
                    imm12,
                });
            }
            _ => unreachable!(),
        }
        ValueRegs::one(rd.to_reg())
    }

    fn band_not_128(&mut self, a: ValueRegs, b: ValueRegs) -> ValueRegs {
        let low = self.temp_writable_reg(I64);
        let high = self.temp_writable_reg(I64);
        self.emit(&MInst::AluRRR {
            alu_op: AluOPRRR::Andn,
            rd: low,
            rs1: a.regs()[0],
            rs2: b.regs()[0],
        });
        self.emit(&MInst::AluRRR {
            alu_op: AluOPRRR::Andn,
            rd: high,
            rs1: a.regs()[1],
            rs2: b.regs()[1],
        });
        ValueRegs::two(low.to_reg(), high.to_reg())
    }

    fn bitmaip_imm12(&mut self, op: &AluOPRRI, shamt: u8) -> Imm12 {
        op.funct12(op.need_shamt().map(|mask| {
            assert!(mask >= shamt);
            shamt
        }))
        .1
    }

    fn lower_extend(&mut self, val: Reg, is_signed: bool, from_bits: u8, to_bits: u8) -> ValueRegs {
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

    fn b128_binary(&mut self, op: &AluOPRRR, a: ValueRegs, b: ValueRegs) -> ValueRegs {
        let op = *op;
        let low = self.temp_writable_reg(I64);
        let high = self.temp_writable_reg(I64);
        self.emit(&MInst::AluRRR {
            alu_op: op,
            rd: low,
            rs1: a.regs()[0],
            rs2: b.regs()[0],
        });
        self.emit(&MInst::AluRRR {
            alu_op: op,
            rd: high,
            rs1: a.regs()[1],
            rs2: b.regs()[1],
        });
        ValueRegs::two(low.to_reg(), high.to_reg())
    }

    fn lower_rotl(&mut self, ty: Type, rs: Reg, amount: Reg) -> Reg {
        let rd = self.temp_writable_reg(I64);
        match ty.bits() {
            64 => {
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Rol,
                    rd: rd,
                    rs1: rs,
                    rs2: amount,
                });
            }
            32 => {
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Rolw,
                    rd: rd,
                    rs1: rs,
                    rs2: amount,
                });
            }
            16 | 8 => {
                let amount = {
                    //shift is bigger than it's bits is useless.
                    //get rid of.
                    let old_amount = amount;
                    let tmp = self.temp_writable_reg(I64);
                    self.emit(&MInst::load_constant_imm12(
                        tmp,
                        Imm12::from_bits(ty.bits() as i16),
                    ));
                    let amount = self.temp_writable_reg(I64);
                    self.emit(&MInst::AluRRR {
                        alu_op: AluOPRRR::RemU,
                        rd: amount,
                        rs1: old_amount,
                        rs2: tmp.to_reg(),
                    });
                    amount.to_reg()
                };
                let value = self.temp_writable_reg(I64);
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Rol,
                    rd: value,
                    rs1: rs,
                    rs2: amount,
                });
                let mut insts = SmallInstVec::new();
                let tmp = self.temp_writable_reg(I64);
                let tmp_shift = {
                    let tmp_shift = self.temp_writable_reg(I64);
                    insts.extend(MInst::construct_imm_sub_rs(
                        tmp_shift,
                        ty.bits() as u64,
                        amount,
                    ));
                    tmp_shift.to_reg()
                };
                insts.extend(BitsShifter::new_r(tmp_shift).shift_out_right(tmp, value.to_reg()));
                let tmp2 = self.temp_writable_reg(I64);
                insts.extend(BitsShifter::new_i(ty.bits() as u8).shift_right(tmp2, value.to_reg()));
                insts.push(MInst::AluRRR {
                    alu_op: AluOPRRR::And,
                    rd,
                    rs1: tmp.to_reg(),
                    rs2: tmp2.to_reg(),
                });
                insts.iter().for_each(|i| self.emit(i));
            }
            _ => unreachable!(),
        }
        rd.to_reg()
    }

    fn lower_rotr(&mut self, ty: Type, rs: Reg, amount: Reg) -> Reg {
        let rd = self.temp_writable_reg(I64);
        match ty.bits() {
            64 => {
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Ror,
                    rd: rd,
                    rs1: rs,
                    rs2: amount,
                });
            }
            32 => {
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Rorw,
                    rd: rd,
                    rs1: rs,
                    rs2: amount,
                });
            }

            16 | 8 => {
                let amount = {
                    //shift is bigger than it's bits is useless.
                    //get rid of.
                    let old_amount = amount;
                    let tmp = self.temp_writable_reg(I64);
                    self.emit(&MInst::load_constant_imm12(
                        tmp,
                        Imm12::from_bits(ty.bits() as i16),
                    ));
                    let amount = self.temp_writable_reg(I64);
                    self.emit(&MInst::AluRRR {
                        alu_op: AluOPRRR::RemU,
                        rd: amount,
                        rs1: old_amount,
                        rs2: tmp.to_reg(),
                    });
                    amount.to_reg()
                };
                let value = self.temp_writable_reg(I64);
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Ror,
                    rd: value,
                    rs1: rs,
                    rs2: amount,
                });
                /*
                    let's protend amount == 5 and ty == I8
                        a's range is [0,8-5)    3 bits.
                        b's range is [64-5,64)  5 bits.
                    reuslt is a & (b >> 64-5)
                */
                todo!()
            }
            _ => unreachable!(),
        }
        rd.to_reg()
    }

    fn lower_mlhi(&mut self, is_signed: bool, ty: Type, a: Reg, b: Reg) -> Reg {
        let rd = self.temp_writable_reg(I64);
        if ty.bits() == 64 {
            self.emit(&MInst::AluRRR {
                alu_op: if is_signed {
                    AluOPRRR::Mulh
                } else {
                    AluOPRRR::Mulhu
                },
                rd: rd,
                rs1: a,
                rs2: b,
            });
        } else {
            /*
                first we must narrow down int,because we perform unsinged multiply.
            */
            let a = if is_signed {
                a
            } else {
                generated_code::constructor_narrow_int(self, ty, a).unwrap()
            };
            let b = if is_signed {
                b
            } else {
                generated_code::constructor_narrow_int(self, ty, b).unwrap()
            };
            self.emit(&MInst::AluRRR {
                alu_op: AluOPRRR::Mul,
                rd: rd,
                rs1: a,
                rs2: b,
            });

            self.emit(&MInst::AluRRImm12 {
                alu_op: AluOPRRI::Srli,
                rd,
                rs: rd.to_reg(),
                imm12: Imm12::from_bits(ty.bits() as i16),
            });
        }

        rd.to_reg()
    }

    fn lower_cls(&mut self, val: ValueRegs, ty: Type) -> Reg {
        let tmp = self.temp_writable_reg(I64);
        if ty.bits() != 128 {
            self.emit(&MInst::Cls {
                rs: val.regs()[0],
                rd: tmp,
                ty,
            });
        } else {
            let t0 = self.temp_writable_reg(I64);
            let t1 = self.temp_writable_reg(I64);
            self.emit(&MInst::I128Arithmetic {
                op: I128OP::Cls,
                t0,
                t1,
                dst: vec![tmp],
                x: val,
                y: ValueRegs::two(zero_reg(), zero_reg()), // not used
            });
        }
        tmp.to_reg()
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
mod test {}
