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

    fn lower_float_bnot(&mut self, ty: Type, r: Reg) -> Reg {
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

    fn lower_band_not_i128(&mut self, a: ValueRegs, b: ValueRegs) -> ValueRegs {
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

    fn lower_popcnt_i128(&mut self, val: ValueRegs) -> ValueRegs {
        let low = self.temp_writable_reg(I64);
        let high = self.temp_writable_reg(I64);
        let (op, imm12) = AluOPRRI::Cpop.funct12(None);

        self.emit(&MInst::AluRRImm12 {
            alu_op: op,
            rd: low,
            rs: val.regs()[0],
            imm12,
        });
        self.emit(&MInst::AluRRImm12 {
            alu_op: op,
            rd: high,
            rs: val.regs()[1],
            imm12,
        });
        // add low and high together.
        self.emit(&MInst::AluRRR {
            alu_op: AluOPRRR::Add,
            rd: low,
            rs1: low.to_reg(),
            rs2: high.to_reg(),
        });
        self.emit(&MInst::load_constant_imm12(high, Imm12::from_bits(0)));
        ValueRegs::two(low.to_reg(), high.to_reg())
    }
    fn lower_i128_xnor(&mut self, x: ValueRegs, y: ValueRegs) -> ValueRegs {
        let low = self.temp_writable_reg(I64);
        let high = self.temp_writable_reg(I64);
        self.emit(&MInst::AluRRR {
            alu_op: AluOPRRR::Xnor,
            rd: low,
            rs1: x.regs()[0],
            rs2: y.regs()[0],
        });
        self.emit(&MInst::AluRRR {
            alu_op: AluOPRRR::Xnor,
            rd: high,
            rs1: x.regs()[1],
            rs2: y.regs()[1],
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

    fn lower_float_xnot(&mut self, ty: Type, x: Reg, y: Reg) -> Reg {
        let tmpx = self.temp_writable_reg(I64);
        let tmpy = self.temp_writable_reg(I64);
        let move_to_x_reg_op = if ty == F32 {
            AluOPRR::FmvXW
        } else {
            AluOPRR::FmvXD
        };
        // move to x registers
        self.emit(&MInst::AluRR {
            alu_op: move_to_x_reg_op,
            rd: tmpx,
            rs: x,
        });
        self.emit(&MInst::AluRR {
            alu_op: move_to_x_reg_op,
            rd: tmpy,
            rs: y,
        });
        // xnor
        self.emit(&MInst::AluRRR {
            alu_op: AluOPRRR::Xnor,
            rd: tmpx,
            rs1: tmpx.to_reg(),
            rs2: tmpy.to_reg(),
        });

        // move back to f register
        let move_f_reg_op = if ty == F32 {
            AluOPRR::FmvWX
        } else {
            AluOPRR::FmvDX
        };
        let result_reg = self.temp_writable_reg(ty);
        self.emit(&MInst::AluRR {
            alu_op: move_f_reg_op,
            rd: result_reg,
            rs: tmpx.to_reg(),
        });
        result_reg.to_reg()
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
                        rd: low,
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
                let shamt = {
                    //shift is bigger than it's bits is useless.
                    //get rid of.
                    let shamt = self.temp_writable_reg(I64);
                    self.emit(&MInst::AluRRImm12 {
                        alu_op: AluOPRRI::Andi,
                        rd: shamt,
                        rs: amount,
                        imm12: Imm12::from_bits((ty.bits() - 1) as i16),
                    });
                    shamt.to_reg()
                };
                let len_sub_shamt = {
                    let len_sub_shamt = self.temp_writable_reg(I64);
                    self.emit(&MInst::load_constant_imm12(
                        len_sub_shamt,
                        Imm12::from_bits(ty.bits() as i16),
                    ));
                    self.emit(&MInst::AluRRR {
                        alu_op: AluOPRRR::Sub,
                        rd: len_sub_shamt,
                        rs1: len_sub_shamt.to_reg(),
                        rs2: shamt,
                    });
                    len_sub_shamt.to_reg()
                };
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Sll,
                    rd: rd,
                    rs1: rs,
                    rs2: shamt,
                });
                let value2 = self.temp_writable_reg(I64);
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Srl,
                    rd: value2,
                    rs1: rs,
                    rs2: len_sub_shamt,
                });

                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Or,
                    rd: rd,
                    rs1: rd.to_reg(),
                    rs2: value2.to_reg(),
                });
            }
            _ => unreachable!(),
        }
        rd.to_reg()
    }

    fn lower_rotr(&mut self, ty: Type, rs: Reg, shamt: Reg) -> Reg {
        let rd = self.temp_writable_reg(I64);
        match ty.bits() {
            64 => {
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Ror,
                    rd: rd,
                    rs1: rs,
                    rs2: shamt,
                });
            }
            32 => {
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Rorw,
                    rd: rd,
                    rs1: rs,
                    rs2: shamt,
                });
            }

            16 | 8 => {
                let shamt = {
                    let tmp = self.temp_writable_reg(I64);
                    self.emit(&MInst::AluRRImm12 {
                        alu_op: AluOPRRI::Andi,
                        rd: tmp,
                        rs: shamt,
                        imm12: Imm12::from_bits((ty.bits() - 1) as i16),
                    });
                    tmp.to_reg()
                };
                let len_sub_shamt = {
                    let len_sub_shamt = self.temp_writable_reg(I64);
                    self.emit(&MInst::load_constant_imm12(
                        len_sub_shamt,
                        Imm12::from_bits(ty.bits() as i16),
                    ));
                    self.emit(&MInst::AluRRR {
                        alu_op: AluOPRRR::Sub,
                        rd: len_sub_shamt,
                        rs1: len_sub_shamt.to_reg(),
                        rs2: shamt,
                    });
                    len_sub_shamt.to_reg()
                };
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Srl,
                    rd: rd,
                    rs1: rs,
                    rs2: shamt,
                });
                let value2 = self.temp_writable_reg(I64);
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Sll,
                    rd: value2,
                    rs1: rs,
                    rs2: len_sub_shamt,
                });

                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Or,
                    rd: rd,
                    rs1: rd.to_reg(),
                    rs2: value2.to_reg(),
                });
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

    fn lower_cls(&mut self, val: ValueRegs, ty: Type) -> ValueRegs {
        let tmp = self.temp_writable_reg(I64);
        if ty.bits() != 128 {
            self.emit(&MInst::Cls {
                rs: val.regs()[0],
                rd: tmp,
                ty,
            });
            ValueRegs::one(tmp.to_reg())
        } else {
            let t0 = self.temp_writable_reg(I64);
            let t1 = self.temp_writable_reg(I64);
            let tmp_high = self.temp_writable_reg(I64);
            self.emit(&MInst::I128Arithmetic {
                op: I128OP::Cls,
                t0,
                t1,
                dst: vec![tmp, tmp_high],
                x: val,
                y: ValueRegs::two(zero_reg(), zero_reg()), // not used
            });
            ValueRegs::two(tmp.to_reg(), tmp_high.to_reg())
        }
    }

    // i128 implemetation
    fn lowre_i128_rotate(
        &mut self,
        shift_left: bool,
        val: ValueRegs,
        shamt: ValueRegs,
    ) -> ValueRegs {
        let shamt = {
            let tmp = self.temp_writable_reg(I64);
            self.emit(&MInst::AluRRImm12 {
                alu_op: AluOPRRI::Andi,
                rd: tmp,
                /*
                    todo what is purpose of using i128 as rotate shamt.
                    i128 is too big.....
                */
                rs: shamt.regs()[0],
                imm12: Imm12::from_bits(63),
            });
            tmp.to_reg()
        };

        let xlen_sub_shamt = {
            let tmp = self.temp_writable_reg(I64);
            self.emit(&MInst::load_constant_imm12(tmp, Imm12::from_bits(64)));
            self.emit(&MInst::AluRRR {
                alu_op: AluOPRRR::Sub,
                rd: tmp,
                rs1: tmp.to_reg(),
                rs2: shamt,
            });
            tmp.to_reg()
        };

        let low = self.temp_writable_reg(I64);

        /*
        Rotate Right
            This instruction performs a rotate right of rs1 by the amount in least-significant log2(XLEN)
        Operation
        ~~~
                let shamt = if xlen == 32
                then X(rs2)[4..0]
                else X(rs2)[5..0];
                let result = (X(rs1) >> shamt) | (X(rs1) << (xlen - shamt));
                X(rd) = result;
        ~~~
                first_shift means "(X(rs1) >> shamt)"
                second_shift means " (X(rs1) << (xlen - shamt))"
                        */

        let first_shift = || {
            if shift_left {
                AluOPRRR::Sll
            } else {
                AluOPRRR::Srl
            }
        };
        let second_shift = || {
            if shift_left {
                AluOPRRR::Srl
            } else {
                AluOPRRR::Sll
            }
        };
        // low part
        {
            self.emit(&MInst::AluRRR {
                alu_op: first_shift(),
                rd: low,
                rs1: val.regs()[0],
                rs2: shamt,
            });
            //
            let tmp = self.temp_writable_reg(I64);
            self.emit(&MInst::AluRRR {
                alu_op: second_shift(),
                rd: tmp,
                rs1: val.regs()[1],
                rs2: xlen_sub_shamt,
            });
            // xlen_sub_shamt == 64 srl will overflow. use zero instead .
            self.emit(&MInst::SelectReg {
                rd: tmp,
                rs1: zero_reg(),
                rs2: tmp.to_reg(),
                condition: IntegerCompare {
                    rs1: shamt,
                    rs2: zero_reg(),
                    kind: IntCC::Equal,
                },
            });
            self.emit(&MInst::AluRRR {
                alu_op: AluOPRRR::Or,
                rd: low,
                rs1: low.to_reg(),
                rs2: tmp.to_reg(),
            });
        }

        let high = self.temp_writable_reg(I64);
        // high part
        {
            self.emit(&MInst::AluRRR {
                alu_op: first_shift(),
                rd: high,
                rs1: val.regs()[1],
                rs2: shamt,
            });
            //
            let tmp = self.temp_writable_reg(I64);
            self.emit(&MInst::AluRRR {
                alu_op: second_shift(),
                rd: tmp,
                rs1: val.regs()[0],
                rs2: xlen_sub_shamt,
            });
            // xlen_sub_shamt == 64 srl will overflow. use zero instead .
            self.emit(&MInst::SelectReg {
                rd: tmp,
                rs1: zero_reg(),
                rs2: tmp.to_reg(),
                condition: IntegerCompare {
                    rs1: shamt,
                    rs2: zero_reg(),
                    kind: IntCC::Equal,
                },
            });
            self.emit(&MInst::AluRRR {
                alu_op: AluOPRRR::Or,
                rd: high,
                rs1: high.to_reg(),
                rs2: tmp.to_reg(),
            });
        }
        //
        let constant_64_in_reg = {
            let tmp = self.temp_writable_reg(I64);
            self.emit(&MInst::load_constant_imm12(tmp, Imm12::from_bits(64)));
            tmp.to_reg()
        };

        let shamt_127 = {
            let tmp = self.temp_writable_reg(I64);
            self.emit(&MInst::AluRRImm12 {
                alu_op: AluOPRRI::Andi,
                rd: tmp,
                rs: shamt,
                imm12: Imm12::from_bits(127),
            });
            tmp.to_reg()
        };
        // check if switch low and high.
        let (new_low, new_high) = {
            let new_low = self.temp_writable_reg(I64);
            self.emit(&MInst::SelectReg {
                rd: new_low,
                rs1: low.to_reg(),
                rs2: high.to_reg(),
                condition: IntegerCompare {
                    rs1: shamt_127,
                    rs2: constant_64_in_reg,
                    kind: IntCC::UnsignedLessThan,
                },
            });
            let new_high = self.temp_writable_reg(I64);
            self.emit(&MInst::SelectReg {
                rd: new_high,
                rs1: high.to_reg(),
                rs2: low.to_reg(),
                condition: IntegerCompare {
                    rs1: shamt_127,
                    rs2: constant_64_in_reg,
                    kind: IntCC::UnsignedLessThan,
                },
            });
            (new_low, new_high)
        };

        ValueRegs::two(new_low.to_reg(), new_high.to_reg())
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
}

#[cfg(test)]
mod test {}
