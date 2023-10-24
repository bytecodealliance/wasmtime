//! zkASM ISA: binary code emission.

use crate::binemit::StackMap;
use crate::ir::{self, RelSourceLoc, TrapCode};
use crate::isa::zkasm::inst::*;
use crate::machinst::{AllocationConsumer, Reg, Writable};
use crate::trace;
use cranelift_control::ControlPlane;
use cranelift_entity::EntityRef;
use regalloc2::Allocation;

pub struct EmitInfo {
    shared_flag: settings::Flags,
    isa_flags: super::super::zkasm_settings::Flags,
}

impl EmitInfo {
    pub(crate) fn new(
        shared_flag: settings::Flags,
        isa_flags: super::super::zkasm_settings::Flags,
    ) -> Self {
        Self {
            shared_flag,
            isa_flags,
        }
    }
}

/// load constant by put the constant in the code stream.
/// calculate the pc and using load instruction.
/// This is only allow used in the emit stage.
/// Because of those instruction must execute together.
/// see https://github.com/bytecodealliance/wasmtime/pull/5612
#[derive(Clone, Copy)]
pub(crate) enum LoadConstant {
    U32(u32),
    U64(u64),
}

#[allow(unused)]
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

    pub(crate) fn load_constant<F: FnMut(Type) -> Writable<Reg>>(
        self,
        rd: Writable<Reg>,
        alloc_tmp: &mut F,
    ) -> SmallInstVec<Inst> {
        todo!()
        /*
        let mut insts = SmallInstVec::new();
        // get current pc.
        let pc = alloc_tmp(I64);
        insts.push(Inst::Auipc {
            rd: pc,
            imm: Imm20 { bits: 0 },
        });
        // load
        insts.push(Inst::Load {
            rd,
            op: self.load_op(),
            flags: MemFlags::new(),
            from: AMode::RegOffset(pc.to_reg(), 12, self.load_ty()),
        });
        let data = self.to_le_bytes();
        // jump over.
        insts.push(Inst::Jal {
            dest: BranchTarget::ResolvedOffset(Inst::INSTRUCTION_SIZE + data.len() as i32),
        });
        insts.push(Inst::RawData { data });
        insts
        */
    }

    // load and perform an extra add.
    pub(crate) fn load_constant_and_add(self, rd: Writable<Reg>, rs: Reg) -> SmallInstVec<Inst> {
        todo!()
        /*
        let mut insts = self.load_constant(rd, &mut |_| rd);
        insts.push(Inst::AluRRR {
            alu_op: AluOPRRR::Add,
            rd,
            rs1: rd.to_reg(),
            rs2: rs,
        });
        insts
        */
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
    cur_srcloc: RelSourceLoc,
    /// Only used during fuzz-testing. Otherwise, it is a zero-sized struct and
    /// optimized away at compiletime. See [cranelift_control].
    ctrl_plane: ControlPlane,
}

impl EmitState {
    fn take_stack_map(&mut self) -> Option<StackMap> {
        self.stack_map.take()
    }

    fn clear_post_insn(&mut self) {
        self.stack_map = None;
    }

    fn cur_srcloc(&self) -> RelSourceLoc {
        self.cur_srcloc
    }
}

impl MachInstEmitState<Inst> for EmitState {
    fn new(
        abi: &Callee<crate::isa::zkasm::abi::ZkAsmMachineDeps>,
        ctrl_plane: ControlPlane,
    ) -> Self {
        EmitState {
            virtual_sp_offset: 0,
            nominal_sp_to_fp: abi.frame_size() as i64,
            stack_map: None,
            cur_srcloc: RelSourceLoc::default(),
            ctrl_plane,
        }
    }

    fn pre_safepoint(&mut self, stack_map: StackMap) {
        self.stack_map = Some(stack_map);
    }

    fn pre_sourceloc(&mut self, srcloc: RelSourceLoc) {
        self.cur_srcloc = srcloc;
    }

    fn ctrl_plane_mut(&mut self) -> &mut ControlPlane {
        &mut self.ctrl_plane
    }

    fn take_ctrl_plane(self) -> ControlPlane {
        self.ctrl_plane
    }

    fn on_new_block(&mut self) {}
}

#[allow(unused)]
impl Inst {
    /// construct a "imm - rs".
    pub(crate) fn construct_imm_sub_rs(rd: Writable<Reg>, imm: u64, rs: Reg) -> SmallInstVec<Inst> {
        todo!()
        /* let mut insts = Inst::load_constant_u64(rd, imm, &mut |_| rd);
        insts.push(Inst::AluRRR {
            alu_op: AluOPRRR::Sub,
            rd,
            rs1: rd.to_reg(),
            rs2: rs,
        });
        insts */
    }

    /// Load int mask.
    /// If ty is int then 0xff in rd.
    pub(crate) fn load_int_mask(rd: Writable<Reg>, ty: Type) -> SmallInstVec<Inst> {
        todo!()
        /* let mut insts = SmallInstVec::new();
        assert!(ty.is_int() && ty.bits() <= 64);
        match ty {
            I64 => {
                insts.push(Inst::load_imm12(rd, Imm12::from_bits(-1)));
            }
            I32 | I16 => {
                insts.push(Inst::load_imm12(rd, Imm12::from_bits(-1)));
                insts.push(Inst::Extend {
                    rd: rd,
                    rn: rd.to_reg(),
                    signed: false,
                    from_bits: ty.bits() as u8,
                    to_bits: 64,
                });
            }
            I8 => {
                insts.push(Inst::load_imm12(rd, Imm12::from_bits(255)));
            }
            _ => unreachable!("ty:{:?}", ty),
        }
        insts */
    }
    ///  inverse all bit
    pub(crate) fn construct_bit_not(rd: Writable<Reg>, rs: Reg) -> Inst {
        todo!()
    }

    // emit a float is not a nan.
    pub(crate) fn emit_not_nan(rd: Writable<Reg>, rs: Reg, ty: Type) -> Inst {
        todo!()
    }

    pub(crate) fn emit_fabs(rd: Writable<Reg>, rs: Reg, ty: Type) -> Inst {
        todo!()
    }
    /// If a float is zero.
    pub(crate) fn emit_if_float_not_zero(
        tmp: Writable<Reg>,
        rs: Reg,
        ty: Type,
        taken: BranchTarget,
        not_taken: BranchTarget,
    ) -> SmallInstVec<Inst> {
        todo!()
        /* let mut insts = SmallInstVec::new();
        let class_op = if ty == F32 {
            FpuOPRR::FclassS
        } else {
            FpuOPRR::FclassD
        };
        insts.push(Inst::FpuRR {
            alu_op: class_op,
            frm: None,
            rd: tmp,
            rs: rs,
        });
        insts.push(Inst::AluRRImm12 {
            alu_op: AluOPRRI::Andi,
            rd: tmp,
            rs: tmp.to_reg(),
            imm12: Imm12::from_bits(FClassResult::is_zero_bits() as i16),
        });
        insts.push(Inst::CondBr {
            taken,
            not_taken,
            kind: IntegerCompare {
                kind: IntCC::Equal,
                rs1: tmp.to_reg(),
                rs2: zero_reg(),
            },
        });
        insts */
    }
    pub(crate) fn emit_fneg(rd: Writable<Reg>, rs: Reg, ty: Type) -> Inst {
        todo!()
    }

    pub(crate) fn lower_br_icmp(
        cc: IntCC,
        a: ValueRegs<Reg>,
        b: ValueRegs<Reg>,
        taken: BranchTarget,
        not_taken: BranchTarget,
        ty: Type,
    ) -> SmallInstVec<Inst> {
        todo!()
        /* let mut insts = SmallInstVec::new();
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
                // if high part not equal,
                // then we can go to not_taken otherwise fallthrough.
                insts.push(Inst::CondBr {
                    taken: not_taken,
                    not_taken: BranchTarget::zero(),
                    kind: high(IntCC::NotEqual),
                });
                // the rest part.
                insts.push(Inst::CondBr {
                    taken,
                    not_taken,
                    kind: low(IntCC::Equal),
                });
            }

            IntCC::NotEqual => {
                // if the high part not equal ,
                // we know the whole must be not equal,
                // we can goto the taken part , otherwise fallthrought.
                insts.push(Inst::CondBr {
                    taken,
                    not_taken: BranchTarget::zero(), //  no branch
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
        }
        insts */
    }
}

fn put_string(s: &str, sink: &mut MachBuffer<Inst>) {
    sink.put_data("  ".as_bytes());
    sink.put_data(s.as_bytes());
}

fn access_reg_with_offset(reg: Reg, offset: i64) -> String {
    let name = reg_name(reg);
    match offset.cmp(&0) {
        core::cmp::Ordering::Less => format!("{name} - {}", -offset),
        core::cmp::Ordering::Equal => name,
        core::cmp::Ordering::Greater => format!("{name} + {}", offset),
    }
}

#[allow(unused)]
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
                todo!() /* let x = Inst::AluRRImm12 {
                            alu_op: AluOPRRI::Addi,
                            rd: Writable::from_reg(zero_reg()),
                            rs: zero_reg(),
                            imm12: Imm12::zero(),
                        };
                        x.emit(&[], sink, emit_info, state) */
            }
            &Inst::Label { imm } => {
                sink.put_data(format!("label_{imm}:\n").as_bytes());
            }
            &Inst::RawData { ref data } => {
                // Right now we only put a u32 or u64 in this instruction.
                // It is not very long, no need to check if need `emit_island`.
                // If data is very long , this is a bug because RawData is typecial
                // use to load some data and rely on some positon in the code stream.
                // and we may exceed `Inst::worst_case_size`.
                // for more information see https://github.com/bytecodealliance/wasmtime/pull/5612.
                todo!() // sink.put_data(&data[..]);
            }
            &Inst::Lui { rd, ref imm } => {
                todo!() /* let rd = allocs.next_writable(rd);
                        let x: u32 = 0b0110111 | reg_to_gpr_num(rd.to_reg()) << 7 | (imm.as_u32() << 12);
                        sink.put4(x); */
            }
            &Inst::LoadConst32 { rd, imm } => {
                let rd = allocs.next_writable(rd);
                put_string(&format!("{imm} => {}\n", reg_name(rd.to_reg())), sink);
            }
            &Inst::LoadConst64 { rd, imm } => {
                let rd = allocs.next_writable(rd);
                put_string(&format!("{imm} => {}\n", reg_name(rd.to_reg())), sink);
            }
            &Inst::Unwind { ref inst } => {
                put_string(&format!("Unwind\n"), sink);
                // sink.add_unwind(inst.clone());
            }
            &Inst::DummyUse { reg } => {
                todo!() // allocs.next(reg);
            }
            &Inst::AddImm32 { rd, src1, src2 } => {
                let rd = allocs.next(rd.to_reg());
                // TODO(akashin): Should we have a function for `bits` field?
                put_string(
                    &format!("{} + {} => {}\n", src1.bits, src2.bits, reg_name(rd)),
                    sink,
                );
            }
            &Inst::MulArith { rd, rs1, rs2 } => {
                let rs1 = allocs.next(rs1);
                let rs2 = allocs.next(rs2);
                debug_assert_eq!(rs1, a0());
                debug_assert_eq!(rs2, b0());
                let rd = allocs.next_writable(rd);
                // Arith asserts that A * B + C = op + 2^256 * D.
                // Now ZKVM is 256-bit and wasm max type is 64 bit, so it would never be
                // 256 bit overflow. But in future we will need here something like:
                // put_string(
                //   &format!("${{_mulArith / (2 ** 64)}} => D :ARITH\n", reg_name(rd.to_reg())),
                //    sink,
                //);
                // put_string(
                //   &format!("${{_mulArith % (2 ** 64)}} => {} :ARITH\n", reg_name(rd.to_reg())),
                //    sink,
                //);
                // For now we will just clear D in case it was something in it
                put_string("0 => D\n", sink);
                put_string("0 => C\n", sink);
                put_string("$${var _mulArith = A * B}\n", sink);
                put_string(
                    &format!("${{_mulArith}} => {} :ARITH\n", reg_name(rd.to_reg())),
                    sink,
                );
            }
            &Inst::AluRRR {
                alu_op,
                rd,
                rs1,
                rs2,
            } => {
                let rs1 = allocs.next(rs1);
                let rs2 = allocs.next(rs2);
                debug_assert_eq!(rs1, a0());
                debug_assert_eq!(rs2, b0());
                let rd = allocs.next_writable(rd);
                put_string(
                    &format!("$ => {} :{}\n", reg_name(rd.to_reg()), alu_op.op_name()),
                    sink,
                );
            }
            &Inst::Load {
                rd,
                op,
                from,
                flags,
            } => {
                let from = from.clone().with_allocs(&mut allocs);
                let base = from.get_base_register();
                let offset = from.get_offset_with_state(state);
                let rd = allocs.next_writable(rd);
                let insn = match from {
                    AMode::RegOffset(r, off, _) => format!(
                        "$ => {} :MLOAD({})\n",
                        reg_name(rd.to_reg()),
                        access_reg_with_offset(r, off),
                    ),
                    AMode::SPOffset(off, _) | AMode::NominalSPOffset(off, _) => {
                        format!(
                            "$ => {} :MLOAD({})\n",
                            reg_name(rd.to_reg()),
                            access_reg_with_offset(stack_reg(), off),
                        )
                    }
                    AMode::FPOffset(off, _) => {
                        format!(
                            "$ => {} :MLOAD({})\n",
                            reg_name(rd.to_reg()),
                            access_reg_with_offset(fp_reg(), off),
                        )
                    }
                    // FIXME: these don't actually produce valid zkASM
                    AMode::Const(_) => format!("$ => {} :MLOAD({})\n", reg_name(rd.to_reg()), from),
                    AMode::Label(_) => format!("$ => {} :MLOAD({})\n", reg_name(rd.to_reg()), from),
                };
                put_string(&insn, sink);
            }
            &Inst::Store { op, src, flags, to } => {
                let to = to.clone().with_allocs(&mut allocs);
                let src = allocs.next(src);

                let insn = match to {
                    AMode::RegOffset(r, off, _) => format!(
                        "{} :MSTORE({})\n",
                        reg_name(src),
                        access_reg_with_offset(r, off),
                    ),
                    AMode::SPOffset(off, _) | AMode::NominalSPOffset(off, _) => {
                        format!(
                            "{} :MSTORE({})\n",
                            reg_name(src),
                            access_reg_with_offset(stack_reg(), off),
                        )
                    }
                    AMode::FPOffset(off, _) => {
                        format!(
                            "{} :MSTORE({})\n",
                            reg_name(src),
                            access_reg_with_offset(fp_reg(), off),
                        )
                    }
                    // FIXME: these don't actually produce valid zkASM
                    AMode::Const(_) => format!("{} :MSTORE({})\n", reg_name(src), to),
                    AMode::Label(_) => format!("{} :MSTORE({})\n", reg_name(src), to),
                };
                put_string(&insn, sink);
            }
            &Inst::Args { .. } => {
                // Nothing: this is a pseudoinstruction that serves
                // only to constrain registers at a certain point.
            }
            &Inst::Ret {
                stack_bytes_to_pop, ..
            } => {
                if stack_bytes_to_pop != 0 {
                    Inst::ReleaseSp {
                        amount: stack_bytes_to_pop,
                    }
                    .emit(&[], sink, emit_info, state);
                }
                // TODO: use the `RETURN` instruction instead!
                // put_string(&format!("RETURN\n"), sink);
                put_string(":JMP(RR)\n", sink);
            }

            &Inst::Extend {
                rd,
                rn,
                signed,
                from_bits,
                to_bits: _to_bits,
            } => {
                todo!() /* let rn = allocs.next(rn);
                        let rd = allocs.next_writable(rd);
                        let mut insts = SmallInstVec::new();
                        let shift_bits = (64 - from_bits) as i16;
                        let is_u8 = || from_bits == 8 && signed == false;
                        if is_u8() {
                            // special for u8.
                            insts.push(Inst::AluRRImm12 {
                                alu_op: AluOPRRI::Andi,
                                rd,
                                rs: rn,
                                imm12: Imm12::from_bits(255),
                            });
                        } else {
                            insts.push(Inst::AluRRImm12 {
                                alu_op: AluOPRRI::Slli,
                                rd,
                                rs: rn,
                                imm12: Imm12::from_bits(shift_bits),
                            });
                            insts.push(Inst::AluRRImm12 {
                                alu_op: if signed {
                                    AluOPRRI::Srai
                                } else {
                                    AluOPRRI::Srli
                                },
                                rd,
                                rs: rd.to_reg(),
                                imm12: Imm12::from_bits(shift_bits),
                            });
                        }
                        insts
                            .into_iter()
                            .for_each(|i| i.emit(&[], sink, emit_info, state)); */
            }
            &Inst::ReleaseSp { amount } => {
                // Stack is growing "up" in zkASM contrary to traditional architectures.
                // Furthermore, addressing is done in slots rather than bytes.
                //
                // FIXME: add helper functions to implement these conversions.
                let amount = amount.checked_div(8).unwrap();
                put_string(&format!("SP - {amount} => SP\n"), sink);
            }
            &Inst::ReserveSp { amount } => {
                // Stack is growing "up" in zkASM contrary to traditional architectures.
                // Furthermore, addressing is done in slots rather than bytes.
                //
                // FIXME: add helper functions to implement these conversions.
                let amount = amount.checked_div(8).unwrap();
                put_string(&format!("SP + {amount} => SP\n"), sink);
            }
            &Inst::Call { ref info } => {
                // call
                match info.dest {
                    ExternalName::User(name) => {
                        // For now we only support calls.
                        assert!(info.opcode.is_call());
                        sink.add_call_site(info.opcode);
                        sink.add_reloc(Reloc::RiscvCall, &info.dest, 0);
                        // This will be patched externally to do a necessary jump.
                        put_string(&format!("; CALL {name}\n"), sink);

                        // match name.index() {
                        //     // Special case for ASSERT call.
                        //     0 => {
                        //         Inst::Mov {
                        //             ty: types::I64,
                        //             rd: regs::writable_a0(),
                        //             rm: info.uses[0].preg,
                        //         }
                        //         .emit(&[], sink, emit_info, state);
                        //         put_string(
                        //             &format!("{} :ASSERT\n", reg_name(info.uses[1].preg)),
                        //             sink,
                        //         );
                        //     }
                        //     v => {
                        //         Inst::Jal {
                        //             dest: BranchTarget::Label(MachLabel::new(v)),
                        //         }
                        //         .emit(&[], sink, emit_info, state);
                        //     }
                        // };

                        // if let Some(s) = state.take_stack_map() {
                        //     sink.add_stack_map(StackMapExtent::UpcomingBytes(8), s);
                        // }
                        // Inst::construct_auipc_and_jalr(
                        //     Some(writable_link_reg()),
                        //     writable_link_reg(),
                        //     0,
                        // )
                        // .into_iter()
                        // .for_each(|i| i.emit(&[], sink, emit_info, state));
                    }
                    ExternalName::LibCall(..)
                    | ExternalName::TestCase { .. }
                    | ExternalName::KnownSymbol(..) => {
                        unimplemented!();
                        // use indirect call. it is more simple.
                        // load ext name.
                        // Inst::LoadExtName {
                        //     rd: writable_spilltmp_reg2(),
                        //     name: Box::new(info.dest.clone()),
                        //     offset: 0,
                        // }
                        // .emit(&[], sink, emit_info, state);
                        //
                        // if let Some(s) = state.take_stack_map() {
                        //     sink.add_stack_map(StackMapExtent::UpcomingBytes(4), s);
                        // }
                        // if info.opcode.is_call() {
                        //     sink.add_call_site(info.opcode);
                        // }
                        // call
                        // Inst::Jalr {
                        //     rd: writable_link_reg(),
                        //     base: spilltmp_reg2(),
                        //     offset: Imm12::zero(),
                        // }
                        // .emit(&[], sink, emit_info, state);
                    }
                }

                let callee_pop_size = i64::from(info.callee_pop_size);
                state.virtual_sp_offset -= callee_pop_size;
                trace!(
                    "call adjusts virtual sp offset by {callee_pop_size} -> {}",
                    state.virtual_sp_offset
                );
            }
            &Inst::CallInd { ref info } => {
                // let rn = allocs.next(info.rn);
                // put_string(&format!("CALL {}, {:?}\n", reg_name(rn), info.uses), sink);

                dbg!(info);
                todo!();
                // For now we only support calls.
                // assert!(info.opcode.is_call());

                /*
                if let Some(s) = state.take_stack_map() {
                    sink.add_stack_map(StackMapExtent::UpcomingBytes(4), s);
                }

                Inst::Jalr {
                    rd: writable_link_reg(),
                    base: rn,
                    offset: Imm12::zero(),
                }
                .emit(&[], sink, emit_info, state);

                let callee_pop_size = i64::from(info.callee_pop_size);
                state.virtual_sp_offset -= callee_pop_size;
                trace!(
                    "call adjusts virtual sp offset by {callee_pop_size} -> {}",
                    state.virtual_sp_offset
                ); */
            }

            &Inst::ReturnCall {
                ref callee,
                ref info,
            } => {
                todo!() /* emit_return_call_common_sequence(
                            &mut allocs,
                            sink,
                            emit_info,
                            state,
                            info.new_stack_arg_size,
                            info.old_stack_arg_size,
                            &info.uses,
                        );

                        sink.add_call_site(ir::Opcode::ReturnCall);
                        sink.add_reloc(Reloc::RiscvCall, &callee, 0);
                        Inst::construct_auipc_and_jalr(None, writable_spilltmp_reg(), 0)
                            .into_iter()
                            .for_each(|i| i.emit(&[], sink, emit_info, state));

                        // `emit_return_call_common_sequence` emits an island if
                        // necessary, so we can safely disable the worst-case-size check
                        // in this case.
                        start_off = sink.cur_offset(); */
            }

            &Inst::ReturnCallInd { callee, ref info } => {
                todo!() /* let callee = allocs.next(callee);

                        emit_return_call_common_sequence(
                            &mut allocs,
                            sink,
                            emit_info,
                            state,
                            info.new_stack_arg_size,
                            info.old_stack_arg_size,
                            &info.uses,
                        );

                        Inst::Jalr {
                            rd: writable_zero_reg(),
                            base: callee,
                            offset: Imm12::zero(),
                        }
                        .emit(&[], sink, emit_info, state);

                        // `emit_return_call_common_sequence` emits an island if
                        // necessary, so we can safely disable the worst-case-size check
                        // in this case.
                        start_off = sink.cur_offset(); */
            }

            &Inst::Jal { dest } => {
                match dest {
                    BranchTarget::Label(label) => {
                        // TODO: the following two lines allow eg. optimizing out jump-to-here
                        // sink.use_label_at_offset(start_off, label, LabelUse::Jal20);
                        // sink.add_uncond_branch(start_off, start_off + 4, label);
                        put_string(&format!(":JMP(label_{})\n", label.index()), sink);
                    }
                    BranchTarget::ResolvedOffset(offset) => {
                        todo!() /*
                                let offset = offset as i64;
                                if offset != 0 {
                                    if LabelUse::Jal20.offset_in_range(offset) {
                                        let mut code = code.to_le_bytes();
                                        LabelUse::Jal20.patch_raw_offset(&mut code, offset);
                                        sink.put_data(&code[..]);
                                    } else {
                                        Inst::construct_auipc_and_jalr(
                                            None,
                                            writable_spilltmp_reg(),
                                            offset,
                                        )
                                        .into_iter()
                                        .for_each(|i| i.emit(&[], sink, emit_info, state));
                                    }
                                } else {
                                    // CondBr often generate Jal {dest : 0}, means otherwise no jump.
                                } */
                    }
                }
            }
            &Inst::CondBr {
                taken,
                not_taken,
                mut kind,
            } => {
                kind.rs1 = allocs.next(kind.rs1);
                kind.rs2 = allocs.next(kind.rs2);
                // TODO(akashin): Support other types of comparisons.
                assert!(matches!(kind.kind, IntCC::NotEqual));
                assert_eq!(kind.rs2, zero_reg());
                match taken {
                    BranchTarget::Label(label) => {
                        put_string(
                            &format!("{} :JMPNZ(label_{})\n", reg_name(kind.rs1), label.index()),
                            sink,
                        );

                        // let code = kind.emit();
                        // let code_inverse = kind.inverse().emit().to_le_bytes();
                        // sink.use_label_at_offset(start_off, label, LabelUse::B12);
                        // sink.add_cond_branch(start_off, start_off + 4, label, &code_inverse);
                        // sink.put4(code);
                    }
                    BranchTarget::ResolvedOffset(offset) => {
                        assert!(offset != 0);
                        todo!();

                        // if LabelUse::B12.offset_in_range(offset as i64) {
                        //     let code = kind.emit();
                        //     let mut code = code.to_le_bytes();
                        //     LabelUse::B12.patch_raw_offset(&mut code, offset as i64);
                        //     sink.put_data(&code[..])
                        // } else {
                        //     let mut code = kind.emit().to_le_bytes();
                        //     // jump over the condbr , 4 bytes.
                        //     LabelUse::B12.patch_raw_offset(&mut code[..], 4);
                        //     sink.put_data(&code[..]);
                        //     Inst::construct_auipc_and_jalr(
                        //         None,
                        //         writable_spilltmp_reg(),
                        //         offset as i64,
                        //     )
                        //     .into_iter()
                        //     .for_each(|i| i.emit(&[], sink, emit_info, state));
                        // }
                    }
                }
                // TODO(akashin): Can also merge this as an else in jump.
                Inst::Jal { dest: not_taken }.emit(&[], sink, emit_info, state);
            }

            &Inst::Mov { rd, rm, ty } => {
                if rd.to_reg() == rm {
                    return;
                }

                let rm = allocs.next(rm);
                let rd = allocs.next_writable(rd);
                put_string(
                    &format!("{} => {}\n", reg_name(rm), reg_name(rd.to_reg())),
                    sink,
                );

                // match rm.class() {
                //     RegClass::Int => Inst::AluRRImm12 {
                //         alu_op: AluOPRRI::Ori,
                //         rd: rd,
                //         rs: rm,
                //         imm12: Imm12::zero(),
                //     },
                //     RegClass::Float => Inst::FpuRRR {
                //         alu_op: if ty == F32 {
                //             FpuOPRRR::FsgnjS
                //         } else {
                //             FpuOPRRR::FsgnjD
                //         },
                //         frm: None,
                //         rd: rd,
                //         rs1: rm,
                //         rs2: rm,
                //     },
                //     RegClass::Vector => Inst::VecAluRRImm5 {
                //         op: VecAluOpRRImm5::VmvrV,
                //         vd: rd,
                //         vs2: rm,
                //         // Imm 0 means copy 1 register.
                //         imm: Imm5::maybe_from_i8(0).unwrap(),
                //         mask: VecOpMasking::Disabled,
                //         // Vstate for this instruction is ignored.
                //         vstate: VState::from_type(ty),
                //     },
                // }
                // .emit(&[], sink, emit_info, state);
            }

            &Inst::MovFromPReg { rd, rm } => {
                todo!() /* debug_assert!([px_reg(2), px_reg(8)].contains(&rm));
                        let rd = allocs.next_writable(rd);
                        let x = Inst::AluRRImm12 {
                            alu_op: AluOPRRI::Ori,
                            rd,
                            rs: Reg::from(rm),
                            imm12: Imm12::zero(),
                        };
                        x.emit(&[], sink, emit_info, state); */
            }

            &Inst::BrTable {
                index,
                tmp1,
                tmp2,
                ref targets,
            } => {
                todo!() /* let index = allocs.next(index);
                        let tmp1 = allocs.next_writable(tmp1);
                        let tmp2 = allocs.next_writable(tmp2);
                        let ext_index = writable_spilltmp_reg();

                        // The default target is passed in as the 0th element of `targets`
                        // separate it here for clarity.
                        let default_target = targets[0];
                        let targets = &targets[1..];

                        // We emit a bounds check on the index, if the index is larger than the number of
                        // jump table entries, we jump to the default block.  Otherwise we compute a jump
                        // offset by multiplying the index by 8 (the size of each entry) and then jump to
                        // that offset. Each jump table entry is a regular auipc+jalr which we emit sequentially.
                        //
                        // Build the following sequence:
                        //
                        // extend_index:
                        //     zext.w  ext_index, index
                        // bounds_check:
                        //     li      tmp, n_labels
                        //     bltu    ext_index, tmp, compute_target
                        // jump_to_default_block:
                        //     auipc   pc, 0
                        //     jalr    zero, pc, default_block
                        // compute_target:
                        //     auipc   pc, 0
                        //     slli    tmp, ext_index, 3
                        //     add     pc, pc, tmp
                        //     jalr    zero, pc, 0x10
                        // jump_table:
                        //     ; This repeats for each entry in the jumptable
                        //     auipc   pc, 0
                        //     jalr    zero, pc, block_target

                        // Extend the index to 64 bits.
                        //
                        // This prevents us branching on the top 32 bits of the index, which
                        // are undefined.
                        Inst::Extend {
                            rd: ext_index,
                            rn: index,
                            signed: false,
                            from_bits: 32,
                            to_bits: 64,
                        }
                        .emit(&[], sink, emit_info, state);

                        // Bounds check.
                        //
                        // Check if the index passed in is larger than the number of jumptable
                        // entries that we have. If it is, we fallthrough to a jump into the
                        // default block.
                        Inst::load_constant_u32(tmp2, targets.len() as u64, &mut |_| tmp2)
                            .iter()
                            .for_each(|i| i.emit(&[], sink, emit_info, state));
                        Inst::CondBr {
                            taken: BranchTarget::offset(Inst::INSTRUCTION_SIZE * 3),
                            not_taken: BranchTarget::zero(),
                            kind: IntegerCompare {
                                kind: IntCC::UnsignedLessThan,
                                rs1: ext_index.to_reg(),
                                rs2: tmp2.to_reg(),
                            },
                        }
                        .emit(&[], sink, emit_info, state);
                        sink.use_label_at_offset(
                            sink.cur_offset(),
                            default_target.as_label().unwrap(),
                            LabelUse::PCRel32,
                        );
                        Inst::construct_auipc_and_jalr(None, tmp2, 0)
                            .iter()
                            .for_each(|i| i.emit(&[], sink, emit_info, state));

                        // Compute the jump table offset.
                        // We need to emit a PC relative offset,

                        // Get the current PC.
                        Inst::Auipc {
                            rd: tmp1,
                            imm: Imm20::from_bits(0),
                        }
                        .emit(&[], sink, emit_info, state);

                        // Multiply the index by 8, since that is the size in
                        // bytes of each jump table entry
                        Inst::AluRRImm12 {
                            alu_op: AluOPRRI::Slli,
                            rd: tmp2,
                            rs: ext_index.to_reg(),
                            imm12: Imm12::from_bits(3),
                        }
                        .emit(&[], sink, emit_info, state);

                        // Calculate the base of the jump, PC + the offset from above.
                        Inst::AluRRR {
                            alu_op: AluOPRRR::Add,
                            rd: tmp1,
                            rs1: tmp1.to_reg(),
                            rs2: tmp2.to_reg(),
                        }
                        .emit(&[], sink, emit_info, state);

                        // Jump to the middle of the jump table.
                        // We add a 16 byte offset here, since we used 4 instructions
                        // since the AUIPC that was used to get the PC.
                        Inst::Jalr {
                            rd: writable_zero_reg(),
                            base: tmp1.to_reg(),
                            offset: Imm12::from_bits((4 * Inst::INSTRUCTION_SIZE) as i16),
                        }
                        .emit(&[], sink, emit_info, state);

                        // Emit the jump table.
                        //
                        // Each entry is a aupc + jalr to the target block. We also start with a island
                        // if necessary.

                        // Each entry in the jump table is 2 instructions, so 8 bytes. Check if
                        // we need to emit a jump table here to support that jump.
                        let distance = (targets.len() * 2 * Inst::INSTRUCTION_SIZE as usize) as u32;
                        if sink.island_needed(distance) {
                            sink.emit_island(&mut state.ctrl_plane);
                        }

                        // Emit the jumps back to back
                        for target in targets.iter() {
                            sink.use_label_at_offset(
                                sink.cur_offset(),
                                target.as_label().unwrap(),
                                LabelUse::PCRel32,
                            );

                            Inst::construct_auipc_and_jalr(None, tmp2, 0)
                                .iter()
                                .for_each(|i| i.emit(&[], sink, emit_info, state));
                        }

                        // We've just emitted an island that is safe up to *here*.
                        // Mark it as such so that we don't needlessly emit additional islands.
                        start_off = sink.cur_offset(); */
            }

            &Inst::VirtualSPOffsetAdj { amount } => {
                println!("virtual_sp_offset_adj {amount}");
                // crate::trace!(
                //     "virtual sp offset adjusted by {} -> {}",
                //     amount,
                //     state.virtual_sp_offset + amount
                //     );
                // state.virtual_sp_offset += amount;
            }
            &Inst::Auipc { rd, imm } => {
                todo!() /* let rd = allocs.next_writable(rd);
                        let x = enc_auipc(rd, imm);
                        sink.put4(x); */
            }

            &Inst::LoadAddr { rd, mem } => {
                todo!() /* let mem = mem.with_allocs(&mut allocs);
                        let rd = allocs.next_writable(rd);

                        let base = mem.get_base_register();
                        let offset = mem.get_offset_with_state(state);
                        let offset_imm12 = Imm12::maybe_from_u64(offset as u64);

                        match (mem, base, offset_imm12) {
                            (_, Some(rs), Some(imm12)) => {
                                Inst::AluRRImm12 {
                                    alu_op: AluOPRRI::Addi,
                                    rd,
                                    rs,
                                    imm12,
                                }
                                .emit(&[], sink, emit_info, state);
                            }
                            (_, Some(rs), None) => {
                                LoadConstant::U64(offset as u64)
                                    .load_constant_and_add(rd, rs)
                                    .into_iter()
                                    .for_each(|inst| inst.emit(&[], sink, emit_info, state));
                            }
                            (AMode::Const(addr), None, _) => {
                                // Get an address label for the constant and recurse.
                                let label = sink.get_label_for_constant(addr);
                                Inst::LoadAddr {
                                    rd,
                                    mem: AMode::Label(label),
                                }
                                .emit(&[], sink, emit_info, state);
                            }
                            (AMode::Label(label), None, _) => {
                                // Get the current PC.
                                sink.use_label_at_offset(sink.cur_offset(), label, LabelUse::PCRelHi20);
                                let inst = Inst::Auipc {
                                    rd,
                                    imm: Imm20::from_bits(0),
                                };
                                inst.emit(&[], sink, emit_info, state);

                                // Emit an add to the address with a relocation.
                                // This later gets patched up with the correct offset.
                                sink.use_label_at_offset(sink.cur_offset(), label, LabelUse::PCRelLo12I);
                                Inst::AluRRImm12 {
                                    alu_op: AluOPRRI::Addi,
                                    rd,
                                    rs: rd.to_reg(),
                                    imm12: Imm12::zero(),
                                }
                                .emit(&[], sink, emit_info, state);
                            }
                            (amode, _, _) => {
                                unimplemented!("LoadAddr: {:?}", amode);
                            }
                        } */
            }

            &Inst::Select {
                ref dst,
                condition,
                ref x,
                ref y,
                ty: _ty,
            } => {
                todo!() /* let condition = allocs.next(condition);
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
                                rs1: condition,
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
                        sink.bind_label(label_false, &mut state.ctrl_plane);
                        // select second value1
                        insts.extend(gen_moves(&dst[..], y.regs()));
                        insts
                            .into_iter()
                            .for_each(|i| i.emit(&[], sink, emit_info, state));
                        sink.bind_label(label_jump_over, &mut state.ctrl_plane); */
            }
            &Inst::Jalr { rd, base, offset } => {
                todo!() /* let rd = allocs.next_writable(rd);
                        let x = enc_jalr(rd, base, offset);
                        sink.put4(x); */
            }
            &Inst::ECall => {
                todo!() // sink.put4(0x00000073);
            }
            &Inst::EBreak => {
                todo!() // sink.put4(0x00100073);
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

                let a = a
                    .only_reg()
                    .expect("Only support 1 register in comparison now");
                let b = b
                    .only_reg()
                    .expect("Only support 1 register in comparison now");
                debug_assert_eq!(a, a0());
                debug_assert_eq!(b, b0());

                let opcode = match cc {
                    IntCC::Equal => "EQ",
                    IntCC::NotEqual => "NEQ",
                    IntCC::SignedLessThan => "SLT",
                    IntCC::SignedGreaterThanOrEqual => todo!(),
                    IntCC::SignedGreaterThan => todo!(),
                    IntCC::SignedLessThanOrEqual => todo!(),
                    IntCC::UnsignedLessThan => "LT",
                    IntCC::UnsignedGreaterThanOrEqual => todo!(),
                    IntCC::UnsignedGreaterThan => todo!(),
                    IntCC::UnsignedLessThanOrEqual => todo!(),
                };

                put_string(&format!("$ => {} :{opcode}\n", reg_name(rd.to_reg())), sink);

                /*
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

                sink.bind_label(label_true, &mut state.ctrl_plane);
                Inst::load_imm12(rd, Imm12::TRUE).emit(&[], sink, emit_info, state);
                Inst::Jal {
                    dest: BranchTarget::offset(Inst::INSTRUCTION_SIZE * 2),
                }
                .emit(&[], sink, emit_info, state);
                sink.bind_label(label_false, &mut state.ctrl_plane);
                Inst::load_imm12(rd, Imm12::FALSE).emit(&[], sink, emit_info, state); */
            }
            &Inst::IntSelect {
                op,
                ref dst,
                ref x,
                ref y,
                ty,
            } => {
                todo!() /* let x = alloc_value_regs(x, &mut allocs);
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
                            let mut insts = SmallInstVec::new();
                            insts.push(Inst::Mov {
                                rd: dst[0],
                                rm: val.regs()[0],
                                ty: I64,
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
                        sink.bind_label(label_true, &mut state.ctrl_plane);
                        gen_move(&dst, &x, sink, state);
                        Inst::gen_jump(label_done).emit(&[], sink, emit_info, state);
                        // here is false use y
                        sink.bind_label(label_false, &mut state.ctrl_plane);
                        gen_move(&dst, &y, sink, state);
                        sink.bind_label(label_done, &mut state.ctrl_plane); */
            }

            &Inst::SelectReg {
                condition,
                rd,
                rs1,
                rs2,
            } => {
                todo!() /* let mut condition = condition.clone();
                        condition.rs1 = allocs.next(condition.rs1);
                        condition.rs2 = allocs.next(condition.rs2);
                        let rs1 = allocs.next(rs1);
                        let rs2 = allocs.next(rs2);
                        let rd = allocs.next_writable(rd);
                        let label_true = sink.get_label();
                        let label_jump_over = sink.get_label();
                        let ty = Inst::canonical_type_for_rc(rs1.class());

                        sink.use_label_at_offset(sink.cur_offset(), label_true, LabelUse::B12);
                        let x = condition.emit();
                        sink.put4(x);
                        // here is false , use rs2
                        Inst::gen_move(rd, rs2, ty).emit(&[], sink, emit_info, state);
                        // and jump over
                        Inst::Jal {
                            dest: BranchTarget::Label(label_jump_over),
                        }
                        .emit(&[], sink, emit_info, state);
                        // here condition is true , use rs1
                        sink.bind_label(label_true, &mut state.ctrl_plane);
                        Inst::gen_move(rd, rs1, ty).emit(&[], sink, emit_info, state);
                        sink.bind_label(label_jump_over, &mut state.ctrl_plane); */
            }
            &Inst::LoadExtName {
                rd,
                ref name,
                offset,
            } => {
                // dbg!(rd, name, offset);
                // let rd = allocs.next_writable(rd);
                // put_string(&format!("CALL {name:?} => {}\n", reg_name(rd.to_reg())), sink);

                /*
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
                    from: AMode::RegOffset(
                        rd.to_reg(),
                        12, // auipc load and jal.
                        I64,
                    ),
                }
                .emit(&[], sink, emit_info, state);
                // jump over.
                Inst::Jal {
                    // jal and abs8 size for 12.
                    dest: BranchTarget::offset(12),
                }
                .emit(&[], sink, emit_info, state);

                sink.add_reloc(Reloc::Abs8, name.as_ref(), offset);
                sink.put8(0); */
            }
            &Inst::TrapIfC {
                rs1,
                rs2,
                cc,
                trap_code,
            } => {
                todo!() /* let rs1 = allocs.next(rs1);
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
                        sink.bind_label(label_trap, &mut state.ctrl_plane);
                        Inst::Udf { trap_code }.emit(&[], sink, emit_info, state);
                        sink.bind_label(label_jump_over, &mut state.ctrl_plane); */
            }
            &Inst::TrapIf { test, trap_code } => {
                todo!() /* let test = allocs.next(test);
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
                        sink.bind_label(label_trap, &mut state.ctrl_plane);
                        Inst::Udf {
                            trap_code: trap_code,
                        }
                        .emit(&[], sink, emit_info, state);
                        sink.bind_label(label_jump_over, &mut state.ctrl_plane); */
            }
            &Inst::Udf { trap_code } => {
                todo!() /* sink.add_trap(trap_code);
                        if let Some(s) = state.take_stack_map() {
                            sink.add_stack_map(StackMapExtent::UpcomingBytes(4), s);
                        }
                        sink.put_data(Inst::TRAP_OPCODE); */
            }

            &Inst::Popcnt {
                sum,
                tmp,
                step,
                rs,
                ty,
            } => {
                todo!() /* let rs = allocs.next(rs);
                        let tmp = allocs.next_writable(tmp);
                        let step = allocs.next_writable(step);
                        let sum = allocs.next_writable(sum);
                        // load 0 to sum , init.
                        Inst::gen_move(sum, zero_reg(), I64).emit(&[], sink, emit_info, state);
                        // load
                        Inst::load_imm12(step, Imm12::from_bits(ty.bits() as i16)).emit(
                            &[],
                            sink,
                            emit_info,
                            state,
                        );
                        //
                        Inst::load_imm12(tmp, Imm12::from_bits(1)).emit(&[], sink, emit_info, state);
                        Inst::AluRRImm12 {
                            alu_op: AluOPRRI::Slli,
                            rd: tmp,
                            rs: tmp.to_reg(),
                            imm12: Imm12::from_bits((ty.bits() - 1) as i16),
                        }
                        .emit(&[], sink, emit_info, state);
                        let label_done = sink.get_label();
                        let label_loop = sink.get_label();
                        sink.bind_label(label_loop, &mut state.ctrl_plane);
                        Inst::CondBr {
                            taken: BranchTarget::Label(label_done),
                            not_taken: BranchTarget::zero(),
                            kind: IntegerCompare {
                                kind: IntCC::SignedLessThanOrEqual,
                                rs1: step.to_reg(),
                                rs2: zero_reg(),
                            },
                        }
                        .emit(&[], sink, emit_info, state);
                        // test and add sum.
                        {
                            Inst::AluRRR {
                                alu_op: AluOPRRR::And,
                                rd: writable_spilltmp_reg2(),
                                rs1: tmp.to_reg(),
                                rs2: rs,
                            }
                            .emit(&[], sink, emit_info, state);
                            let label_over = sink.get_label();
                            Inst::CondBr {
                                taken: BranchTarget::Label(label_over),
                                not_taken: BranchTarget::zero(),
                                kind: IntegerCompare {
                                    kind: IntCC::Equal,
                                    rs1: zero_reg(),
                                    rs2: spilltmp_reg2(),
                                },
                            }
                            .emit(&[], sink, emit_info, state);
                            Inst::AluRRImm12 {
                                alu_op: AluOPRRI::Addi,
                                rd: sum,
                                rs: sum.to_reg(),
                                imm12: Imm12::from_bits(1),
                            }
                            .emit(&[], sink, emit_info, state);
                            sink.bind_label(label_over, &mut state.ctrl_plane);
                        }
                        // set step and tmp.
                        {
                            Inst::AluRRImm12 {
                                alu_op: AluOPRRI::Addi,
                                rd: step,
                                rs: step.to_reg(),
                                imm12: Imm12::from_bits(-1),
                            }
                            .emit(&[], sink, emit_info, state);
                            Inst::AluRRImm12 {
                                alu_op: AluOPRRI::Srli,
                                rd: tmp,
                                rs: tmp.to_reg(),
                                imm12: Imm12::from_bits(1),
                            }
                            .emit(&[], sink, emit_info, state);
                            Inst::Jal {
                                dest: BranchTarget::Label(label_loop),
                            }
                            .emit(&[], sink, emit_info, state);
                        }
                        sink.bind_label(label_done, &mut state.ctrl_plane); */
            }
            &Inst::Rev8 { rs, rd, tmp, step } => {
                todo!() /* let rs = allocs.next(rs);
                        let tmp = allocs.next_writable(tmp);
                        let step = allocs.next_writable(step);
                        let rd = allocs.next_writable(rd);
                        // init.
                        Inst::gen_move(rd, zero_reg(), I64).emit(&[], sink, emit_info, state);
                        Inst::gen_move(tmp, rs, I64).emit(&[], sink, emit_info, state);
                        // load 56 to step.
                        Inst::load_imm12(step, Imm12::from_bits(56)).emit(&[], sink, emit_info, state);
                        let label_done = sink.get_label();
                        let label_loop = sink.get_label();
                        sink.bind_label(label_loop, &mut state.ctrl_plane);
                        Inst::CondBr {
                            taken: BranchTarget::Label(label_done),
                            not_taken: BranchTarget::zero(),
                            kind: IntegerCompare {
                                kind: IntCC::SignedLessThan,
                                rs1: step.to_reg(),
                                rs2: zero_reg(),
                            },
                        }
                        .emit(&[], sink, emit_info, state);
                        Inst::AluRRImm12 {
                            alu_op: AluOPRRI::Andi,
                            rd: writable_spilltmp_reg(),
                            rs: tmp.to_reg(),
                            imm12: Imm12::from_bits(255),
                        }
                        .emit(&[], sink, emit_info, state);
                        Inst::AluRRR {
                            alu_op: AluOPRRR::Sll,
                            rd: writable_spilltmp_reg(),
                            rs1: spilltmp_reg(),
                            rs2: step.to_reg(),
                        }
                        .emit(&[], sink, emit_info, state);

                        Inst::AluRRR {
                            alu_op: AluOPRRR::Or,
                            rd: rd,
                            rs1: rd.to_reg(),
                            rs2: spilltmp_reg(),
                        }
                        .emit(&[], sink, emit_info, state);
                        {
                            // reset step
                            Inst::AluRRImm12 {
                                alu_op: AluOPRRI::Addi,
                                rd: step,
                                rs: step.to_reg(),
                                imm12: Imm12::from_bits(-8),
                            }
                            .emit(&[], sink, emit_info, state);
                            //reset tmp.
                            Inst::AluRRImm12 {
                                alu_op: AluOPRRI::Srli,
                                rd: tmp,
                                rs: tmp.to_reg(),
                                imm12: Imm12::from_bits(8),
                            }
                            .emit(&[], sink, emit_info, state);
                            // loop.
                            Inst::Jal {
                                dest: BranchTarget::Label(label_loop),
                            }
                        }
                        .emit(&[], sink, emit_info, state);
                        sink.bind_label(label_done, &mut state.ctrl_plane); */
            }
            &Inst::Cltz {
                sum,
                tmp,
                step,
                rs,
                leading,
                ty,
            } => {
                todo!() /* let rs = allocs.next(rs);
                        let tmp = allocs.next_writable(tmp);
                        let step = allocs.next_writable(step);
                        let sum = allocs.next_writable(sum);
                        // load 0 to sum , init.
                        Inst::gen_move(sum, zero_reg(), I64).emit(&[], sink, emit_info, state);
                        // load
                        Inst::load_imm12(step, Imm12::from_bits(ty.bits() as i16)).emit(
                            &[],
                            sink,
                            emit_info,
                            state,
                        );
                        //
                        Inst::load_imm12(tmp, Imm12::from_bits(1)).emit(&[], sink, emit_info, state);
                        if leading {
                            Inst::AluRRImm12 {
                                alu_op: AluOPRRI::Slli,
                                rd: tmp,
                                rs: tmp.to_reg(),
                                imm12: Imm12::from_bits((ty.bits() - 1) as i16),
                            }
                            .emit(&[], sink, emit_info, state);
                        }
                        let label_done = sink.get_label();
                        let label_loop = sink.get_label();
                        sink.bind_label(label_loop, &mut state.ctrl_plane);
                        Inst::CondBr {
                            taken: BranchTarget::Label(label_done),
                            not_taken: BranchTarget::zero(),
                            kind: IntegerCompare {
                                kind: IntCC::SignedLessThanOrEqual,
                                rs1: step.to_reg(),
                                rs2: zero_reg(),
                            },
                        }
                        .emit(&[], sink, emit_info, state);
                        // test and add sum.
                        {
                            Inst::AluRRR {
                                alu_op: AluOPRRR::And,
                                rd: writable_spilltmp_reg2(),
                                rs1: tmp.to_reg(),
                                rs2: rs,
                            }
                            .emit(&[], sink, emit_info, state);
                            Inst::CondBr {
                                taken: BranchTarget::Label(label_done),
                                not_taken: BranchTarget::zero(),
                                kind: IntegerCompare {
                                    kind: IntCC::NotEqual,
                                    rs1: zero_reg(),
                                    rs2: spilltmp_reg2(),
                                },
                            }
                            .emit(&[], sink, emit_info, state);
                            Inst::AluRRImm12 {
                                alu_op: AluOPRRI::Addi,
                                rd: sum,
                                rs: sum.to_reg(),
                                imm12: Imm12::from_bits(1),
                            }
                            .emit(&[], sink, emit_info, state);
                        }
                        // set step and tmp.
                        {
                            Inst::AluRRImm12 {
                                alu_op: AluOPRRI::Addi,
                                rd: step,
                                rs: step.to_reg(),
                                imm12: Imm12::from_bits(-1),
                            }
                            .emit(&[], sink, emit_info, state);
                            Inst::AluRRImm12 {
                                alu_op: if leading {
                                    AluOPRRI::Srli
                                } else {
                                    AluOPRRI::Slli
                                },
                                rd: tmp,
                                rs: tmp.to_reg(),
                                imm12: Imm12::from_bits(1),
                            }
                            .emit(&[], sink, emit_info, state);
                            Inst::Jal {
                                dest: BranchTarget::Label(label_loop),
                            }
                            .emit(&[], sink, emit_info, state);
                        }
                        sink.bind_label(label_done, &mut state.ctrl_plane); */
            }
            &Inst::Brev8 {
                rs,
                ty,
                step,
                tmp,
                tmp2,
                rd,
            } => {
                todo!() /* let rs = allocs.next(rs);
                        let step = allocs.next_writable(step);
                        let tmp = allocs.next_writable(tmp);
                        let tmp2 = allocs.next_writable(tmp2);
                        let rd = allocs.next_writable(rd);
                        Inst::gen_move(rd, zero_reg(), I64).emit(&[], sink, emit_info, state);
                        Inst::load_imm12(step, Imm12::from_bits(ty.bits() as i16)).emit(
                            &[],
                            sink,
                            emit_info,
                            state,
                        );
                        //
                        Inst::load_imm12(tmp, Imm12::from_bits(1)).emit(&[], sink, emit_info, state);
                        Inst::AluRRImm12 {
                            alu_op: AluOPRRI::Slli,
                            rd: tmp,
                            rs: tmp.to_reg(),
                            imm12: Imm12::from_bits((ty.bits() - 1) as i16),
                        }
                        .emit(&[], sink, emit_info, state);
                        Inst::load_imm12(tmp2, Imm12::from_bits(1)).emit(&[], sink, emit_info, state);
                        Inst::AluRRImm12 {
                            alu_op: AluOPRRI::Slli,
                            rd: tmp2,
                            rs: tmp2.to_reg(),
                            imm12: Imm12::from_bits((ty.bits() - 8) as i16),
                        }
                        .emit(&[], sink, emit_info, state);

                        let label_done = sink.get_label();
                        let label_loop = sink.get_label();
                        sink.bind_label(label_loop, &mut state.ctrl_plane);
                        Inst::CondBr {
                            taken: BranchTarget::Label(label_done),
                            not_taken: BranchTarget::zero(),
                            kind: IntegerCompare {
                                kind: IntCC::SignedLessThanOrEqual,
                                rs1: step.to_reg(),
                                rs2: zero_reg(),
                            },
                        }
                        .emit(&[], sink, emit_info, state);
                        // test and set bit.
                        {
                            Inst::AluRRR {
                                alu_op: AluOPRRR::And,
                                rd: writable_spilltmp_reg2(),
                                rs1: tmp.to_reg(),
                                rs2: rs,
                            }
                            .emit(&[], sink, emit_info, state);
                            let label_over = sink.get_label();
                            Inst::CondBr {
                                taken: BranchTarget::Label(label_over),
                                not_taken: BranchTarget::zero(),
                                kind: IntegerCompare {
                                    kind: IntCC::Equal,
                                    rs1: zero_reg(),
                                    rs2: spilltmp_reg2(),
                                },
                            }
                            .emit(&[], sink, emit_info, state);
                            Inst::AluRRR {
                                alu_op: AluOPRRR::Or,
                                rd: rd,
                                rs1: rd.to_reg(),
                                rs2: tmp2.to_reg(),
                            }
                            .emit(&[], sink, emit_info, state);
                            sink.bind_label(label_over, &mut state.ctrl_plane);
                        }
                        // set step and tmp.
                        {
                            Inst::AluRRImm12 {
                                alu_op: AluOPRRI::Addi,
                                rd: step,
                                rs: step.to_reg(),
                                imm12: Imm12::from_bits(-1),
                            }
                            .emit(&[], sink, emit_info, state);
                            Inst::AluRRImm12 {
                                alu_op: AluOPRRI::Srli,
                                rd: tmp,
                                rs: tmp.to_reg(),
                                imm12: Imm12::from_bits(1),
                            }
                            .emit(&[], sink, emit_info, state);
                            {
                                // reset tmp2
                                // if (step %=8 == 0) then tmp2 = tmp2 >> 15
                                // if (step %=8 != 0) then tmp2 = tmp2 << 1
                                let label_over = sink.get_label();
                                let label_sll_1 = sink.get_label();
                                Inst::load_imm12(writable_spilltmp_reg2(), Imm12::from_bits(8)).emit(
                                    &[],
                                    sink,
                                    emit_info,
                                    state,
                                );
                                Inst::AluRRR {
                                    alu_op: AluOPRRR::Rem,
                                    rd: writable_spilltmp_reg2(),
                                    rs1: step.to_reg(),
                                    rs2: spilltmp_reg2(),
                                }
                                .emit(&[], sink, emit_info, state);
                                Inst::CondBr {
                                    taken: BranchTarget::Label(label_sll_1),
                                    not_taken: BranchTarget::zero(),
                                    kind: IntegerCompare {
                                        kind: IntCC::NotEqual,
                                        rs1: spilltmp_reg2(),
                                        rs2: zero_reg(),
                                    },
                                }
                                .emit(&[], sink, emit_info, state);
                                Inst::AluRRImm12 {
                                    alu_op: AluOPRRI::Srli,
                                    rd: tmp2,
                                    rs: tmp2.to_reg(),
                                    imm12: Imm12::from_bits(15),
                                }
                                .emit(&[], sink, emit_info, state);
                                Inst::Jal {
                                    dest: BranchTarget::Label(label_over),
                                }
                                .emit(&[], sink, emit_info, state);
                                sink.bind_label(label_sll_1, &mut state.ctrl_plane);
                                Inst::AluRRImm12 {
                                    alu_op: AluOPRRI::Slli,
                                    rd: tmp2,
                                    rs: tmp2.to_reg(),
                                    imm12: Imm12::from_bits(1),
                                }
                                .emit(&[], sink, emit_info, state);
                                sink.bind_label(label_over, &mut state.ctrl_plane);
                            }
                            Inst::Jal {
                                dest: BranchTarget::Label(label_loop),
                            }
                            .emit(&[], sink, emit_info, state);
                        }
                        sink.bind_label(label_done, &mut state.ctrl_plane); */
            }
            &Inst::StackProbeLoop {
                guard_size,
                probe_count,
                tmp: guard_size_tmp,
            } => {
                todo!() /* let step = writable_spilltmp_reg();
                        Inst::load_constant_u64(
                            step,
                            (guard_size as u64) * (probe_count as u64),
                            &mut |_| step,
                        )
                        .iter()
                        .for_each(|i| i.emit(&[], sink, emit_info, state));
                        Inst::load_constant_u64(guard_size_tmp, guard_size as u64, &mut |_| guard_size_tmp)
                            .iter()
                            .for_each(|i| i.emit(&[], sink, emit_info, state));

                        let loop_start = sink.get_label();
                        let label_done = sink.get_label();
                        sink.bind_label(loop_start, &mut state.ctrl_plane);
                        Inst::CondBr {
                            taken: BranchTarget::Label(label_done),
                            not_taken: BranchTarget::zero(),
                            kind: IntegerCompare {
                                kind: IntCC::UnsignedLessThanOrEqual,
                                rs1: step.to_reg(),
                                rs2: guard_size_tmp.to_reg(),
                            },
                        }
                        .emit(&[], sink, emit_info, state);
                        // compute address.
                        Inst::AluRRR {
                            alu_op: AluOPRRR::Sub,
                            rd: writable_spilltmp_reg2(),
                            rs1: stack_reg(),
                            rs2: step.to_reg(),
                        }
                        .emit(&[], sink, emit_info, state);
                        Inst::Store {
                            to: AMode::RegOffset(spilltmp_reg2(), 0, I8),
                            op: StoreOP::Sb,
                            flags: MemFlags::new(),
                            src: zero_reg(),
                        }
                        .emit(&[], sink, emit_info, state);
                        // reset step.
                        Inst::AluRRR {
                            alu_op: AluOPRRR::Sub,
                            rd: step,
                            rs1: step.to_reg(),
                            rs2: guard_size_tmp.to_reg(),
                        }
                        .emit(&[], sink, emit_info, state);
                        Inst::Jal {
                            dest: BranchTarget::Label(loop_start),
                        }
                        .emit(&[], sink, emit_info, state);
                        sink.bind_label(label_done, &mut state.ctrl_plane); */
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
    match orgin.regs().len() {
        1 => ValueRegs::one(alloc.next(orgin.regs()[0])),
        2 => ValueRegs::two(alloc.next(orgin.regs()[0]), alloc.next(orgin.regs()[1])),
        _ => unreachable!(),
    }
}

#[allow(unused)]
fn emit_return_call_common_sequence(
    allocs: &mut AllocationConsumer<'_>,
    sink: &mut MachBuffer<Inst>,
    emit_info: &EmitInfo,
    state: &mut EmitState,
    new_stack_arg_size: u32,
    old_stack_arg_size: u32,
    uses: &CallArgList,
) {
    todo!()
    /* for u in uses {
        let _ = allocs.next(u.vreg);
    }

    // We are emitting a dynamic number of instructions and might need an
    // island. We emit four instructions regardless of how many stack arguments
    // we have, up to two instructions for the actual call, and then two
    // instructions per word of stack argument space.
    let new_stack_words = new_stack_arg_size / 8;
    let insts = 4 + 2 + 2 * new_stack_words;
    let space_needed = insts * u32::try_from(Inst::INSTRUCTION_SIZE).unwrap();
    if sink.island_needed(space_needed) {
        let jump_around_label = sink.get_label();
        Inst::Jal {
            dest: BranchTarget::Label(jump_around_label),
        }
        .emit(&[], sink, emit_info, state);
        sink.emit_island(&mut state.ctrl_plane);
        sink.bind_label(jump_around_label, &mut state.ctrl_plane);
    }

    // Copy the new frame on top of our current frame.
    //
    // The current stack layout is the following:
    //
    //            | ...                 |
    //            +---------------------+
    //            | ...                 |
    //            | stack arguments     |
    //            | ...                 |
    //    current | return address      |
    //    frame   | old FP              | <-- FP
    //            | ...                 |
    //            | old stack slots     |
    //            | ...                 |
    //            +---------------------+
    //            | ...                 |
    //    new     | new stack arguments |
    //    frame   | ...                 | <-- SP
    //            +---------------------+
    //
    // We need to restore the old FP, restore the return address from the stack
    // to the link register, copy the new stack arguments over the old stack
    // arguments, adjust SP to point to the new stack arguments, and then jump
    // to the callee (which will push the old FP and RA again). Note that the
    // actual jump happens outside this helper function.

    assert_eq!(
        new_stack_arg_size % 8,
        0,
        "size of new stack arguments must be 8-byte aligned"
    );

    // The delta from our frame pointer to the (eventual) stack pointer value
    // when we jump to the tail callee. This is the difference in size of stack
    // arguments as well as accounting for the two words we pushed onto the
    // stack upon entry to this function (the return address and old frame
    // pointer).
    let fp_to_callee_sp = i64::from(old_stack_arg_size) - i64::from(new_stack_arg_size) + 16;

    let tmp1 = regs::writable_spilltmp_reg();
    let tmp2 = regs::writable_spilltmp_reg2();

    // Restore the return address to the link register, and load the old FP into
    // a temporary register.
    //
    // We can't put the old FP into the FP register until after we copy the
    // stack arguments into place, since that uses address modes that are
    // relative to our current FP.
    //
    // Note that the FP is saved in the function prologue for all non-leaf
    // functions, even when `preserve_frame_pointers=false`. Note also that
    // `return_call` instructions make it so that a function is considered
    // non-leaf. Therefore we always have an FP to restore here.

    Inst::gen_load(
        writable_link_reg(),
        AMode::FPOffset(8, I64),
        I64,
        MemFlags::trusted(),
    )
    .emit(&[], sink, emit_info, state);
    Inst::gen_load(tmp1, AMode::FPOffset(0, I64), I64, MemFlags::trusted()).emit(
        &[],
        sink,
        emit_info,
        state,
    );

    // Copy the new stack arguments over the old stack arguments.
    for i in (0..new_stack_words).rev() {
        // Load the `i`th new stack argument word from the temporary stack
        // space.
        Inst::gen_load(
            tmp2,
            AMode::SPOffset(i64::from(i * 8), types::I64),
            types::I64,
            ir::MemFlags::trusted(),
        )
        .emit(&[], sink, emit_info, state);

        // Store it to its final destination on the stack, overwriting our
        // current frame.
        Inst::gen_store(
            AMode::FPOffset(fp_to_callee_sp + i64::from(i * 8), types::I64),
            tmp2.to_reg(),
            types::I64,
            ir::MemFlags::trusted(),
        )
        .emit(&[], sink, emit_info, state);
    }

    // Initialize the SP for the tail callee, deallocating the temporary stack
    // argument space and our current frame at the same time.
    Inst::AluRRImm12 {
        alu_op: AluOPRRI::Addi,
        rd: regs::writable_stack_reg(),
        rs: regs::fp_reg(),
        imm12: Imm12::maybe_from_u64(fp_to_callee_sp as u64).unwrap(),
    }
    .emit(&[], sink, emit_info, state);

    // Move the old FP value from the temporary into the FP register.
    Inst::Mov {
        ty: types::I64,
        rd: regs::writable_fp_reg(),
        rm: tmp1.to_reg(),
    }
    .emit(&[], sink, emit_info, state);

    state.virtual_sp_offset -= i64::from(new_stack_arg_size);
    trace!(
        "return_call[_ind] adjusts virtual sp offset by {} -> {}",
        new_stack_arg_size,
        state.virtual_sp_offset
    ); */
}
