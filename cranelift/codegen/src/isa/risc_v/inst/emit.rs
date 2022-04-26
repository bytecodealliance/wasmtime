//! AArch64 ISA: binary code emission.

use crate::binemit::StackMap;
use crate::isa::risc_v::inst::*;
use std::collections::HashSet;

use super::*;
use crate::isa::risc_v::inst::{zero_reg, AluOPRRR};
use alloc::vec;
use regalloc::{Reg, Writable};

pub struct EmitInfo(settings::Flags);

impl EmitInfo {
    pub(crate) fn new(flags: settings::Flags) -> Self {
        Self(flags)
    }
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
    /*
        alloc some registers for load large constant, or something else.
        if exclusive is given, which we must not include the  exclusive register (it's aleady been allocted or something),so must skip it.
    */
    fn alloc_registers(
        exclusive: Option<HashSet<Writable<Reg>>>,
        amount: u8,
    ) -> Vec<Writable<Reg>> {
        let mut v = vec![];
        let available = s2_to_s11();
        debug_assert!(amount <= available.len() as u8);
        for r in available {
            if let Some(ref set) = exclusive {
                if set.contains(&r) {
                    continue;
                }
            }
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
                op: StoreOP::SD,
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
                op: LoadOP::LD,
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

    fn emit(&self, sink: &mut MachBuffer<Inst>, emit_info: &Self::Info, state: &mut EmitState) {
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
            // ADDI x0, x0, 0
            &Inst::Nop4 => {
                let x = Inst::AluRRImm12 {
                    alu_op: AluOPRRI::ADDI,
                    rd: Writable::from_reg(zero_reg()),
                    rs: zero_reg(),
                    imm12: Imm12::zero(),
                };
                x.emit(sink, emit_info, state)
            }

            &Inst::Lui { rd, ref imm } => {
                let x: u32 =
                    0b0110111 | (rd.to_reg().get_hw_encoding() as u32) << 7 | (imm.as_u32() << 12);
                sink.put4(x);
            }
            &Inst::AluRR { alu_op, rd, rs } => {
                let x = alu_op.op_code()
                    | (rd.to_reg().get_hw_encoding() as u32) << 7
                    | alu_op.funct3() << 12
                    | (rs.get_hw_encoding() as u32) << 15
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
                let x = alu_op.op_code()
                    | (rd.to_reg().get_hw_encoding() as u32) << 7
                    | alu_op.funct3() << 12
                    | (rs1.get_hw_encoding() as u32) << 15
                    | (rs2.get_hw_encoding() as u32) << 20
                    | alu_op.funct2() << 25
                    | (rs3.get_hw_encoding() as u32) << 27;

                sink.put4(x);
            }
            &Inst::AluRRR {
                alu_op,
                rd,
                rs1,
                rs2,
            } => {
                let x: u32 = alu_op.op_code()
                    | (rd.to_reg().as_real_reg().unwrap().get_hw_encoding() as u32) << 7
                    | (alu_op.funct3()) << 12
                    | (rs1.as_real_reg().unwrap().get_hw_encoding() as u32) << 15
                    | (rs2.as_real_reg().unwrap().get_hw_encoding() as u32) << 20
                    | alu_op.funct7() << 25;
                sink.put4(x);
            }
            &Inst::AluRRImm12 {
                alu_op,
                rd,
                rs,
                imm12,
            } => {
                let x = if let Some(funct6) = alu_op.option_funct6() {
                    alu_op.op_code()
                        | (rd.to_reg().as_real_reg().unwrap().get_hw_encoding() as u32) << 7
                        | alu_op.funct3() << 12
                        | (rs.as_real_reg().unwrap().get_hw_encoding() as u32) << 15
                        | (imm12.as_u32()) << 20
                        | funct6 << 26
                } else if let Some(funct7) = alu_op.option_funct7() {
                    alu_op.op_code()
                        | (rd.to_reg().as_real_reg().unwrap().get_hw_encoding() as u32) << 7
                        | alu_op.funct3() << 12
                        | (rs.as_real_reg().unwrap().get_hw_encoding() as u32) << 15
                        | (imm12.as_u32()) << 20
                        | funct7 << 25
                } else {
                    alu_op.op_code()
                        | (rd.to_reg().as_real_reg().unwrap().get_hw_encoding() as u32) << 7
                        | alu_op.funct3() << 12
                        | (rs.as_real_reg().unwrap().get_hw_encoding() as u32) << 15
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
                let offset = from.get_offset_with_state(state);
                if let Some(imm12) = Imm12::maybe_from_u64(offset as u64) {
                    x = op.op_code()
                        | (rd.to_reg().as_real_reg().unwrap().get_hw_encoding() as u32) << 7
                        | op.funct3() << 12
                        | (base.as_real_reg().unwrap().get_hw_encoding() as u32) << 15
                        | (imm12.as_u32()) << 20;
                    sink.put4(x);
                } else {
                    let mut insts: SmallInstVec<Inst> = SmallInstVec::new();
                    let registers = Inst::alloc_registers(None, 1);
                    insts.extend(Inst::push_registers(&registers));
                    //registers[0] = offset
                    insts.extend(Inst::load_constant_u64(registers[0], offset as u64));
                    //registers[0] += base
                    insts.push(Inst::AluRRR {
                        alu_op: AluOPRRR::ADD,
                        rd: registers[0],
                        rs1: registers[0].to_reg(),
                        rs2: base,
                    });
                    //lw rd , registers[0]
                    insts.push(Inst::Load {
                        op,
                        from: AMode::RegOffset(registers[0].to_reg(), Imm12::zero().into(), I64),
                        rd,
                        flags: MemFlags::new(),
                    });
                    // restore uses register
                    insts.extend(Inst::pop_registers(&registers));
                    for v in insts {
                        v.emit(sink, emit_info, state);
                    }
                }
            }
            &Inst::Store { op, src, flags, to } => {
                let base = to.get_base_register();
                let offset = to.get_offset_with_state(state);
                let x;
                if let Some(imm12) = Imm12::maybe_from_u64(offset as u64) {
                    x = op.op_code()
                        | (imm12.as_u32() & 0x1f) << 7
                        | op.funct3() << 12
                        | (base.get_hw_encoding() as u32) << 15
                        | (src.as_real_reg().unwrap().get_hw_encoding() as u32) << 20
                        | (imm12.as_u32() >> 5) << 25;
                    sink.put4(x);
                } else {
                    let mut insts: SmallInstVec<Inst> = smallvec![];
                    let registers = Inst::alloc_registers(None, 1);
                    insts.extend(Inst::push_registers(&registers));
                    insts.extend(Inst::load_constant_u64(registers[0], offset as u64));
                    // registers[0] = base + offset
                    insts.push(Inst::AluRRR {
                        alu_op: AluOPRRR::ADD,
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
                    insts.extend(Inst::pop_registers(&registers));
                    for v in insts {
                        v.emit(sink, emit_info, state);
                    }
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
            &Inst::Extend {
                rd,
                rn,
                signed,
                from_bits,
                to_bits,
            } => {
                //todo:: actual extend the value ;
                Inst::AluRRImm12 {
                    alu_op: AluOPRRI::ADDI,
                    rd: rd,
                    rs: rn,
                    imm12: Imm12::zero(),
                }
                .emit(sink, emit_info, state);
            }
            &Inst::AjustSp { amount } => {
                if let Some(imm) = Imm12::maybe_from_u64(amount as u64) {
                    Inst::AluRRImm12 {
                        alu_op: AluOPRRI::ADDI,
                        rd: writable_stack_reg(),
                        rs: stack_reg(),
                        imm12: imm,
                    }
                    .emit(sink, emit_info, state);
                } else {
                    let mut insts: SmallInstVec<Inst> = SmallInstVec::new();
                    let registers = Inst::alloc_registers(None, 1);
                    insts.extend(Inst::push_registers(&registers));
                    insts.extend(Inst::load_constant_u64(registers[0], amount as u64));
                    insts.push(Inst::AluRRR {
                        alu_op: AluOPRRR::ADD,
                        rd: writable_stack_reg(),
                        rs1: stack_reg(),
                        rs2: registers[0].to_reg(),
                    });
                    insts.extend(Inst::pop_registers(&registers));
                    for v in insts {
                        v.emit(sink, emit_info, state);
                    }
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
                sink.use_label_at_offset(start_off, dest.as_label().unwrap(), LabelUse::Jal20);
                sink.add_uncond_branch(start_off, start_off + 4, dest.as_label().unwrap());
                let x: u32 = (0b1101111) | (zero_reg().get_hw_encoding() as u32) << 7 | (0 << 12);
                sink.put4(x);
            }
            &Inst::CondBr {
                taken,
                not_taken,
                kind,
                ty,
            } => {
                let code = kind.emit();
                let code_inverse = kind.inverse().emit().to_le_bytes();
                sink.use_label_at_offset(start_off, taken.as_label().unwrap(), LabelUse::B12);
                sink.add_cond_branch(
                    start_off,
                    start_off + 4,
                    taken.as_label().unwrap(),
                    &code_inverse,
                );
                sink.put4(code);
                Inst::Jump { dest: not_taken }.emit(sink, emit_info, state);
            }
            &Inst::Mov { rd, rm } => {
                let x = Inst::AluRRImm12 {
                    alu_op: AluOPRRI::ORI,
                    rd: rd,
                    rs: rm,
                    imm12: Imm12::zero(),
                };
                x.emit(sink, emit_info, state);
            }

            &Inst::VirtualSPOffsetAdj { amount } => {
                state.virtual_sp_offset += amount;
            }

            _ => unimplemented!(),
        };

        let end_off = sink.cur_offset();
        debug_assert!((end_off - start_off) <= Inst::worst_case_size());
    }

    fn pretty_print(&self, mb_rru: Option<&RealRegUniverse>, state: &mut EmitState) -> String {
        self.print_with_state(mb_rru, state)
    }
}
