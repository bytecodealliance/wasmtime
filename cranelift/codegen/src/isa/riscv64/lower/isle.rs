//! ISLE integration glue code for riscv64 lowering.

// Pull in the ISLE generated code.
pub mod generated_code;

use self::generated_code::I128OP;

// Types that the generated ISLE code uses via `use super::*`.
use super::{writable_zero_reg, zero_reg, Inst as MInst};

use crate::isa::riscv64::lower_inst::is_valid_atomic_transaction_ty;
use crate::isa::riscv64::settings::Flags as IsaFlags;
use crate::machinst::{isle::*, MachInst, SmallInstVec};
use crate::settings::Flags;

use crate::machinst::{VCodeConstant, VCodeConstantData};
use crate::{
    ir::{
        immediates::*, types::*, AtomicRmwOp, ExternalName, Inst, InstructionData, MemFlags,
        StackSlot, TrapCode, Value, ValueList,
    },
    isa::riscv64::inst::*,
    machinst::{InsnOutput, LowerCtx},
};

use std::boxed::Box;
use std::convert::TryFrom;

use crate::machinst::Reg;

type BoxCallInfo = Box<CallInfo>;
type BoxCallIndInfo = Box<CallIndInfo>;
type BoxExternalName = Box<ExternalName>;

pub(crate) const F32_POS_1: u64 = 0x3f800000;
pub(crate) const F64_POS_1: u64 = 0x3ff0000000000000;

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
    fn vec_regs_to_value_regs(&mut self, val: &VecWritableReg) -> ValueRegs {
        match val.len() {
            1 => ValueRegs::one(val[0].to_reg()),
            2 => ValueRegs::two(val[0].to_reg(), val[1].to_reg()),
            _ => unreachable!(),
        }
    }
    fn vec_writable_clone(&mut self, v: &VecWritableReg) -> VecWritableReg {
        v.clone()
    }
    fn con_vec_writable(&mut self, ty: Type) -> VecWritableReg {
        if ty.is_int() {
            if ty.bits() <= 64 {
                vec![self.temp_writable_reg(I64)]
            } else {
                vec![self.temp_writable_reg(I64), self.temp_writable_reg(I64)]
            }
        } else if ty.is_float() {
            vec![self.temp_writable_reg(ty)]
        } else {
            unimplemented!("ty:{:?}", ty)
        }
    }

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

    fn i128_arithmetic(&mut self, op: &I128OP, x: ValueRegs, y: ValueRegs) -> ValueRegs {
        match *op {
            I128OP::Add => {
                let (low, carry) = {
                    let result = self.temp_writable_reg(I64);
                    self.emit(&MInst::AluRRR {
                        alu_op: AluOPRRR::Add,
                        rd: result,
                        rs1: x.regs()[0],
                        rs2: y.regs()[0],
                    });
                    let carry = self.temp_writable_reg(I64);
                    self.emit(&MInst::AluRRR {
                        alu_op: AluOPRRR::SltU,
                        rd: carry,
                        rs1: result.to_reg(),
                        rs2: x.regs()[0],
                    });

                    (result.to_reg(), carry.to_reg())
                };

                let high = self.temp_writable_reg(I64);
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Add,
                    rd: high,
                    rs1: x.regs()[1],
                    rs2: y.regs()[1],
                });
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Add,
                    rd: high,
                    rs1: high.to_reg(),
                    rs2: carry,
                });
                ValueRegs::two(low, high.to_reg())
            }

            I128OP::Sub => {
                let (low, borrow) = {
                    let result = self.temp_writable_reg(I64);
                    self.emit(&MInst::AluRRR {
                        alu_op: AluOPRRR::Sub,
                        rd: result,
                        rs1: x.regs()[0],
                        rs2: y.regs()[0],
                    });
                    let borrow = self.temp_writable_reg(I64);
                    self.emit(&MInst::AluRRR {
                        alu_op: AluOPRRR::SltU,
                        rd: borrow,
                        rs1: x.regs()[0],
                        rs2: result.to_reg(),
                    });
                    (result.to_reg(), borrow.to_reg())
                };

                let high = self.temp_writable_reg(I64);
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Sub,
                    rd: high,
                    rs1: x.regs()[1],
                    rs2: y.regs()[1],
                });
                self.emit(&MInst::AluRRR {
                    alu_op: AluOPRRR::Sub,
                    rd: high,
                    rs1: high.to_reg(),
                    rs2: borrow,
                });
                ValueRegs::two(low, high.to_reg())
            }
            I128OP::Mul => {
                todo!();
            }
            I128OP::Div => {
                todo!();
            }
            I128OP::Rem => {
                todo!();
            }
        }
    }

    fn gen_default_frm(&mut self) -> OptionFloatRoundingMode {
        None
    }

    fn lower_clz(&mut self, ty: Type, val: ValueRegs) -> Reg {
        assert!(ty.is_int());
        let tmp = self.temp_writable_reg(I64);
        if ty != I128 {
            let rs = val.regs()[0];
            match ty.bits() {
                64 => {
                    let (op, imm12) = (AluOPRRI::Clz, Imm12::zero());
                    self.emit(&MInst::AluRRImm12 {
                        alu_op: op,
                        rd: tmp,
                        rs,
                        imm12,
                    });
                }
                32 => {
                    let (op, imm12) = (AluOPRRI::Clzw, Imm12::zero());
                    self.emit(&MInst::AluRRImm12 {
                        alu_op: op,
                        rd: tmp,
                        rs,
                        imm12,
                    });
                }
                16 | 8 => {
                    let rs = generated_code::constructor_narrow_int(self, ty, rs).unwrap();
                    let (op, imm12) = (AluOPRRI::Clzw, Imm12::zero());
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
        let (op, imm12) = (AluOPRRI::Ctz, Imm12::zero());
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
                let (op, imm12) = (AluOPRRI::Ctz, Imm12::zero());
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
                let (op, imm12) = (AluOPRRI::Bexti, Imm12::from_bits((from_bits - 1) as i16));
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
    fn imm12_const(&mut self, val: i32) -> Imm12 {
        Imm12::maybe_from_u64(val as u64).unwrap()
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

    fn lower_i128_rotate(
        &mut self,
        rotate_left: bool,
        val: ValueRegs,
        shamt: ValueRegs,
    ) -> ValueRegs {
        let shamt_63 = {
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
                rs2: shamt_63,
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
            if rotate_left {
                AluOPRRR::Sll
            } else {
                AluOPRRR::Srl
            }
        };
        let second_shift = || {
            if rotate_left {
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
                rs2: shamt_63,
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
                    rs1: shamt_63,
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
                rs2: shamt_63,
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
                    rs1: shamt_63,
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
                rs: shamt.regs()[0],
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

    fn lower_i128_logical_shift(
        &mut self,
        shift_left: bool,
        is_arithmetic: bool,
        val: ValueRegs,
        shift: ValueRegs,
    ) -> ValueRegs {
        let mut insts = SmallInstVec::new();
        let shift_63 = {
            let tmp = self.temp_writable_reg(I64);
            insts.push(MInst::AluRRImm12 {
                alu_op: AluOPRRI::Andi,
                rd: tmp,
                rs: shift.regs()[0],
                imm12: Imm12::from_bits(63),
            });
            tmp.to_reg()
        };

        let len_sub_shift = {
            let tmp = self.temp_writable_reg(I64);
            insts.extend(MInst::construct_imm_sub_rs(tmp, 64, shift_63));
            tmp.to_reg()
        };

        // low part
        let low = self.temp_writable_reg(I64);
        {
            if shift_left {
                insts.push(MInst::AluRRR {
                    alu_op: AluOPRRR::Sll,
                    rd: low,
                    rs1: val.regs()[0],
                    rs2: shift_63,
                });
            } else {
                insts.push(MInst::AluRRR {
                    alu_op: AluOPRRR::Srl,
                    rd: low,
                    rs1: val.regs()[0],
                    rs2: shift_63,
                });
                let tmp = self.temp_writable_reg(I64);
                insts.push(MInst::AluRRR {
                    alu_op: AluOPRRR::Sll,
                    rd: tmp,
                    rs1: val.regs()[1],
                    rs2: len_sub_shift,
                });
                insts.push(MInst::SelectReg {
                    rd: tmp,
                    rs1: tmp.to_reg(),
                    rs2: zero_reg(),
                    condition: IntegerCompare {
                        kind: IntCC::NotEqual,
                        rs1: zero_reg(),
                        rs2: shift_63,
                    },
                });
                insts.push(MInst::AluRRR {
                    alu_op: AluOPRRR::Or,
                    rd: low,
                    rs1: low.to_reg(),
                    rs2: tmp.to_reg(),
                });
            }
        }
        let high = self.temp_writable_reg(I64);
        {
            if shift_left {
                insts.push(MInst::AluRRR {
                    alu_op: AluOPRRR::Sll,
                    rd: high,
                    rs1: val.regs()[1],
                    rs2: shift_63,
                });
                let tmp = self.temp_writable_reg(I64);
                insts.push(MInst::AluRRR {
                    alu_op: AluOPRRR::Srl,
                    rd: tmp,
                    rs1: val.regs()[0],
                    rs2: len_sub_shift,
                });
                insts.push(MInst::SelectReg {
                    rd: tmp,
                    rs1: tmp.to_reg(),
                    rs2: zero_reg(),
                    condition: IntegerCompare {
                        kind: IntCC::NotEqual,
                        rs1: zero_reg(),
                        rs2: shift_63,
                    },
                });
                insts.push(MInst::AluRRR {
                    alu_op: AluOPRRR::Or,
                    rd: high,
                    rs1: high.to_reg(),
                    rs2: tmp.to_reg(),
                });
            } else {
                insts.push(MInst::AluRRR {
                    alu_op: if is_arithmetic {
                        AluOPRRR::Sra
                    } else {
                        AluOPRRR::Srl
                    },
                    rd: high,
                    rs1: val.regs()[1],
                    rs2: shift_63,
                });
            }
        }

        let condition_shift_less_than_64 = {
            let constant_64 = {
                let tmp = self.temp_writable_reg(I64);
                insts.push(MInst::load_constant_imm12(tmp, Imm12::from_bits(64)));
                tmp.to_reg()
            };
            let shift_127 = {
                let tmp = self.temp_writable_reg(I64);
                insts.push(MInst::AluRRImm12 {
                    alu_op: AluOPRRI::Andi,
                    rd: tmp,
                    rs: shift.regs()[0],
                    imm12: Imm12::from_bits(127),
                });
                tmp.to_reg()
            };
            IntegerCompare {
                rs1: shift_127,
                rs2: constant_64,
                kind: IntCC::UnsignedLessThan,
            }
        };

        {
            // make result
            let new_low = self.temp_writable_reg(I64);
            let new_high = self.temp_writable_reg(I64);
            if shift_left {
                insts.push(MInst::SelectReg {
                    rd: new_low,
                    rs1: low.to_reg(),
                    rs2: zero_reg(),
                    condition: condition_shift_less_than_64,
                });
                insts.push(MInst::SelectReg {
                    rd: new_high,
                    rs1: high.to_reg(),
                    rs2: low.to_reg(),
                    condition: condition_shift_less_than_64,
                });
            } else {
                insts.push(MInst::SelectReg {
                    rd: new_low,
                    rs1: low.to_reg(),
                    rs2: high.to_reg(),
                    condition: condition_shift_less_than_64,
                });
                if !is_arithmetic {
                    insts.push(MInst::SelectReg {
                        rd: new_high,
                        rs1: high.to_reg(),
                        rs2: zero_reg(),
                        condition: condition_shift_less_than_64,
                    });
                } else {
                    let arithmetci_high_value: Reg = {
                        let all1 = self.temp_writable_reg(I64);
                        insts.push(MInst::load_constant_imm12(all1, Imm12::from_bits(-1)));
                        let tmp = self.temp_writable_reg(I64);
                        insts.push(MInst::SelectReg {
                            rd: tmp,

                            rs1: all1.to_reg(),
                            rs2: zero_reg(),
                            condition: IntegerCompare {
                                kind: IntCC::SignedLessThan,
                                rs1: val.regs()[1],
                                rs2: zero_reg(),
                            },
                        });
                        tmp.to_reg()
                    };
                    insts.push(MInst::SelectReg {
                        rd: new_high,
                        rs1: high.to_reg(),
                        rs2: arithmetci_high_value,
                        condition: condition_shift_less_than_64,
                    });
                }
            }
            insts.iter().for_each(|i| self.emit(i));
            ValueRegs::two(new_low.to_reg(), new_high.to_reg())
        }
    }

    fn valueregs_2_reg(&mut self, val: Value) -> Option<Reg> {
        Some(self.put_in_regs(val).regs()[0])
    }

    fn load_float_const(&mut self, val: u64, ty: Type) -> Reg {
        let result = self.temp_writable_reg(ty);
        if ty == F32 {
            MInst::load_fp_constant32(result, val as u32)
                .into_iter()
                .for_each(|i| self.emit(&i));
        } else if ty == F64 {
            MInst::load_fp_constant64(result, val)
                .into_iter()
                .for_each(|i| self.emit(&i));
        } else {
            unimplemented!()
        }
        result.to_reg()
    }
    fn move_f_to_x(&mut self, r: Reg, ty: Type) -> Reg {
        let result = self.temp_writable_reg(I64);
        self.emit(&gen_move(result, I64, r, ty));
        result.to_reg()
    }

    fn move_x_to_f(&mut self, r: Reg, ty: Type) -> Reg {
        let result = self.temp_writable_reg(ty);
        self.emit(&gen_move(result, ty, r, I64));
        result.to_reg()
    }

    fn lower_cls_i128(&mut self, val: ValueRegs) -> ValueRegs {
        // count high part.
        let result = self.temp_writable_reg(I64);
        self.emit(&MInst::Cls {
            rs: val.regs()[1],
            rd: result,
            ty: I64,
        });

        let rest: Reg = {
            let count_low: Reg = {
                let count_positive: Reg = {
                    let tmp = self.temp_writable_reg(I64);
                    let (op, imm12) = (AluOPRRI::Clz, Imm12::zero());
                    self.emit(&MInst::AluRRImm12 {
                        alu_op: op,
                        rd: tmp,
                        rs: val.regs()[0],
                        imm12,
                    });
                    tmp.to_reg()
                };
                let count_negtive: Reg = {
                    let tmp = self.temp_writable_reg(I64);
                    self.emit(&MInst::construct_bit_not(tmp, val.regs()[0]));
                    let (op, imm12) = (AluOPRRI::Clz, Imm12::zero());
                    self.emit(&MInst::AluRRImm12 {
                        alu_op: op,
                        rd: tmp,
                        rs: tmp.to_reg(),
                        imm12,
                    });
                    tmp.to_reg()
                };
                let tmp = self.temp_writable_reg(I64);
                self.emit(&MInst::SelectReg {
                    rd: tmp,
                    rs1: count_negtive,
                    rs2: count_positive,
                    condition: IntegerCompare {
                        kind: IntCC::SignedLessThan,
                        rs1: val.regs()[1],
                        rs2: zero_reg(),
                    },
                });
                tmp.to_reg()
            };

            let const_63_in_reg: Reg = {
                let tmp = self.temp_writable_reg(I64);
                self.emit(&MInst::load_constant_imm12(tmp, Imm12::from_bits(63)));
                tmp.to_reg()
            };

            let tmp = self.temp_writable_reg(I64);
            self.emit(&MInst::SelectReg {
                rd: tmp,
                rs1: count_low,
                rs2: zero_reg(),
                condition: IntegerCompare {
                    /*
                       if we need the low result part.
                    */
                    kind: IntCC::Equal,
                    rs1: const_63_in_reg,
                    rs2: result.to_reg(),
                },
            });
            tmp.to_reg()
        };

        // add rest part.
        self.emit(&MInst::AluRRR {
            alu_op: AluOPRRR::Add,
            rd: result,
            rs1: result.to_reg(),
            rs2: rest,
        });

        let result_high = self.temp_writable_reg(I64);
        self.emit(&MInst::load_constant_imm12(
            result_high,
            Imm12::from_bits(0),
        ));
        ValueRegs::two(result.to_reg(), result_high.to_reg())
    }

    fn pack_float_rounding_mode(&mut self, f: &FRM) -> OptionFloatRoundingMode {
        Some(*f)
    }

    fn con_amode(&mut self, base: Reg, offset: Offset32, ty: Type) -> AMode {
        AMode::RegOffset(base, i64::from(offset), ty)
    }
    fn valid_atomic_transaction(&mut self, ty: Type) -> Option<Type> {
        if is_valid_atomic_transaction_ty(ty) {
            Some(ty)
        } else {
            None
        }
    }
    fn load_op(&mut self, ty: Type) -> LoadOP {
        LoadOP::from_type(ty)
    }
    fn store_op(&mut self, ty: Type) -> StoreOP {
        StoreOP::from_type(ty)
    }
    fn load_ext_name(&mut self, name: ExternalName, offset: i64) -> Reg {
        let tmp = self.temp_writable_reg(I64);
        self.emit(&MInst::LoadExtName {
            rd: tmp,
            name: Box::new(name),
            offset,
        });
        tmp.to_reg()
    }
    fn offset_add(&mut self, a: Offset32, adden: i64) -> Offset32 {
        a.try_add_i64(adden).expect("offset exceed range.")
    }
    fn value_and_type(&mut self, val: Value) -> (Type, Value) {
        let ty = self.lower_ctx.value_ty(val);
        (ty, val)
    }
    fn gen_stack_addr(&mut self, slot: StackSlot, offset: Offset32) -> Reg {
        let result = self.temp_writable_reg(I64);
        let i = self
            .lower_ctx
            .abi()
            .stackslot_addr(slot, i64::from(offset) as u32, result);
        self.emit(&i);
        result.to_reg()
    }
    fn atomic_amo(&mut self) -> AMO {
        AMO::SeqConsistent
    }
    fn gen_move(&mut self, r: Reg, ty: Type) -> Reg {
        let tmp = self.temp_writable_reg(ty);
        self.emit(&MInst::gen_move(tmp, r, ty));
        tmp.to_reg()
    }
    fn con_atomic_load(&mut self, addr: Reg, ty: Type) -> Reg {
        let tmp = self.temp_writable_reg(ty);
        self.emit(&MInst::Atomic {
            addr,
            op: if ty.bits() == 32 {
                AtomicOP::LrW
            } else {
                AtomicOP::LrD
            },
            rd: tmp,
            src: zero_reg(),
            amo: AMO::SeqConsistent,
        });
        tmp.to_reg()
    }
    fn con_atomic_store(&mut self, addr: Reg, ty: Type, src: Reg) -> Reg {
        let tmp = self.temp_writable_reg(ty);
        self.emit(&MInst::Atomic {
            addr,
            op: if ty.bits() == 32 {
                AtomicOP::ScW
            } else {
                AtomicOP::ScD
            },
            rd: tmp,
            src: src,
            amo: AMO::SeqConsistent,
        });
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
}
