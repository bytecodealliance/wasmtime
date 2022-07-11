//! Riscv64 ISA: binary code emission.

use crate::binemit::StackMap;
use crate::isa::riscv64::inst::*;
use crate::isa::riscv64::inst::{zero_reg, AluOPRRR};
use crate::machinst::{AllocationConsumer, Reg, Writable};
use regalloc2::Allocation;

pub struct EmitInfo {
    shared_flag: settings::Flags,
    isa_flags: super::super::riscv_settings::Flags,
}

impl EmitInfo {
    pub(crate) fn new(
        shared_flag: settings::Flags,
        isa_flags: super::super::riscv_settings::Flags,
    ) -> Self {
        Self {
            shared_flag,
            isa_flags,
        }
    }
}

/// load constant by put the constant in the code stream.
/// caculate the pc and using load instruction.
#[derive(Clone, Copy)]
pub(crate) enum LoadConstant {
    U32(u32),
    U64(u64),
}

impl LoadConstant {
    fn to_le_bytes(self) -> Vec<u8> {
        match self {
            LoadConstant::U32(x) => Vec::from_iter(x.to_le_bytes().into_iter()),
            LoadConstant::U64(x) => Vec::from_iter(x.to_le_bytes().into_iter()),
        }
    }
    fn load_op(self) -> LoadOP {
        match self {
            LoadConstant::U32(_) => LoadOP::Lwu,
            LoadConstant::U64(_) => LoadOP::Ld,
        }
    }
    fn load_ty(self) -> Type {
        match self {
            LoadConstant::U32(_) => R32,
            LoadConstant::U64(_) => R64,
        }
    }

    pub(crate) fn load_constant(self, rd: Writable<Reg>) -> SmallInstVec<Inst> {
        let mut insts = SmallInstVec::new();
        // get current pc.
        insts.push(Inst::Auipc {
            rd,
            imm: Imm20 { bits: 0 },
        });
        // load
        insts.push(Inst::Load {
            rd,
            op: self.load_op(),
            flags: MemFlags::new(),
            from: AMode::RegOffset(rd.to_reg(), 12, self.load_ty()),
        });
        let data = self.to_le_bytes();
        // jump over.
        insts.push(Inst::Jal {
            dest: BranchTarget::ResolvedOffset(Inst::INSTRUCTION_SIZE + data.len() as i32),
        });
        insts.push(Inst::RawData { data });
        insts
    }

    // load and perform an extra add.
    pub(crate) fn load_constant_and_add(self, rd: Writable<Reg>, rs: Reg) -> SmallInstVec<Inst> {
        let mut insts = self.load_constant(rd);
        insts.push(Inst::AluRRR {
            alu_op: AluOPRRR::Add,
            rd: rd,
            rs1: rd.to_reg(),
            rs2: rs,
        });
        insts
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
    ) -> SmallInstVec<Inst> {
        assert!(tmp.to_reg().class() == RegClass::Int);
        let mut insts = SmallInstVec::new();
        let mut cc_args = FloatCCArgs::from_floatcc(cc);
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
        let le_op = if ty == F32 {
            FpuOPRRR::FleS
        } else {
            FpuOPRRR::FleD
        };

        // >=
        if cc_args.has_and_clear(FloatCCArgs::GT | FloatCCArgs::EQ) {
            insts.push(Inst::FpuRRR {
                frm: None,
                alu_op: le_op,
                rd: tmp,
                rs1: y, /*  x and y order reversed. */
                rs2: x,
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

        // <=
        if cc_args.has_and_clear(FloatCCArgs::LT | FloatCCArgs::EQ) {
            insts.push(Inst::FpuRRR {
                frm: None,
                alu_op: le_op,
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

        // if eq
        if cc_args.has_and_clear(FloatCCArgs::EQ) {
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
        // if ne
        if cc_args.has_and_clear(FloatCCArgs::NE) {
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
                    kind: IntCC::Equal,
                    rs1: tmp.to_reg(),
                    rs2: zero_reg(),
                },
            });
        }

        // if <
        if cc_args.has_and_clear(FloatCCArgs::LT) {
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
        if cc_args.has_and_clear(FloatCCArgs::GT) {
            insts.push(Inst::FpuRRR {
                frm: None,
                alu_op: lt_op,
                rd: tmp,
                rs1: y, /* x and y order reversed. */
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
        if cc_args.has_and_clear(FloatCCArgs::UN) {
            insts.extend(Inst::lower_float_unordered(tmp, ty, x, y, taken, not_taken));
        } else {
            //make sure we goto the not_taken.
            //finally goto not_taken
            insts.push(Inst::Jal { dest: not_taken });
        }
        // make sure we handle all cases.
        assert!(cc_args.0 == 0);
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
        // compare i128
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
                    kind: high(cc.without_equal()),
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
                    kind: low(cc.unsigned()),
                });
            }
            IntCC::Overflow | IntCC::NotOverflow => overflow_already_lowerd(),
        }
        insts
    }

    /*
        check if float is unordered.
    */
    pub(crate) fn lower_float_unordered(
        tmp: Writable<Reg>,
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
            rd: tmp,
            rs: y,
        });
        insts.push(Inst::AluRRImm12 {
            alu_op: AluOPRRI::Andi,
            rd: tmp,
            rs: tmp.to_reg(),
            imm12: Imm12::from_bits(FClassResult::is_nan_bits() as i16),
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
            &Inst::RawData { ref data } => {
                /*
                    emit_island if need, right now data is not very long.
                */
                let length = data.len() as CodeOffset;
                if sink.island_needed(length) {
                    sink.emit_island(length);
                }
                sink.put_data(&data[..]);
                /*
                    safe to disable code length check.
                */
                start_off = sink.cur_offset();
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
            &Inst::Unwind { ref inst } => {
                sink.add_unwind(inst.clone());
            }
            &Inst::DummyUse { reg } => {
                allocs.next(reg);
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

                if alu_op == AluOPRRR::Sgt || alu_op == AluOPRRR::Sgtu {
                    // special case
                    // sgt and sgtu is not defined in isa.
                    // emit should reserver rs1 and rs2.
                    let x: u32 = alu_op.op_code()
                        | reg_to_gpr_num(rd.to_reg()) << 7
                        | (alu_op.funct3()) << 12
                        | reg_to_gpr_num(rs2) << 15
                        | reg_to_gpr_num(rs1) << 20
                        | alu_op.funct7() << 25;
                    sink.put4(x);
                } else {
                    let x: u32 = alu_op.op_code()
                        | reg_to_gpr_num(rd.to_reg()) << 7
                        | (alu_op.funct3()) << 12
                        | reg_to_gpr_num(rs1) << 15
                        | reg_to_gpr_num(rs2) << 20
                        | alu_op.funct7() << 25;
                    sink.put4(x);
                }
            }
            &Inst::AluRRImm12 {
                alu_op,
                rd,
                rs,
                imm12,
            } => {
                let rs = allocs.next(rs);
                let rd = allocs.next_writable(rd);
                let x = alu_op.op_code()
                    | reg_to_gpr_num(rd.to_reg()) << 7
                    | alu_op.funct3() << 12
                    | reg_to_gpr_num(rs) << 15
                    | alu_op.imm12(imm12) << 20;
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
                    let tmp = writable_spilltmp_reg();
                    let mut insts =
                        LoadConstant::U64(offset as u64).load_constant_and_add(tmp, base);
                    insts.push(Inst::Load {
                        op,
                        from: AMode::RegOffset(tmp.to_reg(), 0, I64),
                        rd,
                        flags,
                    });
                    insts
                        .into_iter()
                        .for_each(|inst| inst.emit(&[], sink, emit_info, state));
                }
            }
            &Inst::Store { op, src, flags, to } => {
                let base = allocs.next(to.get_base_register());
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
                    let tmp = writable_spilltmp_reg();
                    let mut insts =
                        LoadConstant::U64(offset as u64).load_constant_and_add(tmp, base);
                    insts.push(Inst::Store {
                        op,
                        to: AMode::RegOffset(tmp.to_reg(), 0, I64),
                        flags,
                        src,
                    });
                    insts
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
                            taken: BranchTarget::ResolvedOffset(Inst::INSTRUCTION_SIZE * 3),
                            not_taken: BranchTarget::zero(),
                            kind: IntegerCompare {
                                kind: IntCC::Equal,
                                rs1: zero_reg(),
                                rs2: x,
                            },
                        });
                        // here is false
                        insts.push(Inst::load_constant_imm12(rd, Imm12::FALSE));
                        insts.push(Inst::Jal {
                            dest: BranchTarget::ResolvedOffset(Inst::INSTRUCTION_SIZE * 2),
                        });
                        // here is true
                        insts.push(Inst::load_constant_imm12(rd, Imm12::TRUE));
                    }

                    ReferenceCheckOP::IsInvalid => {
                        /*
                            todo:: right now just check if it is null
                            null is a valid reference??????
                        */
                        insts.push(Inst::CondBr {
                            taken: BranchTarget::ResolvedOffset(Inst::INSTRUCTION_SIZE * 3),
                            not_taken: BranchTarget::zero(),
                            kind: IntegerCompare {
                                kind: IntCC::Equal,
                                rs1: zero_reg(),
                                rs2: x,
                            },
                        });
                        // here is false
                        insts.push(Inst::load_constant_imm12(rd, Imm12::FALSE));
                        insts.push(Inst::Jal {
                            dest: BranchTarget::ResolvedOffset(Inst::INSTRUCTION_SIZE * 2),
                        });
                        // here is true
                        insts.push(Inst::load_constant_imm12(rd, Imm12::TRUE));
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
                let shift_bits = (64 - from_bits) as i16;
                if signed {
                    insts.push(Inst::AluRRImm12 {
                        alu_op: AluOPRRI::Slli,
                        rd,
                        rs: rn,
                        imm12: Imm12::from_bits(shift_bits),
                    });
                    insts.push(Inst::AluRRImm12 {
                        alu_op: AluOPRRI::Srai,
                        rd,
                        rs: rd.to_reg(),
                        imm12: Imm12::from_bits(shift_bits),
                    });
                } else {
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
                    let tmp = writable_spilltmp_reg();
                    let mut insts = Inst::load_constant_u64(tmp, amount as u64);
                    insts.push(Inst::AluRRR {
                        alu_op: AluOPRRR::Add,
                        rd: writable_stack_reg(),
                        rs1: tmp.to_reg(),
                        rs2: stack_reg(),
                    });
                    insts
                        .into_iter()
                        .for_each(|i| i.emit(&[], sink, emit_info, state));
                }
            }

            &Inst::Call { ref info } => {
                // call
                match info.dest {
                    ExternalName::User { .. } => {
                        if info.opcode.is_call() {
                            sink.add_call_site(info.opcode);
                        }
                        sink.add_reloc(Reloc::RiscvCall, &info.dest, 0);
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

                if info.opcode.is_call() {
                    sink.add_call_site(info.opcode);
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
                        let offset = offset as i64;
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
                        if LabelUse::B12.offset_in_range(offset as i64) {
                            let code = kind.emit();
                            let mut code = code.to_le_bytes();
                            LabelUse::B12.patch_raw_offset(&mut code, offset as i64);
                            sink.put_data(&code[..])
                        } else {
                            let mut code = kind.emit().to_le_bytes();
                            /*
                                jump over the condbr , 4 bytes.
                            */
                            LabelUse::B12.patch_raw_offset(&mut code[..], 4);
                            sink.put_data(&code[..]);
                            Inst::construct_auipc_and_jalr(writable_spilltmp_reg(), offset as i64)
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
                        Inst::FpuRRR {
                            alu_op: if ty == F32 {
                                FpuOPRRR::FsgnjS
                            } else {
                                FpuOPRRR::FsgnjD
                            },
                            frm: None,
                            rd: rd,
                            rs1: rm,
                            rs2: rm,
                        }
                        .emit(&[], sink, emit_info, state);
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
                amo,
            } => {
                let addr = allocs.next(addr);
                let src = allocs.next(src);
                let rd = allocs.next_writable(rd);
                let x = op.op_code()
                    | reg_to_gpr_num(rd.to_reg()) << 7
                    | op.funct3() << 12
                    | reg_to_gpr_num(addr) << 15
                    | reg_to_gpr_num(src) << 20
                    | op.funct7(amo) << 25;

                sink.put4(x);
            }
            &Inst::Fence { pred, succ, fm } => {
                let x = 0b0001111
                    | 0b00000 << 7
                    | 0b000 << 12
                    | 0b00000 << 15
                    | (succ as u32) << 20
                    | (pred as u32) << 24
                    | fm.as_u32() << 28;

                sink.put4(x);
            }
            &Inst::FenceI => sink.put4(0x0000100f),
            &Inst::Auipc { rd, imm } => {
                let rd = allocs.next_writable(rd);
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
                    let insts = LoadConstant::U64(offset as u64).load_constant_and_add(rd, base);
                    insts
                        .into_iter()
                        .for_each(|i| i.emit(&[], sink, emit_info, state));
                }
            }

            &Inst::Fcmp {
                rd,
                cc,
                ty,
                rs1,
                rs2,
            } => {
                let rs1 = allocs.next(rs1);
                let rs2 = allocs.next(rs2);
                let rd = allocs.next_writable(rd);
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
                )
                .iter()
                .for_each(|i| i.emit(&[], sink, emit_info, state));
                // here is not taken.
                Inst::load_constant_imm12(rd, Imm12::FALSE).emit(&[], sink, emit_info, state);
                // jump over.
                Inst::Jal {
                    dest: BranchTarget::Label(label_jump_over),
                }
                .emit(&[], sink, emit_info, state);
                // here is true
                sink.bind_label(label_true);
                Inst::load_constant_imm12(rd, Imm12::TRUE).emit(&[], sink, emit_info, state);
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
                let rd = allocs.next_writable(rd);
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
                    dest: BranchTarget::offset(Inst::INSTRUCTION_SIZE * 2),
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
                    lr.w dst, (addr)       # Load original value.
                    bne dst, e, fail       # Doesn’t match, so fail.
                    sc.w t0, v, (addr)     # Try to update.
                    bnez t0 , cas          # if store not ok,retry.
                fail:
                                           */
                let fail_label = sink.get_label();
                let cas_lebel = sink.get_label();
                sink.bind_label(cas_lebel);

                Inst::Atomic {
                    op: AtomicOP::load_op(ty),
                    rd: dst,
                    addr,
                    src: zero_reg(),
                    amo: AMO::SeqCst,
                }
                .emit(&[], sink, emit_info, state);
                Inst::CondBr {
                    taken: BranchTarget::Label(fail_label),
                    not_taken: BranchTarget::zero(),
                    kind: IntegerCompare {
                        kind: IntCC::NotEqual,
                        rs1: e,
                        rs2: dst.to_reg(),
                    },
                }
                .emit(&[], sink, emit_info, state);
                Inst::Atomic {
                    op: AtomicOP::store_op(ty),
                    rd: t0,
                    addr,
                    src: v,
                    amo: AMO::SeqCst,
                }
                .emit(&[], sink, emit_info, state);
                // check is our value stored.
                Inst::CondBr {
                    taken: BranchTarget::Label(cas_lebel),
                    not_taken: BranchTarget::zero(),
                    kind: IntegerCompare {
                        kind: IntCC::NotEqual,
                        rs1: t0.to_reg(),
                        rs2: zero_reg(),
                    },
                }
                .emit(&[], sink, emit_info, state);
                sink.bind_label(fail_label);
            }
            &Inst::AtomicNand { dst, ty, p, x, t0 } => {
                let p = allocs.next(p);
                let x = allocs.next(x);
                let t0 = allocs.next_writable(t0);
                let dst = allocs.next_writable(dst);
                let retry = sink.get_label();
                sink.bind_label(retry);
                // load old value.
                Inst::Atomic {
                    op: AtomicOP::load_op(ty),
                    rd: dst,
                    addr: p,
                    src: zero_reg(),
                    amo: AMO::SeqCst,
                }
                .emit(&[], sink, emit_info, state);
                //
                Inst::AluRRR {
                    alu_op: AluOPRRR::And,
                    rd: t0,
                    rs1: dst.to_reg(),
                    rs2: x,
                }
                .emit(&[], sink, emit_info, state);
                Inst::construct_bit_not(t0, t0.to_reg()).emit(&[], sink, emit_info, state);
                // try store.
                Inst::Atomic {
                    op: AtomicOP::store_op(ty),
                    rd: t0,
                    addr: p,
                    src: t0.to_reg(),
                    amo: AMO::SeqCst,
                }
                .emit(&[], sink, emit_info, state);

                // if store is not ok,retry.
                Inst::CondBr {
                    taken: BranchTarget::Label(retry),
                    not_taken: BranchTarget::zero(),
                    kind: IntegerCompare {
                        kind: IntCC::NotEqual,
                        rs1: t0.to_reg(),
                        rs2: zero_reg(),
                    },
                }
                .emit(&[], sink, emit_info, state);
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
                is_signed,
                in_type,
                out_type,
            } => {
                let rs = allocs.next(rs);
                let rd = allocs.next_writable(rd);
                // get class information.
                Inst::FpuRR {
                    frm: None,
                    alu_op: if in_type == F32 {
                        FpuOPRR::FclassS
                    } else {
                        FpuOPRR::FclassD
                    },
                    rd,
                    rs,
                }
                .emit(&[], sink, emit_info, state);
                // rd = rd & is_nan()
                Inst::AluRRImm12 {
                    alu_op: AluOPRRI::Andi,
                    rd: rd,
                    rs: rd.to_reg(),
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
                        rs2: rd.to_reg(),
                    },
                }
                .emit(&[], sink, emit_info, state);
                // convert to int normally.
                Inst::FpuRR {
                    frm: None,
                    alu_op: FpuOPRR::float_convert_2_int_op(in_type, is_signed, out_type),
                    rd,
                    rs,
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

                sink.add_reloc(Reloc::Abs8, name.as_ref(), offset);
                if emit_info.shared_flag.emit_all_ones_funcaddrs() {
                    sink.put8(u64::max_value());
                } else {
                    sink.put8(0);
                }
            }
            &Inst::TrapIfC {
                rs1,
                rs2,
                cc,
                trap_code,
            } => {
                let rs1 = allocs.next(rs1);
                let rs2 = allocs.next(rs2);
                let label_trap = sink.get_label();
                let label_jump_over = sink.get_label();
                Inst::CondBr {
                    taken: BranchTarget::Label(label_trap),
                    not_taken: BranchTarget::Label(label_jump_over),
                    kind: IntegerCompare { kind: cc, rs1, rs2 },
                }
                .emit(&[], sink, emit_info, state);
                // trap
                sink.bind_label(label_trap);
                Inst::Udf {
                    trap_code: trap_code,
                }
                .emit(&[], sink, emit_info, state);
                sink.bind_label(label_jump_over);
            }
            &Inst::TrapIf { test, trap_code } => {
                let test = allocs.next(test);
                let label_trap = sink.get_label();
                let label_jump_over = sink.get_label();
                Inst::CondBr {
                    taken: BranchTarget::Label(label_trap),
                    not_taken: BranchTarget::Label(label_jump_over),
                    kind: IntegerCompare {
                        kind: IntCC::NotEqual,
                        rs1: test,
                        rs2: zero_reg(),
                    },
                }
                .emit(&[], sink, emit_info, state);
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
            } => {
                let x = allocs.next(x);
                let y = allocs.next(y);
                let tmp = allocs.next_writable(tmp);
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
                sink.add_trap(trap_code);
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
                if_spectre_guard: _if_spectre_guard, /* _if_spectre_guard not use because it is used to not be removed by optimization pass and some other staff. */
                ref rd,
                test,
                ref x,
                ref y,
            } => {
                let label_select_x = sink.get_label();
                let label_select_y = sink.get_label();
                let label_jump_over = sink.get_label();
                let test = allocs.next(test);
                let x = alloc_value_regs(x, &mut allocs);
                let y = alloc_value_regs(y, &mut allocs);
                let rd: Vec<_> = rd.iter().map(|r| allocs.next_writable(*r)).collect();
                Inst::CondBr {
                    taken: BranchTarget::Label(label_select_x),
                    not_taken: BranchTarget::Label(label_select_y),
                    kind: IntegerCompare {
                        kind: IntCC::NotEqual,
                        rs1: test,
                        rs2: zero_reg(),
                    },
                }
                .emit(&[], sink, emit_info, state);

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
            &Inst::AtomicLoad { rd, ty, p } => {
                let p = allocs.next(p);
                let rd = allocs.next_writable(rd);
                // emit the fence.
                Inst::Fence {
                    fm: Default::default(),
                    pred: Inst::FENCE_REQ_R | Inst::FENCE_REQ_W,
                    succ: Inst::FENCE_REQ_R | Inst::FENCE_REQ_W,
                }
                .emit(&[], sink, emit_info, state);
                Inst::Load {
                    rd: rd,
                    op: LoadOP::from_type(ty),
                    flags: MemFlags::trusted(),
                    from: AMode::RegOffset(p, 0, ty),
                }
                .emit(&[], sink, emit_info, state);
                Inst::Fence {
                    fm: Default::default(),
                    pred: Inst::FENCE_REQ_R,
                    succ: Inst::FENCE_REQ_R | Inst::FENCE_REQ_W,
                }
                .emit(&[], sink, emit_info, state);
            }
            &Inst::AtomicStore { src, ty, p } => {
                let src = allocs.next(src);
                let p = allocs.next(p);
                Inst::Fence {
                    fm: Default::default(),
                    pred: Inst::FENCE_REQ_R | Inst::FENCE_REQ_W,
                    succ: Inst::FENCE_REQ_W,
                }
                .emit(&[], sink, emit_info, state);
                Inst::Store {
                    to: AMode::RegOffset(p, 0, ty),
                    op: StoreOP::from_type(ty),
                    flags: MemFlags::trusted(),
                    src,
                }
                .emit(&[], sink, emit_info, state);
            }
        };
        let end_off = sink.cur_offset();
        assert!(
            (end_off - start_off) <= Inst::worst_case_size(),
            "Inst:{:?} length:{} worst_case_size:{}",
            self,
            end_off - start_off,
            Inst::worst_case_size()
        );
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
