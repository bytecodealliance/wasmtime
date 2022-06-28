//! ISLE integration glue code for riscv64 lowering.

// Pull in the ISLE generated code.
pub mod generated_code;

// Types that the generated ISLE code uses via `use super::*`.
use super::{writable_zero_reg, zero_reg, Inst as MInst};

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

    fn gen_ceil(&mut self, rs: Reg, ty: Type) -> Reg {
        unimplemented!()
    }
    fn gen_floor(&mut self, rs: Reg, ty: Type) -> Reg {
        unimplemented!()
    }
    fn gen_trunc(&mut self, rs: Reg, ty: Type) -> Reg {
        unimplemented!()
    }
    fn gen_nearest(&mut self, rs: Reg, ty: Type) -> Reg {
        unimplemented!()
    }

    fn gen_moves(&mut self, rs: ValueRegs, in_ty: Type, out_ty: Type) -> ValueRegs {
        match (in_ty.is_vector(), out_ty.is_vector()) {
            (true, true) => todo!(),
            (true, false) => todo!(),
            (false, true) => todo!(),
            (false, false) => {
                assert!(in_ty.bits() == out_ty.bits());
                match in_ty.bits() {
                    128 => {
                        // if 128 must not be a float.
                        let low = self.temp_writable_reg(out_ty);
                        let high = self.temp_writable_reg(out_ty);
                        self.emit(&gen_move_re_interprete(low, I64, rs.regs()[0], I64));
                        self.emit(&gen_move_re_interprete(high, I64, rs.regs()[1], I64));
                        ValueRegs::two(low.to_reg(), high.to_reg())
                    }
                    _ => {
                        let rd = self.temp_writable_reg(out_ty);
                        self.emit(&gen_move_re_interprete(rd, out_ty, rs.regs()[0], in_ty));
                        ValueRegs::one(rd.to_reg())
                    }
                }
            }
        }
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

    fn gen_default_frm(&mut self) -> OptionFloatRoundingMode {
        None
    }
    fn con_select_reg(&mut self, cc: &IntCC, a: Reg, b: Reg, rs1: Reg, rs2: Reg) -> Reg {
        let rd = self.temp_writable_reg(MInst::canonical_type_for_rc(rs1.class()));
        self.emit(&MInst::SelectReg {
            rd,
            rs1,
            rs2,
            condition: IntegerCompare {
                kind: *cc,
                rs1: a,
                rs2: b,
            },
        });
        rd.to_reg()
    }
    fn load_u64_constant(&mut self, val: u64) -> Reg {
        let rd = self.temp_writable_reg(I64);
        MInst::load_constant_u64(rd, val)
            .iter()
            .for_each(|i| self.emit(i));
        rd.to_reg()
    }
    fn u8_as_i32(&mut self, x: u8) -> i32 {
        x as i32
    }

    fn ext_sign_bit(&mut self, ty: Type, r: Reg) -> Reg {
        assert!(ty.is_int());
        let rd = self.temp_writable_reg(I64);
        self.emit(&MInst::AluRRImm12 {
            alu_op: AluOPRRI::Bexti,
            rd,
            rs: r,
            imm12: Imm12::from_bits((ty.bits() - 1) as i16),
        });
        rd.to_reg()
    }
    fn imm12_const(&mut self, val: i32) -> Imm12 {
        Imm12::maybe_from_u64(val as u64).unwrap()
    }
    fn imm12_const_add(&mut self, val: i32, add: i32) -> Imm12 {
        Imm12::maybe_from_u64((val + add) as u64).unwrap()
    }

    //
    fn con_shamt(&mut self, ty: Type, shamt: Reg) -> ValueRegs {
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
        ValueRegs::two(shamt, len_sub_shamt)
    }

    // fn lower_mlhi(&mut self, is_signed: bool, ty: Type, a: Reg, b: Reg) -> Reg {
    //     let rd = self.temp_writable_reg(I64);
    //     if ty.bits() == 64 {
    //         self.emit(&MInst::AluRRR {
    //             alu_op: if is_signed {
    //                 AluOPRRR::Mulh
    //             } else {
    //                 AluOPRRR::Mulhu
    //             },
    //             rd: rd,
    //             rs1: a,
    //             rs2: b,
    //         });
    //     } else {
    //         /*
    //             first we must narrow down int,because we perform unsinged multiply.
    //         */
    //         let a = if is_signed {
    //             a
    //         } else {
    //             generated_code::constructor_narrow_int(self, ty, a).unwrap()
    //         };
    //         let b = if is_signed {
    //             b
    //         } else {
    //             generated_code::constructor_narrow_int(self, ty, b).unwrap()
    //         };
    //         self.emit(&MInst::AluRRR {
    //             alu_op: AluOPRRR::Mul,
    //             rd: rd,
    //             rs1: a,
    //             rs2: b,
    //         });
    //         self.emit(&MInst::AluRRImm12 {
    //             alu_op: AluOPRRI::Srli,
    //             rd,
    //             rs: rd.to_reg(),
    //             imm12: Imm12::from_bits(ty.bits() as i16),
    //         });
    //     }

    //     rd.to_reg()
    // }

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

    fn valueregs_2_reg(&mut self, val: Value) -> Reg {
        self.put_in_regs(val).regs()[0]
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
        self.emit(&gen_move_re_interprete(result, I64, r, ty));
        result.to_reg()
    }

    fn move_x_to_f(&mut self, r: Reg, ty: Type) -> Reg {
        let result = self.temp_writable_reg(ty);
        self.emit(&gen_move_re_interprete(result, ty, r, I64));
        result.to_reg()
    }

    fn pack_float_rounding_mode(&mut self, f: &FRM) -> OptionFloatRoundingMode {
        Some(*f)
    }
    fn float_convert_2_int_op(&mut self, from: Type, is_signed: bool, to: Type) -> FpuOPRR {
        FpuOPRR::float_convert_2_int_op(from, is_signed, to)
    }
    fn int_convert_2_float_op(&mut self, from: Type, is_signed: bool, to: Type) -> FpuOPRR {
        FpuOPRR::int_convert_2_float_op(from, is_signed, to)
    }
    fn con_amode(&mut self, base: Reg, offset: Offset32, ty: Type) -> AMode {
        AMode::RegOffset(base, i64::from(offset), ty)
    }
    fn valid_atomic_transaction(&mut self, ty: Type) -> Option<Type> {
        if ty == I32 || ty == I64 {
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
    fn offset32_add(&mut self, a: Offset32, adden: i64) -> Offset32 {
        a.try_add_i64(adden).expect("offset exceed range.")
    }
    fn type_and_value(&mut self, val: Value) -> (Type, Value) {
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
        self.emit(&gen_move_re_interprete(tmp, ty, r, ty));
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
