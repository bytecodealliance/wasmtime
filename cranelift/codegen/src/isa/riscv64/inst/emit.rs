//! Riscv64 ISA: binary code emission.

use crate::binemit::StackMap;
use crate::isa::riscv64::inst::*;

use crate::isa::riscv64::inst::{zero_reg, AluOPRRR};
use crate::machinst::{AllocationConsumer, Reg, Writable};

use regalloc2::Allocation;

pub struct EmitInfo(settings::Flags);

impl EmitInfo {
    pub(crate) fn new(flags: settings::Flags) -> Self {
        Self(flags)
    }
}

pub(crate) fn reg_to_gpr_num(m: Reg) -> u32 {
    u32::try_from(m.to_real_reg().unwrap().hw_enc() & 31).unwrap()
}

/// State carried between emissions of a sequence of instructions.
#[derive(Default, Clone, Debug)]
pub struct EmitState {
    pub(crate) virtual_sp_offset: i64,
    pub(crate) nominal_sp_to_fp: i64,
    /// Safepoint stack map for upcoming instruction, as provided to `pre_safepoint()`.
    stack_map: Option<StackMap>,
    /// Current source-code location corresponding to instruction to be emitted.
    cur_srcloc: SourceLoc,
}

impl EmitState {
    fn take_stack_map(&mut self) -> Option<StackMap> {
        self.stack_map.take()
    }

    fn clear_post_insn(&mut self) {
        self.stack_map = None;
    }

    fn cur_srcloc(&self) -> SourceLoc {
        self.cur_srcloc
    }
}

impl MachInstEmitState<Inst> for EmitState {
    fn new(abi: &dyn ABICallee<I = Inst>) -> Self {
        EmitState {
            virtual_sp_offset: 0,
            nominal_sp_to_fp: abi.frame_size() as i64,
            stack_map: None,
            cur_srcloc: SourceLoc::default(),
        }
    }

    fn pre_safepoint(&mut self, stack_map: StackMap) {
        self.stack_map = Some(stack_map);
    }

    fn pre_sourceloc(&mut self, srcloc: SourceLoc) {
        self.cur_srcloc = srcloc;
    }
}

impl Inst {
    pub(crate) fn construct_imm_sub_rs(rd: Writable<Reg>, imm: u64, rs: Reg) -> SmallInstVec<Inst> {
        let mut insts = Inst::load_constant_u64(rd, imm);
        insts.push(Inst::AluRRR {
            alu_op: AluOPRRR::Sub,
            rd,
            rs1: rd.to_reg(),
            rs2: rs,
        });
        insts
    }

    /*
        inverse all bit
    */
    pub(crate) fn construct_bit_not(rd: Writable<Reg>, rs: Reg) -> Inst {
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Xori,
            rd,
            rs,
            imm12: Imm12::from_bits(-1),
        }
    }

    pub(crate) fn narrow_down_int(rd: Writable<Reg>, rs: Reg, ty: Type) -> SmallInstVec<Inst> {
        assert!(ty.bits() < 64);
        assert!(ty.is_int());
        let mut insts = SmallInstVec::new();
        let shift = (64 - ty.bits()) as i16;
        insts.push(Inst::AluRRImm12 {
            alu_op: AluOPRRI::Slli,
            rd: rd,
            rs: rs,
            imm12: Imm12::from_bits(shift),
        });
        insts.push(Inst::AluRRImm12 {
            alu_op: AluOPRRI::Srli,
            rd: rd,
            rs: rd.to_reg(),
            imm12: Imm12::from_bits(shift),
        });
        insts
    }

    pub(crate) fn lower_br_fcmp(
        cc: FloatCC,
        x: Reg,
        y: Reg,
        taken: BranchTarget,
        not_taken: BranchTarget,
        ty: Type,
        tmp: Writable<Reg>,
        tmp2: Writable<Reg>,
    ) -> SmallInstVec<Inst> {
        let mut insts = SmallInstVec::new();
        let cc_bit = FloatCCBit::floatcc_2_mask_bits(cc);
        let eq_op = if ty == F32 {
            FpuOPRRR::FeqS
        } else {
            FpuOPRRR::FeqD
        };
        let lt_op = if ty == F32 {
            FpuOPRRR::FltS
        } else {
            FpuOPRRR::FltD
        };
        // if eq
        if cc_bit.constains(FloatCCBit::EQ) {
            insts.push(Inst::FpuRRR {
                frm: None,
                alu_op: eq_op,
                rd: tmp,
                rs1: x,
                rs2: y,
            });
            insts.push(Inst::CondBr {
                taken: taken,
                not_taken: BranchTarget::zero(),
                kind: IntegerCompare {
                    kind: IntCC::NotEqual,
                    rs1: tmp.to_reg(),
                    rs2: zero_reg(),
                },
            });
        }
        // if <
        if cc_bit.constains(FloatCCBit::LT) {
            insts.push(Inst::FpuRRR {
                frm: None,
                alu_op: lt_op,
                rd: tmp,
                rs1: x,
                rs2: y,
            });
            insts.push(Inst::CondBr {
                taken: taken,
                not_taken: BranchTarget::zero(),
                kind: IntegerCompare {
                    kind: IntCC::NotEqual,
                    rs1: tmp.to_reg(),
                    rs2: zero_reg(),
                },
            });
        }
        // if gt
        if cc_bit.constains(FloatCCBit::GT) {
            insts.push(Inst::FpuRRR {
                frm: None,
                alu_op: lt_op,
                rd: tmp,
                rs1: y, //
                rs2: x,
            });
            insts.push(Inst::CondBr {
                taken,
                not_taken: BranchTarget::zero(),
                kind: IntegerCompare {
                    kind: IntCC::NotEqual,
                    rs1: tmp.to_reg(),
                    rs2: zero_reg(),
                },
            });
        }
        // if unorder
        if cc_bit.constains(FloatCCBit::UN) {
            insts.extend(Inst::lower_float_unordered(
                tmp, tmp2, ty, x, y, taken, not_taken,
            ));
        }
        insts
    }
    pub(crate) fn lower_br_icmp(
        cc: IntCC,
        a: ValueRegs<Reg>,
        b: ValueRegs<Reg>,
        taken: BranchTarget,
        not_taken: BranchTarget,
        ty: Type,
    ) -> SmallInstVec<Inst> {
        let mut insts = SmallInstVec::new();
        if ty.bits() <= 64 {
            let rs1 = a.only_reg().unwrap();
            let rs2 = b.only_reg().unwrap();
            let inst = Inst::CondBr {
                taken,
                not_taken,
                kind: IntegerCompare { kind: cc, rs1, rs2 },
            };
            insts.push(inst);
            return insts;
        }

        let low = |cc: IntCC| -> IntegerCompare {
            IntegerCompare {
                rs1: a.regs()[0],
                rs2: b.regs()[0],
                kind: cc,
            }
        };
        let high = |cc: IntCC| -> IntegerCompare {
            IntegerCompare {
                rs1: a.regs()[1],
                rs2: b.regs()[1],
                kind: cc,
            }
        };

        fn remove_eq(cc: IntCC) -> IntCC {
            match cc {
                IntCC::SignedGreaterThanOrEqual => IntCC::SignedGreaterThan,
                IntCC::SignedLessThanOrEqual => IntCC::SignedLessThan,
                IntCC::UnsignedGreaterThanOrEqual => IntCC::UnsignedGreaterThan,
                IntCC::UnsignedLessThanOrEqual => IntCC::UnsignedLessThan,
                _ => cc,
            }
        }

        fn remove_signed(cc: IntCC) -> IntCC {
            let x = match cc {
                IntCC::SignedLessThan => IntCC::UnsignedLessThan,
                IntCC::SignedGreaterThanOrEqual => IntCC::UnsignedGreaterThanOrEqual,
                IntCC::SignedGreaterThan => IntCC::UnsignedGreaterThan,
                IntCC::SignedLessThanOrEqual => IntCC::UnsignedLessThanOrEqual,
                _ => cc,
            };
            x
        }
        match cc {
            IntCC::Equal => {
                /*
                    if high part not equal,
                    then we can go to not_taken otherwise fallthrough.
                */
                insts.push(Inst::CondBr {
                    taken: not_taken,
                    not_taken: BranchTarget::zero(), /*  no branch  */
                    kind: high(IntCC::NotEqual),
                });
                /*
                    the rest part.
                */
                insts.push(Inst::CondBr {
                    taken,
                    not_taken,
                    kind: low(IntCC::Equal),
                });
            }

            IntCC::NotEqual => {
                /*
                    if the high part not equal ,
                    we know the whole must be not equal,
                    we can goto the taken part , otherwise fallthrought.
                */
                insts.push(Inst::CondBr {
                    taken,
                    not_taken: BranchTarget::zero(), /*  no branch  */
                    kind: high(IntCC::NotEqual),
                });

                insts.push(Inst::CondBr {
                    taken,
                    not_taken,
                    kind: low(IntCC::NotEqual),
                });
            }
            IntCC::SignedGreaterThanOrEqual
            | IntCC::SignedLessThanOrEqual
            | IntCC::UnsignedGreaterThanOrEqual
            | IntCC::UnsignedLessThanOrEqual
            | IntCC::SignedGreaterThan
            | IntCC::SignedLessThan
            | IntCC::UnsignedLessThan
            | IntCC::UnsignedGreaterThan => {
                //
                insts.push(Inst::CondBr {
                    taken,
                    not_taken: BranchTarget::zero(),
                    kind: high(remove_eq(cc)),
                });
                //
                insts.push(Inst::CondBr {
                    taken: not_taken,
                    not_taken: BranchTarget::zero(),
                    kind: high(IntCC::NotEqual),
                });
                insts.push(Inst::CondBr {
                    taken,
                    not_taken,
                    kind: low(remove_signed(cc)),
                });
            }
            IntCC::Overflow | IntCC::NotOverflow => unreachable!(),
        }
        insts
    }

    /*
        check if float is unordered.
    */
    pub(crate) fn lower_float_unordered(
        tmp: Writable<Reg>,
        tmp2: Writable<Reg>,
        ty: Type,
        x: Reg,
        y: Reg,
        taken: BranchTarget,
        not_taken: BranchTarget,
    ) -> SmallInstVec<Inst> {
        let mut insts = SmallInstVec::new();
        let class_op = if ty == F32 {
            FpuOPRR::FclassS
        } else {
            FpuOPRR::FclassD
        };
        // if x is nan
        insts.push(Inst::FpuRR {
            frm: None,
            alu_op: class_op,
            rd: tmp,
            rs: x,
        });
        insts.push(Inst::AluRRImm12 {
            alu_op: AluOPRRI::Andi,
            rd: tmp,
            rs: tmp.to_reg(),
            imm12: Imm12::from_bits(FClassResult::is_nan_bits() as i16),
        });
        insts.push(Inst::CondBr {
            taken,
            not_taken: BranchTarget::zero(),
            kind: IntegerCompare {
                kind: IntCC::NotEqual,
                rs1: tmp.to_reg(),
                rs2: zero_reg(),
            },
        });
        // if y is nan.
        insts.push(Inst::FpuRR {
            frm: None,
            alu_op: class_op,
            rd: tmp2,
            rs: y,
        });
        insts.push(Inst::AluRRImm12 {
            alu_op: AluOPRRI::Andi,
            rd: tmp2,
            rs: tmp2.to_reg(),
            imm12: Imm12::from_bits(FClassResult::is_nan_bits() as i16),
        });

        insts.push(Inst::CondBr {
            taken,
            not_taken: BranchTarget::zero(),
            kind: IntegerCompare {
                kind: IntCC::NotEqual,
                rs1: tmp2.to_reg(),
                rs2: zero_reg(),
            },
        });
        // x and y is not nan
        // but there are maybe bother PosInfinite or NegInfinite.
        insts.push(Inst::AluRRR {
            alu_op: AluOPRRR::And,
            rd: tmp,
            rs1: tmp.to_reg(),
            rs2: tmp2.to_reg(),
        });
        insts.push(Inst::AluRRImm12 {
            alu_op: AluOPRRI::Andi,
            rd: tmp,
            rs: tmp.to_reg(),
            imm12: Imm12::from_bits(FClassResult::is_infinite_bits() as i16),
        });
        insts.push(Inst::CondBr {
            taken,
            not_taken,
            kind: IntegerCompare {
                kind: IntCC::NotEqual,
                rs1: tmp.to_reg(),
                rs2: zero_reg(),
            },
        });
        insts
    }

    /*
        1: alloc registers
        2: push into the stack
        3: do something with these registers
        4: restore the registers
    */
    pub(crate) fn do_something_with_registers(
        num: usize,
        mut f: impl std::ops::FnMut(&std::vec::Vec<Writable<Reg>>, &mut SmallInstVec<Inst>),
    ) -> SmallInstVec<Inst> {
        let mut insts = SmallInstVec::new();
        let registers = Self::alloc_registers(num);
        insts.extend(Self::push_registers(&registers));
        f(&registers, &mut insts);
        insts.extend(Self::pop_registers(&registers));
        insts
    }

    /*
        alloc some registers for load large constant, or something else.
    */
    fn alloc_registers(amount: usize) -> Vec<Writable<Reg>> {
        let mut v = vec![];
        let available = bunch_of_normal_registers();
        debug_assert!(amount <= available.len());
        for r in available {
            v.push(r);
            if v.len() == amount {
                return v;
            }
        }
        unreachable!("no enough registers");
    }

    // store a list of regisrer
    fn push_registers(registers: &Vec<Writable<Reg>>) -> SmallInstVec<Inst> {
        let mut insts = smallvec![];
        // ajust sp ; alloc space
        insts.push(Inst::AjustSp {
            amount: -(WORD_SIZE as i64) * (registers.len() as i64),
        });
        //
        let mut cur_offset = 0;
        for r in registers {
            insts.push(Inst::Store {
                // unwrap can check this must be exceed imm12
                to: AMode::SPOffset(
                    Imm12::maybe_from_u64(cur_offset).unwrap().as_u32() as i64,
                    I64,
                ),
                op: StoreOP::Sd,
                src: r.to_reg(),
                flags: MemFlags::new(),
            });
            cur_offset += WORD_SIZE as u64
        }
        insts
    }

    // restore a list of register
    fn pop_registers(registers: &Vec<Writable<Reg>>) -> SmallInstVec<Inst> {
        let mut insts = smallvec![];
        let mut cur_offset = 0;
        for r in registers {
            insts.push(Inst::Load {
                from: AMode::SPOffset(Imm12::maybe_from_u64(cur_offset).unwrap().into(), I64),
                op: LoadOP::Ld,
                rd: r.clone(),
                flags: MemFlags::new(),
            });
            cur_offset += WORD_SIZE as u64
        }
        // restore sp
        insts.push(Inst::AjustSp {
            amount: cur_offset as i64,
        });
        insts
    }
}

impl MachInstEmit for Inst {
    type State = EmitState;
    type Info = EmitInfo;

    fn emit(
        &self,
        allocs: &[Allocation],
        sink: &mut MachBuffer<Inst>,
        emit_info: &Self::Info,
        state: &mut EmitState,
    ) {
        let mut allocs = AllocationConsumer::new(allocs);
        // N.B.: we *must* not exceed the "worst-case size" used to compute
        // where to insert islands, except when islands are explicitly triggered
        // (with an `EmitIsland`). We check this in debug builds. This is `mut`
        // to allow disabling the check for `JTSequence`, which is always
        // emitted following an `EmitIsland`.
        let mut start_off = sink.cur_offset();
        match self {
            &Inst::Nop0 => {
                // do nothing
            }
            // Addi x0, x0, 0
            &Inst::Nop4 => {
                let x = Inst::AluRRImm12 {
                    alu_op: AluOPRRI::Addi,
                    rd: Writable::from_reg(zero_reg()),
                    rs: zero_reg(),
                    imm12: Imm12::zero(),
                };
                x.emit(&[], sink, emit_info, state)
            }

            &Inst::Lui { rd, ref imm } => {
                let rd = allocs.next_writable(rd);
                let x: u32 = 0b0110111 | reg_to_gpr_num(rd.to_reg()) << 7 | (imm.as_u32() << 12);
                sink.put4(x);
            }
            &Inst::FpuRR {
                frm,
                alu_op,
                rd,
                rs,
            } => {
                let rs = allocs.next(rs);
                let rd = allocs.next_writable(rd);
                let x = alu_op.op_code()
                    | reg_to_gpr_num(rd.to_reg()) << 7
                    | alu_op.funct3(frm) << 12
                    | reg_to_gpr_num(rs) << 15
                    | alu_op.rs2_funct5() << 20
                    | alu_op.funct7() << 25;
                sink.put4(x);
            }
            &Inst::FpuRRRR {
                alu_op,
                rd,
                rs1,
                rs2,
                rs3,
                frm,
            } => {
                let rs1 = allocs.next(rs1);
                let rs2 = allocs.next(rs2);
                let rs3 = allocs.next(rs3);
                let rd = allocs.next_writable(rd);
                let x = alu_op.op_code()
                    | reg_to_gpr_num(rd.to_reg()) << 7
                    | alu_op.funct3(frm) << 12
                    | reg_to_gpr_num(rs1) << 15
                    | reg_to_gpr_num(rs2) << 20
                    | alu_op.funct2() << 25
                    | reg_to_gpr_num(rs3) << 27;

                sink.put4(x);
            }
            &Inst::FpuRRR {
                alu_op,
                frm,
                rd,
                rs1,
                rs2,
            } => {
                let rs1 = allocs.next(rs1);
                let rs2 = allocs.next(rs2);
                let rd = allocs.next_writable(rd);
                let x: u32 = alu_op.op_code()
                    | reg_to_gpr_num(rd.to_reg()) << 7
                    | (alu_op.funct3(frm)) << 12
                    | reg_to_gpr_num(rs1) << 15
                    | reg_to_gpr_num(rs2) << 20
                    | alu_op.funct7() << 25;
                sink.put4(x);
            }

            &Inst::AluRRR {
                alu_op,
                rd,
                rs1,
                rs2,
            } => {
                let rs1 = allocs.next(rs1);
                let rs2 = allocs.next(rs2);
                let rd = allocs.next_writable(rd);
                let x: u32 = alu_op.op_code()
                    | reg_to_gpr_num(rd.to_reg()) << 7
                    | (alu_op.funct3()) << 12
                    | reg_to_gpr_num(rs1) << 15
                    | reg_to_gpr_num(rs2) << 20
                    | alu_op.funct7() << 25;
                sink.put4(x);
            }
            &Inst::AluRRImm12 {
                alu_op,
                rd,
                rs,
                imm12,
            } => {
                let rs = allocs.next(rs);
                let rd = allocs.next_writable(rd);
                let x = if let Some(funct6) = alu_op.option_funct6() {
                    alu_op.op_code()
                        | reg_to_gpr_num(rd.to_reg()) << 7
                        | alu_op.funct3() << 12
                        | reg_to_gpr_num(rs) << 15
                        | (imm12.as_u32()) << 20
                        | funct6 << 26
                } else if let Some(funct7) = alu_op.option_funct7() {
                    alu_op.op_code()
                        | reg_to_gpr_num(rd.to_reg()) << 7
                        | alu_op.funct3() << 12
                        | reg_to_gpr_num(rs) << 15
                        | (imm12.as_u32()) << 20
                        | funct7 << 25
                } else {
                    alu_op.op_code()
                        | reg_to_gpr_num(rd.to_reg()) << 7
                        | alu_op.funct3() << 12
                        | reg_to_gpr_num(rs) << 15
                        | (imm12.as_u32()) << 20
                };
                sink.put4(x);
            }
            &Inst::Load {
                rd,
                op,
                from,
                flags,
            } => {
                let x;
                let base = from.get_base_register();
                let base = allocs.next(base);
                let rd = allocs.next_writable(rd);
                let offset = from.get_offset_with_state(state);
                if let Some(imm12) = Imm12::maybe_from_u64(offset as u64) {
                    x = op.op_code()
                        | reg_to_gpr_num(rd.to_reg()) << 7
                        | op.funct3() << 12
                        | reg_to_gpr_num(base) << 15
                        | (imm12.as_u32()) << 20;
                    sink.put4(x);
                } else {
                    Inst::do_something_with_registers(1, |registers, insts| {
                        insts.extend(Inst::load_constant_u64(registers[0], offset as u64));
                        insts.push(Inst::AluRRR {
                            alu_op: AluOPRRR::Add,
                            rd: registers[0],
                            rs1: registers[0].to_reg(),
                            rs2: base,
                        });
                        insts.push(Inst::Load {
                            op,
                            from: AMode::RegOffset(
                                registers[0].to_reg(),
                                Imm12::zero().into(),
                                I64,
                            ),
                            rd,
                            flags,
                        });
                    })
                    .into_iter()
                    .for_each(|inst| inst.emit(&[], sink, emit_info, state));
                }
            }
            &Inst::Store { op, src, flags, to } => {
                let base = to.get_base_register();
                let base = allocs.next(base);
                let src = allocs.next(src);
                let offset = to.get_offset_with_state(state);
                let x;
                if let Some(imm12) = Imm12::maybe_from_u64(offset as u64) {
                    x = op.op_code()
                        | (imm12.as_u32() & 0x1f) << 7
                        | op.funct3() << 12
                        | reg_to_gpr_num(base) << 15
                        | reg_to_gpr_num(src) << 20
                        | (imm12.as_u32() >> 5) << 25;
                    sink.put4(x);
                } else {
                    Inst::do_something_with_registers(1, |registers, insts| {
                        insts.extend(Inst::load_constant_u64(registers[0], offset as u64));
                        // registers[0] = base + offset
                        insts.push(Inst::AluRRR {
                            alu_op: AluOPRRR::Add,
                            rd: registers[0],
                            rs1: registers[0].to_reg(),
                            rs2: base,
                        });
                        // st registers[0] , src
                        insts.push(Inst::Store {
                            op,
                            to: AMode::RegOffset(
                                registers[0].to_reg(),
                                Imm12::zero().as_i16() as i64,
                                I64,
                            ),
                            src,
                            flags,
                        });
                    })
                    .into_iter()
                    .for_each(|inst| inst.emit(&[], sink, emit_info, state));
                }
            }
            &Inst::EpiloguePlaceholder => {
                // Noop; this is just a placeholder for epilogues.
            }
            &Inst::ReferenceCheck { rd, op, x } => {
                let x = allocs.next(x);
                let rd = allocs.next_writable(rd);
                let mut insts = SmallInstVec::new();
                match op {
                    ReferenceCheckOP::IsNull => {
                        insts.push(Inst::CondBr {
                            taken: BranchTarget::ResolvedOffset(Inst::instruction_size() * 3),
                            not_taken: BranchTarget::zero(),
                            kind: IntegerCompare {
                                kind: IntCC::Equal,
                                rs1: zero_reg(),
                                rs2: x,
                            },
                        });
                        // here is false
                        insts.push(Inst::load_constant_imm12(rd, Imm12::form_bool(false)));
                        insts.push(Inst::Jal {
                            dest: BranchTarget::ResolvedOffset(Inst::instruction_size() * 2),
                        });
                        // here is true
                        insts.push(Inst::load_constant_imm12(rd, Imm12::form_bool(true)));
                    }

                    ReferenceCheckOP::IsInvalid => {
                        /*
                            todo:: right now just check if it is null
                            null is a valid reference??????
                        */
                        insts.push(Inst::CondBr {
                            taken: BranchTarget::ResolvedOffset(Inst::instruction_size() * 3),
                            not_taken: BranchTarget::zero(),
                            kind: IntegerCompare {
                                kind: IntCC::Equal,
                                rs1: zero_reg(),
                                rs2: x,
                            },
                        });
                        // here is false
                        insts.push(Inst::load_constant_imm12(rd, Imm12::form_bool(false)));
                        insts.push(Inst::Jal {
                            dest: BranchTarget::ResolvedOffset(Inst::instruction_size() * 2),
                        });
                        // here is true
                        insts.push(Inst::load_constant_imm12(rd, Imm12::form_bool(true)));
                    }
                }

                insts
                    .into_iter()
                    .for_each(|i| i.emit(&[], sink, emit_info, state));
            }
            &Inst::Ret { .. } => {
                //jalr x0, x1, 0
                let x: u32 = (0b1100111) | (1 << 15);
                sink.put4(x);
            }

            &Inst::Extend {
                rd,
                rn,
                signed,
                from_bits,
                to_bits: _to_bits,
            } => {
                let rn = allocs.next(rn);
                let rd = allocs.next_writable(rd);
                let mut insts = SmallInstVec::new();
                if signed {
                    match from_bits {
                        1 => {
                            insts.push(Inst::CondBr {
                                taken: BranchTarget::offset(Inst::instruction_size() * 3),
                                not_taken: BranchTarget::zero(),
                                kind: IntegerCompare {
                                    rs1: rn,
                                    rs2: zero_reg(),
                                    kind: IntCC::NotEqual,
                                },
                            });
                            insts.push(Inst::load_constant_imm12(rd, Imm12::from_bits(0)));
                            insts.push(Inst::Jal {
                                dest: BranchTarget::offset(Inst::instruction_size() * 2),
                            });
                            insts.push(Inst::load_constant_imm12(rd, Imm12::from_bits(-1)));
                        }
                        8 => {
                            let (op, imm12) = AluOPRRI::Sextb.funct12(None);
                            insts.push(Inst::AluRRImm12 {
                                alu_op: op,
                                rd,
                                rs: rn,
                                imm12,
                            });
                        }
                        16 => {
                            let (op, imm12) = AluOPRRI::Sexth.funct12(None);
                            insts.push(Inst::AluRRImm12 {
                                alu_op: op,
                                rd,
                                rs: rn,
                                imm12,
                            });
                        }
                        32 => {
                            let label_signed = sink.get_label();
                            insts.push(Inst::CondBr {
                                taken: BranchTarget::Label(label_signed),
                                not_taken: BranchTarget::zero(),
                                kind: IntegerCompare {
                                    kind: IntCC::SignedLessThan,
                                    rs1: rn,
                                    rs2: zero_reg(),
                                },
                            });
                            insts.push(Inst::AluRRImm12 {
                                alu_op: AluOPRRI::Slli,
                                rd,
                                rs: rn,
                                imm12: Imm12::from_bits(32),
                            });
                            insts.push(Inst::AluRRImm12 {
                                alu_op: AluOPRRI::Srli,
                                rd,
                                rs: rd.to_reg(),
                                imm12: Imm12::from_bits(32),
                            });
                            let label_jump_over = sink.get_label();
                            // here are zero extend.
                            insts.push(Inst::Jal {
                                dest: BranchTarget::Label(label_jump_over),
                            });

                            insts
                                .drain(..)
                                .for_each(|i| i.emit(&[], sink, emit_info, state));
                            sink.bind_label(label_signed);
                            insts.push(Inst::load_constant_imm12(rd, Imm12::from_bits(-1)));
                            insts.push(Inst::AluRRImm12 {
                                alu_op: AluOPRRI::Slli,
                                rd,
                                rs: rd.to_reg(),
                                imm12: Imm12::from_bits(32),
                            });
                            insts.push(Inst::AluRRR {
                                alu_op: AluOPRRR::Or,
                                rd,
                                rs1: rd.to_reg(),
                                rs2: rn,
                            });
                            insts
                                .drain(..)
                                .for_each(|i| i.emit(&[], sink, emit_info, state));
                            sink.bind_label(label_jump_over);
                        }
                        _ => unreachable!("from_bits:{}", from_bits),
                    }
                } else {
                    let shift_bits = (64 - from_bits) as i16;
                    insts.push(Inst::AluRRImm12 {
                        alu_op: AluOPRRI::Slli,
                        rd,
                        rs: rn,
                        imm12: Imm12::from_bits(shift_bits),
                    });
                    insts.push(Inst::AluRRImm12 {
                        alu_op: AluOPRRI::Srli,
                        rd,
                        rs: rd.to_reg(),
                        imm12: Imm12::from_bits(shift_bits),
                    });
                }
                insts
                    .into_iter()
                    .for_each(|i| i.emit(&[], sink, emit_info, state));
            }
            &Inst::AjustSp { amount } => {
                if let Some(imm) = Imm12::maybe_from_u64(amount as u64) {
                    Inst::AluRRImm12 {
                        alu_op: AluOPRRI::Addi,
                        rd: writable_stack_reg(),
                        rs: stack_reg(),
                        imm12: imm,
                    }
                    .emit(&[], sink, emit_info, state);
                } else {
                    Inst::do_something_with_registers(1, |registers, insts| {
                        insts.extend(Inst::load_constant_u64(registers[0], amount as u64));
                        insts.push(Inst::AluRRR {
                            alu_op: AluOPRRR::Add,
                            rd: writable_stack_reg(),
                            rs1: stack_reg(),
                            rs2: registers[0].to_reg(),
                        });
                    })
                    .into_iter()
                    .for_each(|inst| {
                        inst.emit(&[], sink, emit_info, state);
                    });
                }
            }

            &Inst::Call { ref info } => {
                // call
                match info.dest {
                    ExternalName::User { .. } => {
                        let srcloc = state.cur_srcloc();
                        if info.opcode.is_call() {
                            sink.add_call_site(srcloc, info.opcode);
                        }
                        sink.add_reloc(srcloc, Reloc::RiscvCall, &info.dest, 0);
                        if let Some(s) = state.take_stack_map() {
                            sink.add_stack_map(StackMapExtent::UpcomingBytes(8), s);
                        }
                        Inst::construct_auipc_and_jalr(writable_link_reg(), 0)
                            .into_iter()
                            .for_each(|i| i.emit(&[], sink, emit_info, state));
                    }
                    ExternalName::TestCase { .. } => todo!(),
                    ExternalName::LibCall(..) => todo!(),
                }
            }
            &Inst::CallInd { ref info } => {
                let rn = allocs.next(info.rn);
                if let Some(s) = state.take_stack_map() {
                    sink.add_stack_map(StackMapExtent::UpcomingBytes(4), s);
                }
                let loc = state.cur_srcloc();
                if info.opcode.is_call() {
                    sink.add_call_site(loc, info.opcode);
                }
                Inst::Jalr {
                    rd: writable_link_reg(),
                    base: rn,
                    offset: Imm12::zero(),
                }
                .emit(&[], sink, emit_info, state);
            }

            &Inst::Jal { dest } => {
                let code: u32 = 0b1101111;
                match dest {
                    BranchTarget::Label(lable) => {
                        sink.use_label_at_offset(start_off, lable, LabelUse::Jal20);
                        sink.add_uncond_branch(start_off, start_off + 4, lable);
                        sink.put4(code);
                    }
                    BranchTarget::ResolvedOffset(offset) => {
                        if offset != 0 {
                            if LabelUse::Jal20.offset_in_range(offset) {
                                let mut code = code.to_le_bytes();
                                LabelUse::Jal20.patch_raw_offset(&mut code, offset);
                                sink.put_data(&code[..]);
                            } else {
                                Inst::construct_auipc_and_jalr(writable_spilltmp_reg(), offset)
                                    .into_iter()
                                    .for_each(|i| i.emit(&[], sink, emit_info, state));
                            }
                        }
                    }
                }
            }
            &Inst::CondBr {
                taken,
                not_taken,
                kind,
            } => {
                let mut kind = kind;
                kind.rs1 = allocs.next(kind.rs1);
                kind.rs2 = allocs.next(kind.rs2);
                match taken {
                    BranchTarget::Label(label) => {
                        let code = kind.emit();
                        let code_inverse = kind.inverse().emit().to_le_bytes();
                        sink.use_label_at_offset(start_off, label, LabelUse::B12);
                        sink.add_cond_branch(start_off, start_off + 4, label, &code_inverse);
                        sink.put4(code);
                    }
                    BranchTarget::ResolvedOffset(offset) => {
                        if LabelUse::B12.offset_in_range(offset) {
                            let code = kind.emit();
                            let mut code = code.to_le_bytes();
                            LabelUse::B12.patch_raw_offset(&mut code, offset);
                            sink.put_data(&code[..])
                        } else {
                            let mut code = kind.emit().to_le_bytes();
                            /*
                                jump over the condbr , 4 bytes.
                            */
                            LabelUse::B12.patch_raw_offset(&mut code[..], 4);
                            sink.put_data(&code[..]);
                            Inst::construct_auipc_and_jalr(writable_spilltmp_reg(), offset)
                                .into_iter()
                                .for_each(|i| i.emit(&[], sink, emit_info, state));
                        }
                    }
                }
                Inst::Jal { dest: not_taken }.emit(&[], sink, emit_info, state);
            }

            &Inst::Mov { rd, rm, ty } => {
                if rd.to_reg() != rm {
                    let rm = allocs.next(rm);
                    let rd = allocs.next_writable(rd);
                    if ty.is_float() {
                        let mut insts = SmallInstVec::new();
                        insts.push(Inst::FpuRR {
                            frm: None,
                            alu_op: FpuOPRR::move_f_to_x_op(ty),
                            rd: writable_spilltmp_reg(),
                            rs: rm,
                        });
                        insts.push(Inst::FpuRR {
                            frm: None,
                            alu_op: FpuOPRR::move_x_to_f_op(ty),
                            rd: rd,
                            rs: spilltmp_reg(),
                        });
                        insts
                            .into_iter()
                            .for_each(|inst| inst.emit(&[], sink, emit_info, state));
                    } else {
                        let x = Inst::AluRRImm12 {
                            alu_op: AluOPRRI::Ori,
                            rd: rd,
                            rs: rm,
                            imm12: Imm12::zero(),
                        };
                        x.emit(&[], sink, emit_info, state);
                    }
                }
            }
            &Inst::BrTable {
                index,
                tmp1,
                default_,
                ref targets,
            } => {
                let index = allocs.next(index);
                let tmp1 = allocs.next_writable(tmp1);
                let mut insts = SmallInstVec::new();
                {
                    /*
                        if index not match all targets,
                        goto the default.
                    */
                    // index < 0
                    insts.push(Inst::CondBr {
                        taken: default_,
                        not_taken: BranchTarget::zero(),
                        kind: IntegerCompare {
                            kind: IntCC::SignedLessThan,
                            rs1: index,
                            rs2: zero_reg(),
                        },
                    });
                    //index >= targets.len()
                    insts.extend(Inst::load_constant_u64(tmp1, targets.len() as u64));
                    insts.push(Inst::CondBr {
                        taken: default_,
                        not_taken: BranchTarget::zero(),
                        kind: IntegerCompare {
                            kind: IntCC::UnsignedGreaterThanOrEqual,
                            rs1: index,
                            rs2: tmp1.to_reg(),
                        },
                    });
                }
                {
                    let x = Inst::construct_auipc_and_jalr(
                        tmp1, 16, /*  for auipc slli add and jalr.   */
                    );
                    insts.push(x[0].clone());
                    // t *= 8;
                    insts.push(Inst::AluRRImm12 {
                        alu_op: AluOPRRI::Slli,
                        rd: writable_spilltmp_reg(),
                        rs: index,
                        imm12: Imm12::from_bits(3),
                    });
                    // tmp1 += t
                    insts.push(Inst::AluRRR {
                        alu_op: AluOPRRR::Add,
                        rd: tmp1,
                        rs1: tmp1.to_reg(),
                        rs2: spilltmp_reg(),
                    });
                    // finally goto jumps
                    insts.push(x[1].clone());
                }

                /*
                    here is all the jumps.
                */
                let mut need_label_use = vec![];
                for t in targets {
                    need_label_use.push((insts.len(), t.clone()));
                    insts.extend(Inst::construct_auipc_and_jalr(writable_spilltmp_reg(), 0));
                }
                // emit island if need.
                let distance = (insts.len() * 4) as u32;
                if sink.island_needed(distance) {
                    sink.emit_island(distance);
                }
                let mut need_label_use = &need_label_use[..];
                insts.into_iter().enumerate().for_each(|(index, inst)| {
                    if !need_label_use.is_empty() && need_label_use[0].0 == index {
                        sink.use_label_at_offset(
                            sink.cur_offset(),
                            need_label_use[0].1.as_label().unwrap(),
                            LabelUse::PCRel32,
                        );
                        need_label_use = &need_label_use[1..];
                    }
                    inst.emit(&[], sink, emit_info, state);
                });
                // emit the island before, so we can safely
                // disable the worst-case-size check in this case.
                start_off = sink.cur_offset();
            }

            &Inst::VirtualSPOffsetAdj { amount } => {
                log::trace!(
                    "virtual sp offset adjusted by {} -> {}",
                    amount,
                    state.virtual_sp_offset + amount
                );
                state.virtual_sp_offset += amount;
            }

            &Inst::Atomic {
                op,
                rd,
                addr,
                src,
                aq,
                rl,
            } => {
                let addr = allocs.next(addr);
                let src = allocs.next(src);
                let rd = allocs.next_writable(rd);
                let x = op.op_code()
                    | reg_to_gpr_num(rd.to_reg()) << 7
                    | op.funct3() << 12
                    | reg_to_gpr_num(addr) << 15
                    | reg_to_gpr_num(src) << 20
                    | op.funct7(aq, rl) << 25;

                sink.put4(x);
            }
            &Inst::Fence => sink.put4(0x0ff0000f),
            &Inst::FenceI => sink.put4(0x0000100f),
            &Inst::Auipc { rd, imm } => {
                let x = enc_auipc(rd, imm);
                sink.put4(x);
            }

            &Inst::LoadAddr { rd, mem } => {
                let base = mem.get_base_register();
                let base = allocs.next(base);
                let rd = allocs.next_writable(rd);
                let offset = mem.get_offset_with_state(state);
                if let Some(offset) = Imm12::maybe_from_u64(offset as u64) {
                    Inst::AluRRImm12 {
                        alu_op: AluOPRRI::Addi,
                        rd: rd,
                        rs: base,
                        imm12: offset,
                    }
                    .emit(&[], sink, emit_info, state);
                } else {
                    // need more register
                    Inst::do_something_with_registers(1, |registers, insts| {
                        insts.extend(Inst::load_constant_u64(registers[0], offset as u64));

                        insts.push(Inst::AluRRR {
                            rd,
                            alu_op: AluOPRRR::Add,
                            rs1: base,
                            rs2: registers[0].to_reg(),
                        });
                    })
                    .into_iter()
                    .for_each(|inst| inst.emit(&[], sink, emit_info, state));
                }
            }

            &Inst::Fcmp {
                rd,
                tmp,
                cc,
                ty,
                rs1,
                rs2,
            } => {
                let rs1 = allocs.next(rs1);
                let rs2 = allocs.next(rs2);
                let rd = allocs.next_writable(rd);
                let tmp = allocs.next_writable(tmp);
                let label_true = sink.get_label();
                let label_jump_over = sink.get_label();
                Inst::lower_br_fcmp(
                    cc,
                    rs1,
                    rs2,
                    BranchTarget::Label(label_true),
                    BranchTarget::zero(),
                    ty,
                    rd,
                    tmp,
                )
                .iter()
                .for_each(|i| i.emit(&[], sink, emit_info, state));
                // here is not taken.
                Inst::load_constant_imm12(rd, Imm12::form_bool(false)).emit(
                    &[],
                    sink,
                    emit_info,
                    state,
                );
                // jump over.
                Inst::Jal {
                    dest: BranchTarget::Label(label_jump_over),
                }
                .emit(&[], sink, emit_info, state);
                // here is true
                sink.bind_label(label_true);
                Inst::load_constant_imm12(rd, Imm12::form_bool(true)).emit(
                    &[],
                    sink,
                    emit_info,
                    state,
                );
                sink.bind_label(label_jump_over);
            }

            &Inst::Select {
                ref dst,
                conditon,
                ref x,
                ref y,
                ty: _ty,
            } => {
                let conditon = allocs.next(conditon);
                let x = alloc_value_regs(x, &mut allocs);
                let y = alloc_value_regs(y, &mut allocs);
                let dst: Vec<_> = dst
                    .clone()
                    .into_iter()
                    .map(|r| allocs.next_writable(r))
                    .collect();

                let mut insts = SmallInstVec::new();
                let label_false = sink.get_label();
                insts.push(Inst::CondBr {
                    taken: BranchTarget::Label(label_false),
                    not_taken: BranchTarget::zero(),
                    kind: IntegerCompare {
                        kind: IntCC::Equal,
                        rs1: conditon,
                        rs2: zero_reg(),
                    },
                });
                // here is the true
                // select the first value
                insts.extend(gen_moves(&dst[..], x.regs()));
                let label_jump_over = sink.get_label();
                insts.push(Inst::Jal {
                    dest: BranchTarget::Label(label_jump_over),
                });
                // here is false
                insts
                    .drain(..)
                    .for_each(|i: Inst| i.emit(&[], sink, emit_info, state));
                sink.bind_label(label_false);
                // select second value1
                insts.extend(gen_moves(&dst[..], y.regs()));
                insts
                    .into_iter()
                    .for_each(|i| i.emit(&[], sink, emit_info, state));
                sink.bind_label(label_jump_over);
            }
            &Inst::Jalr { rd, base, offset } => {
                let x = enc_jalr(rd, base, offset);
                sink.put4(x);
            }
            &Inst::ECall => {
                sink.put4(0x00000073);
            }
            &Inst::EBreak => {
                sink.put4(0x00100073);
            }
            &Inst::Icmp {
                cc,
                rd,
                ref a,
                ref b,
                ty,
            } => {
                let a = alloc_value_regs(a, &mut allocs);
                let b = alloc_value_regs(b, &mut allocs);
                let rd = allocs.next_writable(rd);
                let label_true = sink.get_label();
                let label_false = sink.get_label();
                Inst::lower_br_icmp(
                    cc,
                    a,
                    b,
                    BranchTarget::Label(label_true),
                    BranchTarget::Label(label_false),
                    ty,
                )
                .into_iter()
                .for_each(|i| i.emit(&[], sink, emit_info, state));

                sink.bind_label(label_true);
                Inst::load_constant_imm12(rd, Imm12::from_bits(-1)).emit(
                    &[],
                    sink,
                    emit_info,
                    state,
                );
                Inst::Jal {
                    dest: BranchTarget::offset(Inst::instruction_size() * 2),
                }
                .emit(&[], sink, emit_info, state);
                sink.bind_label(label_false);
                Inst::load_constant_imm12(rd, Imm12::from_bits(0)).emit(
                    &[],
                    sink,
                    emit_info,
                    state,
                );
            }
            &Inst::AtomicCas {
                t0,
                dst,
                e,
                addr,
                v,
                ty,
            } => {
                let e = allocs.next(e);
                let addr = allocs.next(addr);
                let v = allocs.next(v);
                let t0 = allocs.next_writable(t0);
                let dst = allocs.next_writable(dst);
                /*
                    # addr holds address of memory location
                    # e holds expected value
                    # v holds desired value
                    # dst holds return value

                cas:
                    lr.w t0, (addr) # Load original value.
                    bne t0, e, fail # Doesn’t match, so fail.
                    sc.w dst, v, (addr) # Try to update.
                    bne dst , v , cas  # retry
                fail:
                                           */
                let fail_label = sink.get_label();
                let cas_lebel = sink.get_label();
                sink.bind_label(cas_lebel);
                // lr.w t0, (addr)
                Inst::Atomic {
                    op: if ty.bits() == 64 {
                        AtomicOP::LrD
                    } else {
                        AtomicOP::LrW
                    },
                    rd: t0,
                    addr,
                    src: zero_reg(),
                    aq: true,
                    rl: false,
                }
                .emit(&[], sink, emit_info, state);
                // bne t0, e, fail
                Inst::CondBr {
                    taken: BranchTarget::Label(fail_label),
                    not_taken: BranchTarget::zero(),
                    kind: IntegerCompare {
                        kind: IntCC::NotEqual,
                        rs1: e,
                        rs2: t0.to_reg(),
                    },
                }
                .emit(&[], sink, emit_info, state);
                //  sc.w dst, v, (addr)
                Inst::Atomic {
                    op: if ty.bits() == 64 {
                        AtomicOP::ScD
                    } else {
                        AtomicOP::ScW
                    },
                    rd: dst,
                    addr,
                    src: v,
                    aq: false,
                    rl: true,
                }
                .emit(&[], sink, emit_info, state);
                // bne dst , v , cas retry.
                Inst::CondBr {
                    taken: BranchTarget::Label(cas_lebel),
                    not_taken: BranchTarget::zero(),
                    kind: IntegerCompare {
                        kind: IntCC::NotEqual,
                        rs1: dst.to_reg(),
                        rs2: v,
                    },
                }
                .emit(&[], sink, emit_info, state);
                sink.bind_label(fail_label);
            }
            &Inst::IntSelect {
                op,
                ref dst,
                ref x,
                ref y,
                ty,
            } => {
                let x = alloc_value_regs(x, &mut allocs);
                let y = alloc_value_regs(y, &mut allocs);
                let dst: Vec<_> = dst.iter().map(|r| allocs.next_writable(*r)).collect();
                let label_true = sink.get_label();
                let label_false = sink.get_label();
                let label_done = sink.get_label();
                Inst::lower_br_icmp(
                    op.to_int_cc(),
                    x,
                    y,
                    BranchTarget::Label(label_true),
                    BranchTarget::Label(label_false),
                    ty,
                )
                .into_iter()
                .for_each(|i| i.emit(&[], sink, emit_info, state));

                let gen_move = |dst: &Vec<Writable<Reg>>,
                                val: &ValueRegs<Reg>,
                                sink: &mut MachBuffer<Inst>,
                                state: &mut EmitState| {
                    let ty = if ty.bits() == 128 { I64 } else { ty };
                    let mut insts = SmallInstVec::new();
                    insts.push(Inst::Mov {
                        rd: dst[0],
                        rm: val.regs()[0],
                        ty,
                    });
                    if ty.bits() == 128 {
                        insts.push(Inst::Mov {
                            rd: dst[1],
                            rm: val.regs()[1],
                            ty,
                        });
                    }
                    insts
                        .into_iter()
                        .for_each(|i| i.emit(&[], sink, emit_info, state));
                };
                //here is true , use x.
                sink.bind_label(label_true);
                gen_move(&dst, &x, sink, state);
                Inst::gen_jump(label_done).emit(&[], sink, emit_info, state);
                // here is false use y
                sink.bind_label(label_false);
                gen_move(&dst, &y, sink, state);
                sink.bind_label(label_done);
            }
            &Inst::Csr {
                csr_op,
                rd,
                rs,
                imm,
                csr,
            } => {
                let rs = rs.map(|r| allocs.next(r));
                let rd = allocs.next_writable(rd);
                let x = csr_op.op_code()
                    | reg_to_gpr_num(rd.to_reg()) << 7
                    | csr_op.funct3() << 12
                    | csr_op.rs1(rs, imm) << 15
                    | csr.as_u32() << 20;

                sink.put4(x);
            }

            &Inst::Cls { rs, rd, ty } => {
                let rs = allocs.next(rs);
                let rd = allocs.next_writable(rd);
                //extract sign bit.
                let (op, imm12) = AluOPRRI::Bexti.funct12(Some((ty.bits() - 1) as u8));
                Inst::AluRRImm12 {
                    alu_op: op,
                    rd,
                    rs: rs,
                    imm12,
                }
                .emit(&[], sink, emit_info, state);
                let label_signed_value = sink.get_label();
                Inst::CondBr {
                    taken: BranchTarget::Label(label_signed_value),
                    not_taken: BranchTarget::zero(),
                    kind: IntegerCompare {
                        kind: IntCC::NotEqual,
                        rs1: rd.to_reg(),
                        rs2: zero_reg(),
                    },
                }
                .emit(&[], sink, emit_info, state);

                fn cls(
                    rd: Writable<Reg>,
                    rs: Reg,
                    sink: &mut MachBuffer<Inst>,
                    emit_info: &EmitInfo,
                    state: &mut EmitState,
                    ty: Type,
                ) {
                    if 64 - ty.bits() > 0 {
                        Inst::narrow_down_int(rd, rs, ty)
                            .iter()
                            .for_each(|i| i.emit(&[], sink, emit_info, state));
                        let (op, imm12) = AluOPRRI::Clz.funct12(None);
                        Inst::AluRRImm12 {
                            alu_op: op,
                            rd,
                            rs: rd.to_reg(),
                            imm12,
                        }
                        .emit(&[], sink, emit_info, state);
                    } else {
                        let (op, imm12) = AluOPRRI::Clz.funct12(None);
                        Inst::AluRRImm12 {
                            alu_op: op,
                            rd,
                            rs,
                            imm12,
                        }
                        .emit(&[], sink, emit_info, state);
                    }
                    // make result.
                    Inst::AluRRImm12 {
                        alu_op: AluOPRRI::Addi,
                        rd,
                        rs: rd.to_reg(),
                        imm12: Imm12::from_bits(-(64 - (ty.bits() - 1) as i16)),
                    }
                    .emit(&[], sink, emit_info, state);
                }
                //here is need counting leading zeros.
                cls(rd, rs, sink, emit_info, state, ty);
                let label_jump_over = sink.get_label();
                Inst::Jal {
                    dest: BranchTarget::Label(label_jump_over),
                }
                .emit(&[], sink, emit_info, state);
                // sign bit is 1.
                // reverse all bits.
                sink.bind_label(label_signed_value);
                Inst::construct_bit_not(rd, rs).emit(&[], sink, emit_info, state);
                cls(rd, rd.to_reg(), sink, emit_info, state, ty);
                sink.bind_label(label_jump_over);
            }

            &Inst::SelectReg {
                condition,
                rd,
                rs1,
                rs2,
            } => {
                let mut condition = condition.clone();
                condition.rs1 = allocs.next(condition.rs1);
                condition.rs2 = allocs.next(condition.rs2);
                let rs1 = allocs.next(rs1);
                let rs2 = allocs.next(rs2);
                let rd = allocs.next_writable(rd);
                let label_true = sink.get_label();
                let label_jump_over = sink.get_label();
                sink.use_label_at_offset(sink.cur_offset(), label_true, LabelUse::B12);
                let x = condition.emit();
                sink.put4(x);
                // here is false , use rs2
                Inst::gen_move(rd, rs2, I64).emit(&[], sink, emit_info, state);
                // and jump over
                sink.use_label_at_offset(sink.cur_offset(), label_jump_over, LabelUse::Jal20);

                Inst::Jal {
                    dest: BranchTarget::Label(label_jump_over),
                }
                .emit(&[], sink, emit_info, state);
                // here condition is true , use rs1
                sink.bind_label(label_true);
                Inst::gen_move(rd, rs1, I64).emit(&[], sink, emit_info, state);
                sink.bind_label(label_jump_over);
            }
            &Inst::FcvtToIntSat {
                rd,
                rs,
                tmp,
                is_signed,
                in_type,
                out_type,
            } => {
                let rs = allocs.next(rs);
                let rd = allocs.next_writable(rd);
                let tmp = allocs.next_writable(tmp);
                // get class information.
                Inst::FpuRR {
                    frm: None,
                    alu_op: if in_type == F32 {
                        FpuOPRR::FclassS
                    } else {
                        FpuOPRR::FclassD
                    },
                    rd: tmp,
                    rs,
                }
                .emit(&[], sink, emit_info, state);
                // rd = rd & is_nan()
                Inst::AluRRImm12 {
                    alu_op: AluOPRRI::Andi,
                    rd: tmp,
                    rs: tmp.to_reg(),
                    imm12: Imm12::from_bits(FClassResult::is_nan_bits() as i16),
                }
                .emit(&[], sink, emit_info, state);
                // jump to nan
                let label_jump_nan = sink.get_label();
                Inst::CondBr {
                    taken: BranchTarget::Label(label_jump_nan),
                    not_taken: BranchTarget::zero(),
                    kind: IntegerCompare {
                        kind: IntCC::NotEqual,
                        rs1: zero_reg(),
                        rs2: tmp.to_reg(),
                    },
                }
                .emit(&[], sink, emit_info, state);
                // convert to int normally.
                Inst::FpuRR {
                    frm: None,
                    alu_op: FpuOPRR::float_convert_2_int_op(in_type, is_signed, out_type),
                    rd: rd,
                    rs: rs,
                }
                .emit(&[], sink, emit_info, state);
                // I already have the result,jump over.
                let label_jump_over = sink.get_label();
                Inst::Jal {
                    dest: BranchTarget::Label(label_jump_over),
                }
                .emit(&[], sink, emit_info, state);
                // here is nan , move 0 into rd register
                sink.bind_label(label_jump_nan);
                Inst::load_constant_imm12(rd, Imm12::from_bits(0)).emit(
                    &[],
                    sink,
                    emit_info,
                    state,
                );

                // bind jump_over
                sink.bind_label(label_jump_over);
            }

            &Inst::LoadExtName {
                rd,
                ref name,
                offset,
            } => {
                let rd = allocs.next_writable(rd);
                // get the current pc.
                Inst::Auipc {
                    rd: rd,
                    imm: Imm20::from_bits(0),
                }
                .emit(&[], sink, emit_info, state);
                // load the value.
                Inst::Load {
                    rd: rd,
                    op: LoadOP::Ld,
                    flags: MemFlags::trusted(),
                    from: AMode::RegOffset(rd.to_reg(), 12 /* auipc load and jal */, I64),
                }
                .emit(&[], sink, emit_info, state);
                // jump over.
                Inst::Jal {
                    dest: BranchTarget::offset(12 /* jal and abs8 size  */),
                }
                .emit(&[], sink, emit_info, state);
                let srcloc = state.cur_srcloc;
                sink.add_reloc(srcloc, Reloc::Abs8, name.as_ref(), offset);
                if emit_info.0.emit_all_ones_funcaddrs() {
                    sink.put8(u64::max_value());
                } else {
                    sink.put8(0);
                }
            }

            &Inst::TrapIf {
                cc,
                ref x,
                ref y,
                ty,
                trap_code,
            } => {
                let x = alloc_value_regs(x, &mut allocs);
                let y = alloc_value_regs(y, &mut allocs);
                let label_trap = sink.get_label();
                let label_jump_over = sink.get_label();
                Inst::lower_br_icmp(
                    cc,
                    x,
                    y,
                    BranchTarget::Label(label_trap),
                    BranchTarget::Label(label_jump_over),
                    ty,
                )
                .iter()
                .for_each(|i| i.emit(&[], sink, emit_info, state));
                // trap
                sink.bind_label(label_trap);
                Inst::Udf {
                    trap_code: trap_code,
                }
                .emit(&[], sink, emit_info, state);
                sink.bind_label(label_jump_over);
            }
            &Inst::TrapFf {
                cc,
                x,
                y,
                ty,
                trap_code,
                tmp,
                tmp2,
            } => {
                let x = allocs.next(x);
                let y = allocs.next(y);
                let tmp = allocs.next_writable(tmp);
                let tmp2 = allocs.next_writable(tmp2);
                let label_trap = sink.get_label();
                let label_jump_over = sink.get_label();
                Inst::lower_br_fcmp(
                    cc,
                    x,
                    y,
                    BranchTarget::Label(label_trap),
                    BranchTarget::Label(label_jump_over),
                    ty,
                    tmp,
                    tmp2,
                )
                .iter()
                .for_each(|i| i.emit(&[], sink, emit_info, state));
                // trap
                sink.bind_label(label_trap);
                Inst::Udf {
                    trap_code: trap_code,
                }
                .emit(&[], sink, emit_info, state);
                sink.bind_label(label_jump_over);
            }

            &Inst::Udf { trap_code } => {
                let srcloc = state.cur_srcloc();
                sink.add_trap(srcloc, trap_code);
                if let Some(s) = state.take_stack_map() {
                    sink.add_stack_map(StackMapExtent::UpcomingBytes(4), s);
                }
                /*
                    https://github.com/riscv/riscv-isa-manual/issues/850
                    all zero will cause invalid opcode.
                */
                sink.put4(0);
            }
            &Inst::SelectIf {
                if_spectre_guard: _if_spectre_guard,
                ref rd,
                ref cmp_x,
                ref cmp_y,
                cc,
                ref x,
                ref y,
                cmp_ty,
            } => {
                /*
                    todo:: _if_spectre_guard not used.
                */
                let label_select_x = sink.get_label();
                let label_select_y = sink.get_label();
                let label_jump_over = sink.get_label();
                let cmp_x = alloc_value_regs(cmp_x, &mut allocs);
                let cmp_y = alloc_value_regs(cmp_y, &mut allocs);
                let x = alloc_value_regs(x, &mut allocs);
                let y = alloc_value_regs(y, &mut allocs);
                let rd: Vec<_> = rd.iter().map(|r| allocs.next_writable(*r)).collect();
                Inst::lower_br_icmp(
                    cc,
                    cmp_x,
                    cmp_y,
                    BranchTarget::Label(label_select_x),
                    BranchTarget::Label(label_select_y),
                    cmp_ty,
                )
                .into_iter()
                .for_each(|i| i.emit(&[], sink, emit_info, state));
                // here select x.
                sink.bind_label(label_select_x);
                gen_moves(&rd[..], x.regs())
                    .into_iter()
                    .for_each(|i| i.emit(&[], sink, emit_info, state));
                // jump over
                Inst::Jal {
                    dest: BranchTarget::Label(label_jump_over),
                }
                .emit(&[], sink, emit_info, state);
                // here select y.
                sink.bind_label(label_select_y);
                gen_moves(&rd[..], y.regs())
                    .into_iter()
                    .for_each(|i| i.emit(&[], sink, emit_info, state));
                sink.bind_label(label_jump_over);
            }
        };
        let end_off = sink.cur_offset();
        assert!((end_off - start_off) <= Inst::worst_case_size());
    }

    fn pretty_print_inst(&self, allocs: &[Allocation], state: &mut Self::State) -> String {
        let mut allocs = AllocationConsumer::new(allocs);
        self.print_with_state(state, &mut allocs)
    }
}

// helper function.
fn alloc_value_regs(orgin: &ValueRegs<Reg>, alloc: &mut AllocationConsumer) -> ValueRegs<Reg> {
    let x: Vec<_> = orgin.regs().into_iter().map(|r| alloc.next(*r)).collect();
    match x.len() {
        1 => ValueRegs::one(x[0]),
        2 => ValueRegs::two(x[0], x[1]),
        _ => unreachable!(),
    }
}
