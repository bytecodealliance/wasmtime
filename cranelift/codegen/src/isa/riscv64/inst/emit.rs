//! AArch64 ISA: binary code emission.

use core::panic;

use crate::binemit::StackMap;
use crate::isa::riscv64::inst::*;

use crate::isa::riscv64::inst::{zero_reg, AluOPRRR};
use crate::machinst::{AllocationConsumer, Reg, Writable};
use alloc::borrow::ToOwned;
use regalloc2::Allocation;

use alloc::vec;

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

pub(crate) enum BitsShifter {
    Reg(Reg),
    Imm(u8),
}

impl BitsShifter {
    pub(crate) fn new_r(r: Reg) -> Self {
        Self::Reg(r)
    }

    pub(crate) fn new_i(r: u8) -> Self {
        Self::Imm(r)
    }

    /*
        get rid of all lowest bit value
    */
    pub(crate) fn shift_out_right(self, rd: Writable<Reg>, rs: Reg) -> SmallInstVec<Inst> {
        let mut insts = SmallInstVec::new();
        match self {
            Self::Reg(r) => {
                insts.push(Inst::AluRRR {
                    alu_op: AluOPRRR::Srl,
                    rd,
                    rs1: rs,
                    rs2: r,
                });
                insts.push(Inst::AluRRR {
                    alu_op: AluOPRRR::Sll,
                    rd,
                    rs1: rd.to_reg(),
                    rs2: r,
                });
            }
            Self::Imm(imm) => {
                insts.push(Inst::AluRRImm12 {
                    alu_op: AluOPRRI::Srli,
                    rd,
                    rs: rs,
                    imm12: Imm12::from_bits(imm as i16),
                });
                insts.push(Inst::AluRRImm12 {
                    alu_op: AluOPRRI::Slli,
                    rd,
                    rs: rd.to_reg(),
                    imm12: Imm12::from_bits(imm as i16),
                });
            }
        }
        insts
    }

    pub(crate) fn shift_right(self, rd: Writable<Reg>, rs: Reg) -> SmallInstVec<Inst> {
        let mut insts = SmallInstVec::new();
        match self {
            Self::Reg(r) => {
                insts.push(Inst::AluRRR {
                    alu_op: AluOPRRR::Srl,
                    rd,
                    rs1: rs,
                    rs2: r,
                });
            }
            Self::Imm(imm) => {
                insts.push(Inst::AluRRImm12 {
                    alu_op: AluOPRRI::Srli,
                    rd,
                    rs: rs,
                    imm12: Imm12::from_bits(imm as i16),
                });
            }
        }
        insts
    }

    pub(crate) fn shift_out_left(self, rd: Writable<Reg>, rs: Reg) -> SmallInstVec<Inst> {
        let mut insts = SmallInstVec::new();
        match self {
            Self::Reg(amount) => {
                insts.push(Inst::AluRRR {
                    alu_op: AluOPRRR::Sll,
                    rd,
                    rs1: rs,
                    rs2: amount,
                });
                insts.push(Inst::AluRRR {
                    alu_op: AluOPRRR::Srl,
                    rd,
                    rs1: rd.to_reg(),
                    rs2: amount,
                });
            }
            Self::Imm(imm) => {
                insts.push(Inst::AluRRImm12 {
                    alu_op: AluOPRRI::Slli,
                    rd,
                    rs: rs,
                    imm12: Imm12::from_bits(imm as i16),
                });
                insts.push(Inst::AluRRImm12 {
                    alu_op: AluOPRRI::Srli,
                    rd,
                    rs: rd.to_reg(),
                    imm12: Imm12::from_bits(imm as i16),
                });
            }
        }
        insts
    }
    pub(crate) fn shift_left(self, rd: Writable<Reg>, rs: Reg) -> SmallInstVec<Inst> {
        let mut insts = SmallInstVec::new();
        match self {
            Self::Reg(amount) => {
                insts.push(Inst::AluRRR {
                    alu_op: AluOPRRR::Sll,
                    rd,
                    rs1: rs,
                    rs2: amount,
                });
            }
            Self::Imm(imm) => {
                insts.push(Inst::AluRRImm12 {
                    alu_op: AluOPRRI::Slli,
                    rd,
                    rs: rs,
                    imm12: Imm12::from_bits(imm as i16),
                });
            }
        }
        insts
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
        construct a mask
            if amount = 5 and left_shift = Some(8)
            value in rd will be  0b...001_1111_0000_0000
    */
    pub(crate) fn construct_low_mask(
        rd: Writable<Reg>,
        tmp: Writable<Reg>,
        amount: Reg,
        left_shift: Option<u8>,
    ) -> SmallInstVec<Inst> {
        let mut insts = SmallInstVec::new();
        {
            // make tmp = 64 - rs
            insts.push(Inst::load_constant_imm12(tmp, Imm12::from_bits(64)));
            insts.push(Inst::AluRRR {
                alu_op: AluOPRRR::Sub,
                rd: tmp,
                rs1: tmp.to_reg(),
                rs2: amount,
            });
        }
        insts.push(Inst::load_constant_imm12(rd, Imm12::from_bits(-1)));
        insts.push(Inst::AluRRR {
            alu_op: AluOPRRR::Sll,
            rd: rd,
            rs1: rd.to_reg(),
            rs2: tmp.to_reg(),
        });
        insts.push(Inst::AluRRR {
            alu_op: AluOPRRR::Srl,
            rd: rd,
            rs1: rd.to_reg(),
            rs2: tmp.to_reg(),
        });
        left_shift.map(|x| {
            insts.push(Inst::AluRRImm12 {
                alu_op: AluOPRRI::Slli,
                rd,
                rs: rd.to_reg(),
                imm12: Imm12::from_bits(x as i16),
            });
        });
        insts
    }

    // /*
    //     if amount 5 then
    //         value in rd will be 0b1111100.000...
    // */
    // pub(crate) fn construct_high_mask<F: FnMut(Type) -> Writable<Reg>>(
    //     rd: Writable<Reg>,
    //     amount: Reg,
    //     mut f: F,
    // ) -> SmallInstVec<Inst> {
    //     let mut insts = SmallInstVec::new();
    //     let shift = f(I64);
    //     insts.push(Inst::load_constant_imm12(shift, Imm12::from_bits(64)));
    //     insts.push(Inst::AluRRR {
    //         alu_op: AluOPRRR::Sub,
    //         rd: shift,
    //         rs1: shift.to_reg(),
    //         rs2: amount,
    //     });
    //     let tmp = f(I64);
    //     insts.push(Inst::load_constant_imm12(shift, Imm12::from_bits(-1)));
    //     insts.push(Inst::AluRRR {
    //         alu_op: AluOPRRR::Sll,
    //         rd,
    //         rs1: tmp.to_reg(),
    //         rs2: shift.to_reg(),
    //     });
    //     insts
    // }

    /*
        inverset all bit
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
        assert!(is_int_and_type_signed(ty));
        assert!(ty.bits() != 64);
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
        notice always patch the taken path.
        this will make jump that jump over all insts.
    */
    pub(crate) fn patch_taken_list(insts: &mut SmallInstVec<Inst>, patches: &'_ Vec<usize>) {
        for index in patches {
            let index = *index;
            assert!(insts.len() > index);
            let real_off = (insts.len() - index) as i32 * Inst::instruction_size();
            assert!(real_off > 4);
            match &mut insts[index] {
                &mut Inst::CondBr { ref mut taken, .. } => match taken {
                    &mut BranchTarget::ResolvedOffset(_) => {
                        *taken = BranchTarget::ResolvedOffset(real_off)
                    }
                    _ => unreachable!(),
                },
                &mut Inst::Jal { ref mut dest, .. } => match dest {
                    &mut BranchTarget::ResolvedOffset(_) => {
                        *dest = BranchTarget::ResolvedOffset(real_off)
                    }
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            }
        }
    }
    /*
        rd == 1 is unordered
        rd == 0 is ordered
    */
    pub(crate) fn generate_float_unordered(
        rd: Writable<Reg>,
        ty: Type,
        left: Reg,
        right: Reg,
    ) -> SmallInstVec<Inst> {
        let mut insts = SmallVec::new();
        let mut patch_true = vec![];
        let class_op = if ty == F32 {
            AluOPRR::FclassS
        } else {
            AluOPRR::FclassD
        };
        // left
        insts.push(Inst::AluRR {
            alu_op: class_op,
            rd,
            rs: left,
        });
        insts.push(Inst::AluRRImm12 {
            alu_op: AluOPRRI::Andi,
            rd,
            rs: rd.to_reg(),
            imm12: Imm12::from_bits(FClassResult::is_nan_bits() as i16),
        });
        patch_true.push(insts.len());
        insts.push(Inst::CondBr {
            taken: BranchTarget::zero(),
            not_taken: BranchTarget::zero(),
            kind: IntegerCompare {
                kind: IntCC::NotEqual,
                rs1: rd.to_reg(),
                rs2: zero_reg(),
            },
        });
        //right
        let tmp = writable_spilltmp_reg();
        insts.push(Inst::AluRR {
            alu_op: class_op,
            rd: tmp,
            rs: right,
        });
        insts.push(Inst::AluRRImm12 {
            alu_op: AluOPRRI::Andi,
            rd: tmp,
            rs: tmp.to_reg(),
            imm12: Imm12::from_bits(FClassResult::is_nan_bits() as i16),
        });
        patch_true.push(insts.len());
        insts.push(Inst::CondBr {
            taken: BranchTarget::zero(),
            not_taken: BranchTarget::zero(),
            kind: IntegerCompare {
                kind: IntCC::NotEqual,
                rs1: tmp.to_reg(),
                rs2: zero_reg(),
            },
        });
        // left and right is not nan
        // but there are maybe bother PosInfinite or NegInfinite
        insts.push(Inst::AluRRR {
            alu_op: AluOPRRR::And,
            rd: rd,
            rs1: rd.to_reg(),
            rs2: tmp.to_reg(),
        });
        insts.push(Inst::AluRRImm12 {
            alu_op: AluOPRRI::Andi,
            rd: rd,
            rs: rd.to_reg(),
            imm12: Imm12::from_bits(FClassResult::is_infinite_bits() as i16),
        });
        patch_true.push(insts.len());
        insts.push(Inst::CondBr {
            taken: BranchTarget::zero(),
            not_taken: BranchTarget::zero(),
            kind: IntegerCompare {
                kind: IntCC::NotEqual,
                rs1: tmp.to_reg(),
                rs2: zero_reg(),
            },
        });
        // here is false
        insts.push(Inst::load_constant_imm12(rd, Imm12::form_bool(false)));
        // jump set true
        insts.push(Inst::Jal {
            dest: BranchTarget::offset(Inst::instruction_size() * 2),
        });
        Inst::patch_taken_list(&mut insts, &patch_true);
        // here is true
        insts.push(Inst::load_constant_imm12(rd, Imm12::form_bool(true)));
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
        if exclusive is given, which we must not include the  exclusive register (it's aleady been allocted or something),so must skip it.
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

    /*
    riscv document says.

    We did not include special instruction set support for overflow checks on integer arithmetic
        operations in the base instruction set, as many overflow checks can be cheaply implemented using
        RISC-V branches. Overflow checking for unsigned addition requires only a single additional
        branch instruction after the addition: add t0, t1, t2; bltu t0, t1, overflow.
            */
    fn add_c_u(rd: Writable<Reg>, carry: Writable<Reg>, x: Reg, y: Reg) -> SmallInstVec<Inst> {
        let mut insts = SmallInstVec::new();
        insts.push(Inst::gen_move(carry, x, I64));
        insts.push(Inst::AluRRR {
            alu_op: AluOPRRR::Add,
            rd,
            rs1: x,
            rs2: y,
        });
        insts.push(Inst::AluRRR {
            alu_op: AluOPRRR::SltU,
            rd: carry,
            rs1: rd.to_reg(),
            rs2: carry.to_reg(),
        });
        /*
        gcc generate this .

        add	a4,a2,a0
        mv	a6,a4
        sltu	a6,a6,a2
            // a6 either be 1 or 0
            // nothing looks will change
        slli	a6,a6,32 ??????????
        srli	a6,a6,32 ??????????????

        add	a5,a3,a1
        add	a3,a6,a5
            */
        insts
    }

    fn sub_b_u(rd: Writable<Reg>, borrow: Writable<Reg>, x: Reg, y: Reg) -> SmallInstVec<Inst> {
        let mut insts = SmallInstVec::new();
        insts.push(Inst::gen_move(borrow, x, I64));
        insts.push(Inst::AluRRR {
            alu_op: AluOPRRR::Sub,
            rd,
            rs1: x,
            rs2: y,
        });
        insts.push(Inst::AluRRR {
            alu_op: AluOPRRR::SltU,
            rd: borrow,
            rs1: borrow.to_reg(),
            rs2: rd.to_reg(), // if rd > x then need borrow
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
        let start_off = sink.cur_offset();
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
            &Inst::AluRR { alu_op, rd, rs } => {
                let rs = allocs.next(rs);
                let rd = allocs.next_writable(rd);
                let x = alu_op.op_code()
                    | reg_to_gpr_num(rd.to_reg()) << 7
                    | alu_op.funct3() << 12
                    | reg_to_gpr_num(rs) << 15
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
                let rs1 = allocs.next(rs1);
                let rs2 = allocs.next(rs2);
                let rs3 = allocs.next(rs3);
                let rd = allocs.next_writable(rd);
                let x = alu_op.op_code()
                    | reg_to_gpr_num(rd.to_reg()) << 7
                    | alu_op.funct3() << 12
                    | reg_to_gpr_num(rs1) << 15
                    | reg_to_gpr_num(rs2) << 20
                    | alu_op.funct2() << 25
                    | reg_to_gpr_num(rs3) << 27;

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
                            flags: MemFlags::new(),
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
                            flags: MemFlags::new(),
                        });
                    })
                    .into_iter()
                    .for_each(|inst| inst.emit(&[], sink, emit_info, state));
                }
            }
            &Inst::EpiloguePlaceholder => {
                unimplemented!("what should I Do.");
            }
            &Inst::ReferenceValid { rd, op, x } => {
                let x = allocs.next(x);
                let rd = allocs.next_writable(rd);
                let mut insts = SmallInstVec::new();
                match op {
                    ReferenceValidOP::IsNull => {
                        insts.push(Inst::CondBr {
                            taken: BranchTarget::ResolvedOffset(Inst::instruction_size() * 2),
                            not_taken: BranchTarget::ResolvedOffset(0),
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

                    ReferenceValidOP::IsInvalid => {
                        /*
                            todo:: right now just check if it is null
                            null is a valid reference??????
                        */
                        insts.push(Inst::CondBr {
                            taken: BranchTarget::ResolvedOffset(Inst::instruction_size() * 2),
                            not_taken: BranchTarget::ResolvedOffset(0),
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
                to_bits,
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
                            let mut patch_signed_extend = vec![];
                            patch_signed_extend.push(insts.len());
                            insts.push(Inst::CondBr {
                                taken: BranchTarget::zero(),
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
                            let mut patch_zero_extend_jump_over = vec![];
                            patch_zero_extend_jump_over.push(insts.len());
                            // here are zero extend.
                            insts.push(Inst::Jal {
                                dest: BranchTarget::zero(),
                            });

                            Inst::patch_taken_list(&mut insts, &patch_signed_extend);
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
                            Inst::patch_taken_list(&mut insts, &patch_zero_extend_jump_over);
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
                unimplemented!("call not implamented.")
            }
            &Inst::CallInd { ref info } => {
                /*


                */
                if let Some(s) = state.take_stack_map() {
                    sink.add_stack_map(StackMapExtent::UpcomingBytes(4), s);
                }
                let reigsters = vec![writable_link_reg()];
                let mut insts = Inst::push_registers(&reigsters);
                let rn = allocs.next(info.rn);
                insts.push(Inst::Jalr {
                    rd: writable_link_reg(),
                    base: rn,
                    offset: Imm12::zero(),
                });
                insts.extend(Inst::pop_registers(&reigsters));
                insts
                    .into_iter()
                    .for_each(|i| i.emit(&[], sink, emit_info, state));
                let loc = state.cur_srcloc();

                if info.opcode.is_call() {
                    sink.add_call_site(loc, info.opcode);
                }
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
            &Inst::Jal { dest } => {
                let code: u32 = (0b1101111);
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
                            let code = kind.emit();
                            // jump is zero, this means when condition is met , no jump
                            // fallthrough to next instruction which is the long jump.
                            sink.put4(code);
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
                } else {
                    log::warn!("a redundancy move why???? {:?}->{:?}", rm, rd.to_reg());
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
                {
                    /*
                        if index not match all targets,
                        goto the default.
                    */
                    // index < 0
                    sink.use_label_at_offset(
                        sink.cur_offset(),
                        default_.as_label().unwrap(),
                        LabelUse::B12,
                    );
                    Inst::CondBr {
                        taken: default_,
                        not_taken: BranchTarget::zero(),
                        kind: IntegerCompare {
                            kind: IntCC::SignedLessThan,
                            rs1: index,
                            rs2: zero_reg(),
                        },
                    }
                    .emit(&[], sink, emit_info, state);
                    //index >= targets.len()
                    Inst::load_constant_u64(tmp1, targets.len() as u64)
                        .into_iter()
                        .for_each(|i| {
                            i.emit(&[], sink, emit_info, state);
                        });

                    sink.use_label_at_offset(
                        sink.cur_offset(),
                        default_.as_label().unwrap(),
                        LabelUse::B12,
                    );
                    Inst::CondBr {
                        taken: default_,
                        not_taken: BranchTarget::zero(),
                        kind: IntegerCompare {
                            kind: IntCC::UnsignedGreaterThanOrEqual,
                            rs1: index,
                            rs2: tmp1.to_reg(),
                        },
                    }
                    .emit(&[], sink, emit_info, state);
                }
                {
                    let mut insts = SmallInstVec::new();
                    /*
                        offst == 0 make no jump at all, but get the current pc.
                    */
                    let x = Inst::construct_auipc_and_jalr(tmp1, 16);
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
                    insts
                        .into_iter()
                        .for_each(|i| i.emit(&[], sink, emit_info, state));
                }

                /*
                    here is all the jumps
                */
                for t in targets {
                    sink.use_label_at_offset(
                        sink.cur_offset(),
                        t.as_label().unwrap(),
                        LabelUse::PCRel32,
                    );
                    Inst::construct_auipc_and_jalr(writable_spilltmp_reg(), 0)
                        .into_iter()
                        .for_each(|i| i.emit(&[], sink, emit_info, state));
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
            &Inst::FloatFlagOperation { op, rd, rs, imm } => {
                let rd = allocs.next_writable(rd);
                let rs = if let Some(x) = rs {
                    Some(allocs.next(x))
                } else {
                    None
                };

                let x = op.op_code()
                    | reg_to_gpr_num(rd.to_reg()) << 7
                    | op.funct3() << 12
                    | if op == FloatFlagOp::Fsrmi {
                        /*
                            FIXME
                            Riscv64: FloatFlagOperation { op: Fsrmi, rd: Writable { reg: p10i }, rs: None, imm: Some(Imm12 { bits: 2 }) }, fsrmi a0,2
                            gnu:DebugRTypeIns { op_code: 115, rd: 10, funct3: 5, rs1: 2, rs2: 2, funct7: 0 }
                            my :DebugRTypeIns { op_code: 115, rd: 10, funct3: 5, rs1: 0, rs2: 2, funct7: 0 }
                            gnu:DebugITypeIns { op_code: 115, rd: 10, funct3: 5, rs: 2, imm12: 2 }
                            my :DebugITypeIns { op_code: 115, rd: 10, funct3: 5, rs: 0, imm12: 2 }

                            riscv gnu tool chain has set this to be "2"
                            this should be "0"
                        */
                        2 << 15
                    } else {
                        rs.map(|x| reg_to_gpr_num(x)).unwrap_or(0) << 15
                    }
                    | op.imm12(imm) << 20;

                sink.put4(x);
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

            /*
                todo
                why does fence look like have parameter.
                0000 pred succ 00000 000 00000 0001111
                what is pred and succ???????
            */
            &Inst::Fence => sink.put4(0x0ff0000f),
            &Inst::FenceI => sink.put4(0x0000100f),
            &Inst::Auipc { rd, imm } => {
                let rd = allocs.next_writable(rd);

                let x = 0b0010111 | reg_to_gpr_num(rd.to_reg()) << 7 | imm.as_u32() << 12;
                sink.put4(x);
            }
            // &Inst::LoadExtName { rd, name, offset } => todo!(),
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

            &Inst::Ffcmp {
                rd,
                cc,
                ty,
                rs1,
                rs2,
            } => {
                //
                let rs1 = allocs.next(rs1);
                let rs2 = allocs.next(rs2);
                let rd = allocs.next_writable(rd);
                let cc_bit = FloatCCBit::floatcc_2_mask_bits(cc);
                let eq_op = if ty == F32 {
                    AluOPRRR::FeqS
                } else {
                    AluOPRRR::FeqD
                };
                let lt_op = if ty == F32 {
                    AluOPRRR::FltS
                } else {
                    AluOPRRR::FltD
                };
                let le_op = if ty == F32 {
                    AluOPRRR::FleS
                } else {
                    AluOPRRR::FleD
                };
                /*
                    can be implemented by one risc-v instruction.
                */
                let one_instruction_can_do = if cc_bit.just_eq() {
                    Some(eq_op)
                } else if cc_bit.just_le() {
                    Some(le_op)
                } else if cc_bit.just_lt() {
                    Some(lt_op)
                } else {
                    None
                };
                if let Some(op) = one_instruction_can_do {
                    Inst::AluRRR {
                        alu_op: op,
                        rd,
                        rs1,
                        rs2,
                    }
                    .emit(&[], sink, emit_info, state);
                    return;
                }
                // long path
                let mut insts = SmallInstVec::new();
                let label_jump_true = sink.get_label();
                let label_jump_over = sink.get_label();
                // if eq
                if cc_bit.test(FloatCCBit::EQ) {
                    insts.push(Inst::AluRRR {
                        alu_op: eq_op,
                        rd,
                        rs1,
                        rs2,
                    });
                    insts.push(Inst::CondBr {
                        taken: BranchTarget::Label(label_jump_true),
                        not_taken: BranchTarget::zero(),
                        kind: IntegerCompare {
                            kind: IntCC::NotEqual,
                            rs1: rd.to_reg(),
                            rs2: zero_reg(),
                        },
                    });
                }
                // if <
                if cc_bit.test(FloatCCBit::LT) {
                    insts.push(Inst::AluRRR {
                        alu_op: lt_op,
                        rd,
                        rs1,
                        rs2,
                    });
                    insts.push(Inst::CondBr {
                        taken: BranchTarget::Label(label_jump_true),
                        not_taken: BranchTarget::zero(),
                        kind: IntegerCompare {
                            kind: IntCC::NotEqual,
                            rs1: rd.to_reg(),
                            rs2: zero_reg(),
                        },
                    });
                }
                // if gt
                if cc_bit.test(FloatCCBit::GT) {
                    // number is ordered
                    insts.push(Inst::AluRRR {
                        alu_op: lt_op,
                        rd,
                        rs1: rs2,
                        rs2: rs1,
                    });
                    // could be unorder
                    insts.push(Inst::CondBr {
                        taken: BranchTarget::Label(label_jump_true),
                        not_taken: BranchTarget::zero(),
                        kind: IntegerCompare {
                            kind: IntCC::NotEqual,
                            rs1: rd.to_reg(),
                            rs2: zero_reg(),
                        },
                    });
                }
                // if unorder
                if cc_bit.test(FloatCCBit::UN) {
                    insts.extend(Inst::generate_float_unordered(rd, ty, rs1, rs2));
                    insts.push(Inst::Jal {
                        dest: BranchTarget::Label(label_jump_over),
                    });
                }
                insts
                    .into_iter()
                    .for_each(|inst| inst.emit(&[], sink, emit_info, state));

                Inst::load_constant_imm12(rd, Imm12::form_bool(false)).emit(
                    &[],
                    sink,
                    emit_info,
                    state,
                );
                Inst::Jal {
                    dest: BranchTarget::offset(Inst::instruction_size() * 2),
                }
                .emit(&[], sink, emit_info, state);
                // here is true , load true to rd.
                sink.bind_label(label_jump_true);
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
                ty,
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
                let mut patch_false = vec![];
                patch_false.push(insts.len());
                insts.push(Inst::CondBr {
                    taken: BranchTarget::zero(),
                    not_taken: BranchTarget::zero(),
                    kind: IntegerCompare {
                        kind: IntCC::Equal,
                        rs1: conditon,
                        rs2: zero_reg(),
                    },
                });
                // here is the true
                // select the first value
                let select_result =
                    |src: ValueRegs<Reg>, insts: &mut SmallInstVec<Inst>| match ty.bits() {
                        128 => {
                            insts.push(Inst::Mov {
                                rd: dst[0],
                                rm: src.regs()[0],
                                ty: I64,
                            });
                            insts.push(Inst::Mov {
                                rd: dst[1],
                                rm: src.regs()[1],
                                ty: I64,
                            });
                        }
                        _ => {
                            insts.push(Inst::Mov {
                                rd: dst[0],
                                rm: src.regs()[0],
                                ty,
                            });
                        }
                    };
                select_result(x, &mut insts);
                let patch_true = vec![insts.len()];
                insts.push(Inst::Jal {
                    dest: BranchTarget::zero(),
                });
                // here is false
                Inst::patch_taken_list(&mut insts, &patch_false);
                // select second value1
                select_result(y, &mut insts);
                Inst::patch_taken_list(&mut insts, &patch_true);

                insts
                    .into_iter()
                    .for_each(|i| i.emit(&[], sink, emit_info, state));
            }
            &Inst::Jalr { rd, base, offset } => {
                let rd = allocs.next_writable(rd);
                let base = allocs.next(base);
                let x = 0b1100111 |  reg_to_gpr_num(rd.to_reg() )  << 7 |  0b000 << 12 /* funct3 */  | reg_to_gpr_num(base) << 15 |  offset.as_u32() << 20;
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

                todo :: addr dst could be same register!!!!!!!!!!!!
                is this matter??????

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
            &Inst::I128Arithmetic {
                op,
                t0,
                t1,
                ref dst,
                ref x,
                ref y,
            } => {
                let t0 = allocs.next_writable(t0);
                let t1 = allocs.next_writable(t1);
                let x = alloc_value_regs(x, &mut allocs);
                let y = alloc_value_regs(y, &mut allocs);
                let dst: Vec<_> = dst.iter().map(|f| allocs.next_writable(*f)).collect();
                let mut insts = SmallInstVec::new();
                match op {
                    I128OP::Add => {
                        insts.extend(Inst::add_c_u(dst[0], t0, x.regs()[0], y.regs()[0]));
                        insts.push(Inst::AluRRR {
                            alu_op: AluOPRRR::Add,
                            rd: dst[1],
                            rs1: x.regs()[1],
                            rs2: y.regs()[1],
                        });
                        insts.push(Inst::AluRRR {
                            alu_op: AluOPRRR::Add,
                            rd: dst[1],
                            rs1: dst[1].to_reg(),
                            rs2: t0.to_reg(),
                        });
                    }
                    I128OP::Sub => {
                        insts.extend(Inst::sub_b_u(dst[0], t0, x.regs()[0], y.regs()[0]));
                        insts.push(Inst::AluRRR {
                            alu_op: AluOPRRR::Sub,
                            rd: dst[1],
                            rs1: x.regs()[1],
                            rs2: y.regs()[1],
                        });
                        insts.push(Inst::AluRRR {
                            alu_op: AluOPRRR::Sub,
                            rd: dst[1],
                            rs1: dst[1].to_reg(),
                            rs2: t0.to_reg(),
                        });
                    }

                    I128OP::Mul => {
                        todo!()
                    }
                    I128OP::Div => todo!(),
                    I128OP::Rem => todo!(),
                    I128OP::Ishl => todo!(),
                    I128OP::Ushr => todo!(),
                    I128OP::Sshr => todo!(),
                    I128OP::Rotl => todo!(),
                    I128OP::Rotr => todo!(),
                    I128OP::Xnor => todo!(),
                    I128OP::Orn => todo!(),
                    I128OP::Cls => {
                        let mut insts = SmallInstVec::new();
                        // count the high part
                        insts.push(Inst::Cls {
                            rd: t0,
                            rs: x.regs()[1],
                            ty: I64,
                        });
                        insts.push(Inst::load_constant_imm12(t1, Imm12::from_bits(63)));
                        let label_jump_over = vec![insts.len()];
                        insts.push(Inst::CondBr {
                            taken: BranchTarget::zero(),
                            not_taken: BranchTarget::zero(),
                            kind: IntegerCompare {
                                kind: IntCC::NotEqual,
                                rs1: t0.to_reg(),
                                rs2: t1.to_reg(),
                            },
                        });
                        // count lower part
                        insts.push(Inst::Cls {
                            rd: t1,
                            rs: x.regs()[0],
                            ty: I64,
                        });
                        // add the together

                        insts.push(Inst::AluRRR {
                            alu_op: AluOPRRR::Add,
                            rd: dst[0],
                            rs1: t0.to_reg(),
                            rs2: t1.to_reg(),
                        });

                        Inst::patch_taken_list(&mut insts, &label_jump_over);
                        insts
                            .into_iter()
                            .for_each(|i| i.emit(&[], sink, emit_info, state));
                    }
                }
                insts
                    .into_iter()
                    .for_each(|i| i.emit(&[], sink, emit_info, state));
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
                let mut insts = SmallInstVec::new();
                insts.extend(BitsShifter::new_i((64 - ty.bits()) as u8).shift_out_left(rd, rs));
                //extract sign bit.
                {
                    let (op, imm12) = AluOPRRI::Bexti.funct12(Some((ty.bits() - 1) as u8));
                    insts.push(Inst::AluRRImm12 {
                        alu_op: op,
                        rd,
                        rs: rd.to_reg(),
                        imm12,
                    });
                }
                let patch_signed = vec![insts.len()];
                insts.push(Inst::CondBr {
                    taken: BranchTarget::zero(),
                    not_taken: BranchTarget::zero(),
                    kind: IntegerCompare {
                        kind: IntCC::NotEqual,
                        rs1: rd.to_reg(),
                        rs2: zero_reg(),
                    },
                });

                //here is need counting leading zeros.
                let cls = |rd: Writable<Reg>, rs: Reg, insts: &mut SmallInstVec<Inst>| {
                    {
                        let (op, imm12) = AluOPRRI::Clz.funct12(None);
                        insts.push(Inst::AluRRImm12 {
                            alu_op: op,
                            rd,
                            rs,
                            imm12,
                        });
                    }
                    // make result.
                    insts.push(Inst::AluRRImm12 {
                        alu_op: AluOPRRI::Addi,
                        rd,
                        rs: rd.to_reg(),
                        imm12: Imm12::from_bits(-(64 - (ty.bits() - 1) as i16)),
                    });
                };
                cls(rd, rs, &mut insts);
                let patch_jump_over = vec![insts.len()];
                insts.push(Inst::Jal {
                    dest: BranchTarget::zero(),
                });
                Inst::patch_taken_list(&mut insts, &patch_signed);
                // sign bit is 1.
                // reverse all bits.
                insts.push(Inst::construct_bit_not(rd, rs));
                cls(rd, rd.to_reg(), &mut insts);
                Inst::patch_taken_list(&mut insts, &patch_jump_over);

                insts
                    .into_iter()
                    .for_each(|i| i.emit(&[], sink, emit_info, state));
            }
            _ => todo!("{:?}", self),
        };

        let end_off = sink.cur_offset();
        debug_assert!((end_off - start_off) <= Inst::worst_case_size());
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
