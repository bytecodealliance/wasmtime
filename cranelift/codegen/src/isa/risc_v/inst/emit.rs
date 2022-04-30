//! AArch64 ISA: binary code emission.

use crate::binemit::StackMap;
use crate::isa::risc_v::inst::*;
use std::collections::HashSet;

use crate::isa::risc_v::inst::{zero_reg, AluOPRRR};
use crate::machinst::{AllocationConsumer, Reg, Writable};
use regalloc2::Allocation;

use alloc::vec;

pub struct EmitInfo(settings::Flags);

impl EmitInfo {
    pub(crate) fn new(flags: settings::Flags) -> Self {
        Self(flags)
    }
}

fn machreg_to_gpr_num(m: Reg) -> u32 {
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
    pub(crate) fn construct_bit_not(rd: Writable<Reg>) -> Inst {
        unimplemented!()
    }

    /*
        1: alloc registers
        2: push into the stack
        3: do something with these registers
        4: restore the registers
    */
    pub(crate) fn do_something_with_registers(
        num: u8,
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
        if exclusive is given, which we must not include the  exclusive register (it's aleady been allocted or something),so must skip it.
    */
    fn alloc_registers(amount: u8) -> Vec<Writable<Reg>> {
        let mut v = vec![];
        let available = s1_to_s11();
        debug_assert!(amount <= available.len() as u8);
        for r in available {
            v.push(r);
            if v.len() == amount as usize {
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
        let mut allocs_c = AllocationConsumer::new(allocs);

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
                x.emit(allocs, sink, emit_info, state)
            }

            &Inst::Lui { rd, ref imm } => {
                let rd = allocs_c.next_writable(rd);
                let x: u32 =
                    0b0110111 | machreg_to_gpr_num(rd.to_reg()) << 7 | (imm.as_u32() << 12);
                sink.put4(x);
            }
            &Inst::AluRR { alu_op, rd, rs } => {
                let rd = allocs_c.next_writable(rd);
                let rs = allocs_c.next(rs);

                let x = alu_op.op_code()
                    | machreg_to_gpr_num(rs) << 7
                    | alu_op.funct3() << 12
                    | machreg_to_gpr_num(rd.to_reg()) << 15
                    | alu_op.rs2() << 20
                    | alu_op.funct7() << 25;
                sink.put4(x);
            }
            &Inst::AluRRRR {
                alu_op,
                rd,
                rs1,
                rs2,
                rs3,
            } => {
                let rd = allocs_c.next_writable(rd);
                let rs1 = allocs_c.next(rs1);
                let rs2 = allocs_c.next(rs2);
                let rs3 = allocs_c.next(rs3);
                let x = alu_op.op_code()
                    | machreg_to_gpr_num(rd.to_reg()) << 7
                    | alu_op.funct3() << 12
                    | machreg_to_gpr_num(rs1) << 15
                    | machreg_to_gpr_num(rs2) << 20
                    | alu_op.funct2() << 25
                    | machreg_to_gpr_num(rs3) << 27;

                sink.put4(x);
            }
            &Inst::AluRRR {
                alu_op,
                rd,
                rs1,
                rs2,
            } => {
                let rd = allocs_c.next_writable(rd);
                let rs1 = allocs_c.next(rs1);
                let rs2 = allocs_c.next(rs2);
                let x: u32 = alu_op.op_code()
                    | machreg_to_gpr_num(rd.to_reg()) << 7
                    | (alu_op.funct3()) << 12
                    | machreg_to_gpr_num(rs1) << 15
                    | machreg_to_gpr_num(rs2) << 20
                    | alu_op.funct7() << 25;
                sink.put4(x);
            }
            &Inst::AluRRImm12 {
                alu_op,
                rd,
                rs,
                imm12,
            } => {
                let rd = allocs_c.next_writable(rd);
                let rs = allocs_c.next(rs);

                let x = if let Some(funct6) = alu_op.option_funct6() {
                    alu_op.op_code()
                        | machreg_to_gpr_num(rd.to_reg()) << 7
                        | alu_op.funct3() << 12
                        | machreg_to_gpr_num(rs) << 15
                        | (imm12.as_u32()) << 20
                        | funct6 << 26
                } else if let Some(funct7) = alu_op.option_funct7() {
                    alu_op.op_code()
                        | machreg_to_gpr_num(rd.to_reg()) << 7
                        | alu_op.funct3() << 12
                        | machreg_to_gpr_num(rs) << 15
                        | (imm12.as_u32()) << 20
                        | funct7 << 25
                } else {
                    alu_op.op_code()
                        | machreg_to_gpr_num(rd.to_reg()) << 7
                        | alu_op.funct3() << 12
                        | machreg_to_gpr_num(rs) << 15
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
                let base = allocs_c.next(base);
                let offset = from.get_offset_with_state(state);
                if let Some(imm12) = Imm12::maybe_from_u64(offset as u64) {
                    x = op.op_code()
                        | machreg_to_gpr_num(rd.to_reg()) << 7
                        | op.funct3() << 12
                        | machreg_to_gpr_num(base) << 15
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
                            flags: MemFlags::new(),
                        });
                    })
                    .into_iter()
                    .for_each(|inst| inst.emit(allocs, sink, emit_info, state));
                }
            }
            &Inst::Store { op, src, flags, to } => {
                let base = to.get_base_register();
                let base = allocs_c.next(base);
                let rs = allocs_c.next(src);
                let offset = to.get_offset_with_state(state);
                let x;
                if let Some(imm12) = Imm12::maybe_from_u64(offset as u64) {
                    x = op.op_code()
                        | (imm12.as_u32() & 0x1f) << 7
                        | op.funct3() << 12
                        | machreg_to_gpr_num(base) << 15
                        | machreg_to_gpr_num(src) << 20
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
                            flags: MemFlags::new(),
                        });
                    })
                    .into_iter()
                    .for_each(|inst| inst.emit(allocs, sink, emit_info, state));
                }
            }
            &Inst::EpiloguePlaceholder => {
                unimplemented!("what should I Do.");
            }
            &Inst::Ret => {
                //jalr x0, x1, 0
                let x: u32 = (0b1100111) | (1 << 15);
                sink.put4(x);
            }
            &Inst::Extend { rd, rn, op } => {
                //todo:: actual extend the value;;
                // Inst::AluRRImm12 {
                //     alu_op: AluOPRRI::Addi,
                //     rd: rd,
                //     rs: rn,
                //     imm12: Imm12::zero(),
                // }
                // .emit(sink, emit_info, state);

                unreachable!("no need to extend the value.")
            }
            &Inst::AjustSp { amount } => {
                if let Some(imm) = Imm12::maybe_from_u64(amount as u64) {
                    Inst::AluRRImm12 {
                        alu_op: AluOPRRI::Addi,
                        rd: writable_stack_reg(),
                        rs: stack_reg(),
                        imm12: imm,
                    }
                    .emit(allocs, sink, emit_info, state);
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
                        inst.emit(allocs, sink, emit_info, state);
                    });
                }
            }
            &Inst::Call { ref info } => {
                unimplemented!("call not implamented.")
            }
            &Inst::CallInd { ref info } => {
                unimplemented!("call not implamented.")
            }
            &Inst::TrapIf {
                rs1,
                rs2,
                cond,
                trap_code,
            } => {
                unimplemented!("trap not implamented.")
            }
            &Inst::Trap { trap_code } => {
                unimplemented!("what is the trap code\n");
            }
            &Inst::Jump { dest } => {
                let code: u32 = (0b1101111) | (0 << 12);
                match dest {
                    BranchTarget::Label(lable) => {
                        sink.use_label_at_offset(start_off, lable, LabelUse::Jal20);
                        sink.add_uncond_branch(start_off, start_off + 4, lable);
                        sink.put4(code);
                    }
                    BranchTarget::Patch => {
                        // do nothing
                        panic!("need patch compiler internal fetal error")
                    }
                    BranchTarget::ResolvedOffset(offset) => {
                        if offset != 0 {
                            if LabelUse::Jal20.offset_in_range(offset) {
                                let mut code = code.to_le_bytes();
                                LabelUse::Jal20.patch_raw_offset(&mut code, offset);
                            } else {
                                Inst::construct_auipc_and_jalr(offset)
                                    .into_iter()
                                    .for_each(|i| i.emit(allocs, sink, emit_info, state));
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
                kind.rs1 = allocs_c.next(kind.rs1);
                kind.rs2 = allocs_c.next(kind.rs2);
                match taken {
                    BranchTarget::Label(label) => {
                        let code = kind.emit();
                        let code_inverse = kind.inverse().emit().to_le_bytes();
                        sink.use_label_at_offset(start_off, label, LabelUse::B12);
                        sink.add_cond_branch(start_off, start_off + 4, label, &code_inverse);
                        sink.put4(code);
                    }
                    BranchTarget::Patch => panic!("need patch compiler internal fetal error"),
                    BranchTarget::ResolvedOffset(offset) => {
                        if LabelUse::B12.offset_in_range(offset) {
                            let code = kind.emit();
                            let mut code = code.to_le_bytes();
                            LabelUse::B12.patch_raw_offset(&mut code, offset);
                            code.into_iter().for_each(|b| sink.put1(b));
                        } else {
                            let code = kind.emit();
                            // jump is zero, this means when condition is met , no jump
                            // fallthrough to next instruction which is the long jump.
                            sink.put4(code);
                            Inst::construct_auipc_and_jalr(offset)
                                .into_iter()
                                .for_each(|i| i.emit(allocs, sink, emit_info, state));
                        }
                    }
                }
                Inst::Jump { dest: not_taken }.emit(allocs, sink, emit_info, state);
            }

            &Inst::Mov { rd, rm, ty } => {
                let rd = allocs_c.next_writable(rd);
                let rm = allocs_c.next(rm);
                /*
                    todo::
                    it is possible for rd and rm have diffent RegClass?????
                */
                if rd.to_reg() != rm {
                    if ty.is_float() {
                        let mut insts = SmallInstVec::new();
                        if ty == F32 {
                            insts.push(Inst::AluRR {
                                alu_op: AluOPRR::FmvXW,
                                rd: writable_spilltmp_reg(),
                                rs: rm,
                            });
                            insts.push(Inst::AluRR {
                                alu_op: AluOPRR::FmvWX,
                                rd: rd,
                                rs: spilltmp_reg(),
                            });
                        } else {
                            insts.push(Inst::AluRR {
                                alu_op: AluOPRR::FmvXD,
                                rd: writable_spilltmp_reg(),
                                rs: rm,
                            });
                            insts.push(Inst::AluRR {
                                alu_op: AluOPRR::FmvDX,
                                rd: rd,
                                rs: spilltmp_reg(),
                            });
                        }
                        insts
                            .into_iter()
                            .for_each(|inst| inst.emit(allocs, sink, emit_info, state));
                    } else {
                        let x = Inst::AluRRImm12 {
                            alu_op: AluOPRRI::Ori,
                            rd: rd,
                            rs: rm,
                            imm12: Imm12::zero(),
                        };
                        x.emit(allocs, sink, emit_info, state);
                    }
                }
            }

            &Inst::VirtualSPOffsetAdj { amount } => {
                log::trace!(
                    "virtual sp offset adjusted by {} -> {}",
                    amount,
                    state.virtual_sp_offset + amount
                );
                state.virtual_sp_offset += amount;
            }
            // &Inst::FloatFlagOperation { op, rs, imm, rd } => {
            //     let x = op.op_code()
            //         | machreg_to_gpr_num(rs) << 7
            //         | op.funct3() << 12
            //         | op.rs1(rs) << 15
            //         | op.imm12(imm) << 20;
            //     sink.put4(x);
            // }
            &Inst::Atomic {
                op,
                rd,
                addr,
                src,
                aq,
                rl,
            } => {
                let x = op.op_code()
                    | machreg_to_gpr_num(rd.to_reg()) << 7
                    | op.funct3() << 12
                    | machreg_to_gpr_num(addr) << 15
                    | machreg_to_gpr_num(src) << 20
                    | op.funct7(aq, rl) << 25;

                sink.put4(x);
            }

            /*
                todo
                why does fence look like have parameter.
                0000 pred succ 00000 000 00000 0001111
                what is pred and succ???????
            */
            &Inst::Fence => sink.put4(0x0ff0000f),
            &Inst::FenceI => sink.put4(0x0000100f),
            &Inst::Auipc { rd, imm } => {
                let rd = allocs_c.next_writable(rd);

                let x = 0b0010111 | machreg_to_gpr_num(rd.to_reg()) << 7 | imm.as_u32() << 12;
                sink.put4(x);
            }
            // &Inst::LoadExtName { rd, name, offset } => todo!(),
            &Inst::LoadAddr { rd, mem } => {
                let base = mem.get_base_register();
                let base = allocs_c.next(base);
                let offset = mem.get_offset_with_state(state);
                if let Some(offset) = Imm12::maybe_from_u64(offset as u64) {
                    Inst::AluRRImm12 {
                        alu_op: AluOPRRI::Addi,
                        rd: rd,
                        rs: base,
                        imm12: offset,
                    }
                    .emit(allocs, sink, emit_info, state);
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
                    .for_each(|inst| inst.emit(allocs, sink, emit_info, state));
                }
            }
            _ => todo!(),
        };

        let end_off = sink.cur_offset();
        debug_assert!((end_off - start_off) <= Inst::worst_case_size());
    }

    fn pretty_print_inst(&self, allocs: &[Allocation], state: &mut Self::State) -> String {
        let mut allocs = AllocationConsumer::new(allocs);
        self.print_with_state(state, &mut allocs)
    }
}
