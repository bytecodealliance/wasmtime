use crate::isa::riscv64::inst::*;
use crate::settings;
use alloc::vec::Vec;

/*
    todo:: more instruction
    todo:: risc  tool chain jump is wired.............
*/
#[test]
fn test_riscv64_binemit() {
    struct TestUnit {
        inst: Inst,
        assembly: &'static str,
        code: Option<u32>,
    }
    impl TestUnit {
        fn new(i: Inst, ass: &'static str) -> Self {
            Self {
                inst: i,
                assembly: ass,
                code: None,
            }
        }
    }

    let mut insns = Vec::<TestUnit>::with_capacity(500);
    //todo:: more
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Add,
            rd: writable_fp_reg(),
            rs1: fp_reg(),
            rs2: zero_reg(),
        },
        "add fp,fp,zero",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Addi,
            rd: writable_fp_reg(),
            rs: stack_reg(),
            imm12: Imm12::maybe_from_u64(100).unwrap(),
        },
        "addi fp,sp,100",
    ));
    insns.push(TestUnit::new(
        Inst::Lui {
            rd: writable_zero_reg(),
            imm: Imm20::from_bits(120),
        },
        "lui zero,120",
    ));
    insns.push(TestUnit::new(
        Inst::Auipc {
            rd: writable_zero_reg(),
            imm: Imm20::from_bits(120),
        },
        "auipc zero,120",
    ));

    /*
        todo :  jal zero,120 generate this.
           a.out:     file format elf64-littleriscv

    Disassembly of section .text:

    0000000000000000 <.text>:
    0:   0000006f                j       0x0
    */

    // insns.push(TestUnit::new(
    //     Inst::Jal {
    //         rd: writable_a0(),
    //         dest: BranchTarget::offset(120),
    //     },
    //     "jal a0,120",
    // ));

    insns.push(TestUnit::new(
        Inst::Jalr {
            rd: writable_a0(),
            base: a0(),
            offset: Imm12::from_bits(100),
        },
        "jalr a0,100(a0)",
    ));

    /*
        todo::gnu tool chain  generate looks quit not right
    */
    // insns.push(TestUnit::new(
    //     Inst::CondBr {
    //         taken: BranchTarget::offset(4),
    //         not_taken: BranchTarget::zero(),
    //         kind: IntegerCompare {
    //             kind: IntCC::Equal,
    //             rs1: a0(),
    //             rs2: a0(),
    //         },
    //     },
    //     "beq a0,a0,4\nj 0",
    // ));
    insns.push(TestUnit::new(
        Inst::Load {
            rd: writable_a0(),
            op: LoadOP::Lb,
            flags: MemFlags::new(),
            from: AMode::RegOffset(a1(), 100, I8),
        },
        "lb a0,100(a1)",
    ));
    insns.push(TestUnit::new(
        Inst::Load {
            rd: writable_a0(),
            op: LoadOP::Lbu,
            flags: MemFlags::new(),
            from: AMode::RegOffset(a1(), 100, B8),
        },
        "lbu a0,100(a1)",
    ));
    insns.push(TestUnit::new(
        Inst::Load {
            rd: writable_a0(),
            op: LoadOP::Lh,
            flags: MemFlags::new(),
            from: AMode::RegOffset(a1(), 100, I16),
        },
        "lh a0,100(a1)",
    ));

    insns.push(TestUnit::new(
        Inst::Load {
            rd: writable_a0(),
            op: LoadOP::Lhu,
            flags: MemFlags::new(),
            from: AMode::RegOffset(a1(), 100, B16),
        },
        "lhu a0,100(a1)",
    ));

    insns.push(TestUnit::new(
        Inst::Load {
            rd: writable_a0(),
            op: LoadOP::Lw,
            flags: MemFlags::new(),
            from: AMode::RegOffset(a1(), 100, I32),
        },
        "lw a0,100(a1)",
    ));

    insns.push(TestUnit::new(
        Inst::Load {
            rd: writable_a0(),
            op: LoadOP::Lwu,
            flags: MemFlags::new(),
            from: AMode::RegOffset(a1(), 100, B32),
        },
        "lwu a0,100(a1)",
    ));
    insns.push(TestUnit::new(
        Inst::Load {
            rd: writable_a0(),
            op: LoadOP::Ld,
            flags: MemFlags::new(),
            from: AMode::RegOffset(a1(), 100, I64),
        },
        "ld a0,100(a1)",
    ));
    insns.push(TestUnit::new(
        Inst::Load {
            rd: Writable::from_reg(fa0()),
            op: LoadOP::Flw,
            flags: MemFlags::new(),
            from: AMode::RegOffset(a1(), 100, I64),
        },
        "flw fa0,100(a1)",
    ));

    insns.push(TestUnit::new(
        Inst::Load {
            rd: Writable::from_reg(fa0()),
            op: LoadOP::Fld,
            flags: MemFlags::new(),
            from: AMode::RegOffset(a1(), 100, I64),
        },
        "fld fa0,100(a1)",
    ));
    insns.push(TestUnit::new(
        Inst::Store {
            to: AMode::SPOffset(100, I8),
            op: StoreOP::Sb,
            flags: MemFlags::new(),
            src: a0(),
        },
        "sb a0,100(sp)",
    ));
    insns.push(TestUnit::new(
        Inst::Store {
            to: AMode::SPOffset(100, I16),
            op: StoreOP::Sh,
            flags: MemFlags::new(),
            src: a0(),
        },
        "sh a0,100(sp)",
    ));
    insns.push(TestUnit::new(
        Inst::Store {
            to: AMode::SPOffset(100, I32),
            op: StoreOP::Sw,
            flags: MemFlags::new(),
            src: a0(),
        },
        "sw a0,100(sp)",
    ));
    insns.push(TestUnit::new(
        Inst::Store {
            to: AMode::SPOffset(100, I64),
            op: StoreOP::Sd,
            flags: MemFlags::new(),
            src: a0(),
        },
        "sd a0,100(sp)",
    ));
    insns.push(TestUnit::new(
        Inst::Store {
            to: AMode::SPOffset(100, I64),
            op: StoreOP::Fsw,
            flags: MemFlags::new(),
            src: fa0(),
        },
        "fsw fa0,100(sp)",
    ));
    insns.push(TestUnit::new(
        Inst::Store {
            to: AMode::SPOffset(100, I64),
            op: StoreOP::Fsd,
            flags: MemFlags::new(),
            src: fa0(),
        },
        "fsd fa0,100(sp)",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Addi,
            rd: writable_a0(),
            rs: a0(),
            imm12: Imm12::from_bits(100),
        },
        "addi a0,a0,100",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Slti,
            rd: writable_a0(),
            rs: a0(),
            imm12: Imm12::from_bits(100),
        },
        "slti a0,a0,100",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::SltiU,
            rd: writable_a0(),
            rs: a0(),
            imm12: Imm12::from_bits(100),
        },
        "sltiu a0,a0,100",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Xori,
            rd: writable_a0(),
            rs: a0(),
            imm12: Imm12::from_bits(100),
        },
        "xori a0,a0,100",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Andi,
            rd: writable_a0(),
            rs: a0(),
            imm12: Imm12::from_bits(100),
        },
        "andi a0,a0,100",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Slli,
            rd: writable_a0(),
            rs: a0(),
            imm12: Imm12::from_bits(5),
        },
        "slli a0,a0,5",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Srli,
            rd: writable_a0(),
            rs: a0(),
            imm12: Imm12::from_bits(5),
        },
        "srli a0,a0,5",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Srai,
            rd: writable_a0(),
            rs: a0(),
            imm12: Imm12::from_bits(5),
        },
        "srai a0,a0,5",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Addiw,
            rd: writable_a0(),
            rs: a0(),
            imm12: Imm12::from_bits(120),
        },
        "addiw a0,a0,120",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Slliw,
            rd: writable_a0(),
            rs: a0(),
            imm12: Imm12::from_bits(5),
        },
        "slliw a0,a0,5",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::SrliW,
            rd: writable_a0(),
            rs: a0(),
            imm12: Imm12::from_bits(5),
        },
        "srliw a0,a0,5",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Sraiw,
            rd: writable_a0(),
            rs: a0(),
            imm12: Imm12::from_bits(5),
        },
        "sraiw a0,a0,5",
    ));

    insns.push(TestUnit::new(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Sraiw,
            rd: writable_a0(),
            rs: a0(),
            imm12: Imm12::from_bits(5),
        },
        "sraiw a0,a0,5",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Add,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "add a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Sub,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "sub a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Sll,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "sll a0,a0,a1",
    ));

    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Slt,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "slt a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::SltU,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "sltu a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Xor,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "xor a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Srl,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "srl a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Sra,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "sra a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Or,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "or a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::And,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "and a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Addw,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "addw a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Subw,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "subw a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Sllw,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "sllw a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Srlw,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "srlw a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Sraw,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "sraw a0,a0,a1",
    ));

    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Mul,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "mul a0,a0,a1",
    ));

    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Mulh,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "mulh a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Mulhsu,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "mulhsu a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Mulhu,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "mulhu a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Div,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "div a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::DivU,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "divu a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Rem,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "rem a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::RemU,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "remu a0,a0,a1",
    ));

    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Mulw,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "mulw a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Divw,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "divw a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Remw,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "remw a0,a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::Remuw,
            rd: writable_a0(),
            rs1: a0(),
            rs2: a1(),
        },
        "remuw a0,a0,a1",
    ));

    //
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FaddS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fadd.s fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FsubS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fsub.s fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FmulS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fmul.s fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FdivS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fdiv.s fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FsgnjS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fsgnj.s fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FsgnjnS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fsgnjn.s fa0,fa0,fa1",
    ));

    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FsgnjxS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fsgnjx.s fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FminS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fmin.s fa0,fa0,fa1",
    ));

    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FmaxS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fmax.s fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FeqS,
            rd: writable_a0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "feq.s a0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FltS,
            rd: writable_a0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "flt.s a0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FleS,
            rd: writable_a0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fle.s a0,fa0,fa1",
    ));

    //
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FaddD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fadd.d fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FsubD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fsub.d fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FmulD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fmul.d fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FdivD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fdiv.d fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FsgnjD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fsgnj.d fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FsgnjnD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fsgnjn.d fa0,fa0,fa1",
    ));

    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FsgnjxD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fsgnjx.d fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FminD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fmin.d fa0,fa0,fa1",
    ));

    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FmaxD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fmax.d fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FeqD,
            rd: writable_a0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "feq.d a0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FltD,
            rd: writable_a0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "flt.d a0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRR {
            alu_op: AluOPRRR::FleD,
            rd: writable_a0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fle.d a0,fa0,fa1",
    ));

    //
    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FsqrtS,
            rd: writable_fa0(),
            rs: fa1(),
        },
        "fsqrt.s fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FcvtWS,
            rd: writable_a0(),
            rs: fa1(),
        },
        "fcvt.w.s a0,fa1",
    ));

    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FcvtWuS,
            rd: writable_a0(),
            rs: fa1(),
        },
        "fcvt.wu.s a0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FmvXW,
            rd: writable_a0(),
            rs: fa1(),
        },
        "fmv.x.w a0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FclassS,
            rd: writable_a0(),
            rs: fa1(),
        },
        "fclass.s a0,fa1",
    ));

    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FcvtSw,
            rd: writable_fa0(),
            rs: a0(),
        },
        "fcvt.s.w fa0,a0",
    ));
    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FcvtSwU,
            rd: writable_fa0(),
            rs: a0(),
        },
        "fcvt.s.wu fa0,a0",
    ));

    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FmvWX,
            rd: writable_fa0(),
            rs: a0(),
        },
        "fmv.w.x fa0,a0",
    ));
    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FcvtLS,
            rd: writable_a0(),
            rs: fa0(),
        },
        "fcvt.l.s a0,fa0",
    ));
    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FcvtLuS,
            rd: writable_a0(),
            rs: fa0(),
        },
        "fcvt.lu.s a0,fa0",
    ));
    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FcvtSL,
            rd: writable_fa0(),
            rs: a0(),
        },
        "fcvt.s.l fa0,a0",
    ));
    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FcvtSLU,
            rd: writable_fa0(),
            rs: a0(),
        },
        "fcvt.s.lu fa0,a0",
    ));

    /////////////////////////
    ///
    ///
    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FsqrtD,
            rd: writable_fa0(),
            rs: fa1(),
        },
        "fsqrt.d fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FcvtWD,
            rd: writable_a0(),
            rs: fa1(),
        },
        "fcvt.w.d a0,fa1",
    ));

    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FcvtWuD,
            rd: writable_a0(),
            rs: fa1(),
        },
        "fcvt.wu.d a0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FmvXD,
            rd: writable_a0(),
            rs: fa1(),
        },
        "fmv.x.d a0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FclassD,
            rd: writable_a0(),
            rs: fa1(),
        },
        "fclass.d a0,fa1",
    ));

    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FcvtSd,
            rd: writable_fa0(),
            rs: fa0(),
        },
        "fcvt.s.d fa0,fa0",
    ));
    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FcvtDWU,
            rd: writable_fa0(),
            rs: a0(),
        },
        "fcvt.d.wu fa0,a0",
    ));

    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FmvDX,
            rd: writable_fa0(),
            rs: a0(),
        },
        "fmv.d.x fa0,a0",
    ));
    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FcvtLd,
            rd: writable_a0(),
            rs: fa0(),
        },
        "fcvt.l.d a0,fa0",
    ));
    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FcvtLuD,
            rd: writable_a0(),
            rs: fa0(),
        },
        "fcvt.lu.d a0,fa0",
    ));
    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FcvtDL,
            rd: writable_fa0(),
            rs: a0(),
        },
        "fcvt.d.l fa0,a0",
    ));
    insns.push(TestUnit::new(
        Inst::AluRR {
            alu_op: AluOPRR::FcvtDLu,
            rd: writable_fa0(),
            rs: a0(),
        },
        "fcvt.d.lu fa0,a0",
    ));
    //////////////////////

    insns.push(TestUnit::new(
        Inst::AluRRRR {
            alu_op: AluOPRRRR::FmaddS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
            rs3: fa7(),
        },
        "fmadd.s fa0,fa0,fa1,fa7",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRRR {
            alu_op: AluOPRRRR::FmsubS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
            rs3: fa7(),
        },
        "fmsub.s fa0,fa0,fa1,fa7",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRRR {
            alu_op: AluOPRRRR::FnmsubS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
            rs3: fa7(),
        },
        "fnmsub.s fa0,fa0,fa1,fa7",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRRR {
            alu_op: AluOPRRRR::FnmaddS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
            rs3: fa7(),
        },
        "fnmadd.s fa0,fa0,fa1,fa7",
    ));

    insns.push(TestUnit::new(
        Inst::AluRRRR {
            alu_op: AluOPRRRR::FmaddD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
            rs3: fa7(),
        },
        "fmadd.d fa0,fa0,fa1,fa7",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRRR {
            alu_op: AluOPRRRR::FmsubD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
            rs3: fa7(),
        },
        "fmsub.d fa0,fa0,fa1,fa7",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRRR {
            alu_op: AluOPRRRR::FnmsubD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
            rs3: fa7(),
        },
        "fnmsub.d fa0,fa0,fa1,fa7",
    ));
    insns.push(TestUnit::new(
        Inst::AluRRRR {
            alu_op: AluOPRRRR::FnmaddD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
            rs3: fa7(),
        },
        "fnmadd.d fa0,fa0,fa1,fa7",
    ));
    ///////////
    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::LrW,
            rd: writable_a0(),
            addr: a1(),
            src: zero_reg(),
            aq: true,
            rl: false,
        },
        "lr.w.aq a0,(a1)",
    ));
    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::ScW,
            rd: writable_a0(),
            addr: a1(),
            src: a2(),
            aq: false,
            rl: true,
        },
        "sc.w.rl a0,a2,(a1)",
    ));
    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::AmoswapW,
            rd: writable_a0(),
            addr: a1(),
            src: a2(),
            aq: false,
            rl: false,
        },
        "amoswap.w a0,a2,(a1)",
    ));

    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::AmoaddW,
            rd: writable_a0(),
            addr: a1(),
            src: a2(),
            aq: false,
            rl: false,
        },
        "amoadd.w a0,a2,(a1)",
    ));
    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::AmoxorW,
            rd: writable_a0(),
            addr: a1(),
            src: a2(),
            aq: false,
            rl: false,
        },
        "amoxor.w a0,a2,(a1)",
    ));
    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::AmoandW,
            rd: writable_a0(),
            addr: a1(),
            src: a2(),
            aq: false,
            rl: false,
        },
        "amoand.w a0,a2,(a1)",
    ));

    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::AmoorW,
            rd: writable_a0(),
            addr: a1(),
            src: a2(),
            aq: false,
            rl: false,
        },
        "amoor.w a0,a2,(a1)",
    ));
    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::AmominW,
            rd: writable_a0(),
            addr: a1(),
            src: a2(),
            aq: false,
            rl: false,
        },
        "amomin.w a0,a2,(a1)",
    ));
    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::AmomaxW,
            rd: writable_a0(),
            addr: a1(),
            src: a2(),
            aq: false,
            rl: false,
        },
        "amomax.w a0,a2,(a1)",
    ));
    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::AmominuW,
            rd: writable_a0(),
            addr: a1(),
            src: a2(),
            aq: false,
            rl: false,
        },
        "amominu.w a0,a2,(a1)",
    ));
    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::AmomaxuW,
            rd: writable_a0(),
            addr: a1(),
            src: a2(),
            aq: false,
            rl: false,
        },
        "amomaxu.w a0,a2,(a1)",
    ));

    /////////////////////
    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::LrD,
            rd: writable_a0(),
            addr: a1(),
            src: zero_reg(),
            aq: true,
            rl: false,
        },
        "lr.d.aq a0,(a1)",
    ));
    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::ScD,
            rd: writable_a0(),
            addr: a1(),
            src: a2(),
            aq: false,
            rl: true,
        },
        "sc.d.rl a0,a2,(a1)",
    ));
    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::AmoswapD,
            rd: writable_a0(),
            addr: a1(),
            src: a2(),
            aq: false,
            rl: false,
        },
        "amoswap.d a0,a2,(a1)",
    ));

    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::AmoaddD,
            rd: writable_a0(),
            addr: a1(),
            src: a2(),
            aq: false,
            rl: false,
        },
        "amoadd.d a0,a2,(a1)",
    ));
    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::AmoxorD,
            rd: writable_a0(),
            addr: a1(),
            src: a2(),
            aq: false,
            rl: false,
        },
        "amoxor.d a0,a2,(a1)",
    ));
    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::AmoandD,
            rd: writable_a0(),
            addr: a1(),
            src: a2(),
            aq: false,
            rl: false,
        },
        "amoand.d a0,a2,(a1)",
    ));

    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::AmoorD,
            rd: writable_a0(),
            addr: a1(),
            src: a2(),
            aq: false,
            rl: false,
        },
        "amoor.d a0,a2,(a1)",
    ));
    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::AmominD,
            rd: writable_a0(),
            addr: a1(),
            src: a2(),
            aq: false,
            rl: false,
        },
        "amomin.d a0,a2,(a1)",
    ));
    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::AmomaxD,
            rd: writable_a0(),
            addr: a1(),
            src: a2(),
            aq: false,
            rl: false,
        },
        "amomax.d a0,a2,(a1)",
    ));
    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::AmominuD,
            rd: writable_a0(),
            addr: a1(),
            src: a2(),
            aq: false,
            rl: false,
        },
        "amominu.d a0,a2,(a1)",
    ));
    insns.push(TestUnit::new(
        Inst::Atomic {
            op: AtomicOP::AmomaxuD,
            rd: writable_a0(),
            addr: a1(),
            src: a2(),
            aq: false,
            rl: false,
        },
        "amomaxu.d a0,a2,(a1)",
    ));

    /////////
    insns.push(TestUnit::new(Inst::Fence {}, "fence"));
    insns.push(TestUnit::new(Inst::FenceI {}, "fence.i"));
    insns.push(TestUnit::new(Inst::ECall {}, "ecall"));
    insns.push(TestUnit::new(Inst::EBreak {}, "ebreak"));
    ////// fcsrs
    insns.push(TestUnit::new(
        Inst::FloatFlagOperation {
            op: FloatFlagOp::Frcsr,
            rd: writable_a0(),
            rs: None,
            imm: None,
        },
        "frcsr a0",
    ));
    insns.push(TestUnit::new(
        Inst::FloatFlagOperation {
            op: FloatFlagOp::Frrm,
            rd: writable_a0(),
            rs: None,
            imm: None,
        },
        "frrm a0",
    ));
    insns.push(TestUnit::new(
        Inst::FloatFlagOperation {
            op: FloatFlagOp::Frflags,
            rd: writable_a0(),
            rs: None,
            imm: None,
        },
        "frflags a0",
    ));

    insns.push(TestUnit::new(
        Inst::FloatFlagOperation {
            op: FloatFlagOp::Fscsr,
            rd: writable_a0(),
            rs: Some(a1()),
            imm: None,
        },
        "fscsr a0,a1",
    ));

    insns.push(TestUnit::new(
        Inst::FloatFlagOperation {
            op: FloatFlagOp::Fsrm,
            rd: writable_a0(),
            rs: Some(a1()),
            imm: None,
        },
        "fsrm a0,a1",
    ));
    insns.push(TestUnit::new(
        Inst::FloatFlagOperation {
            op: FloatFlagOp::Fsflags,
            rd: writable_a0(),
            rs: Some(a1()),
            imm: None,
        },
        "fsflags a0,a1",
    ));

    insns.push(TestUnit::new(
        Inst::FloatFlagOperation {
            op: FloatFlagOp::Fsrmi,
            rd: writable_a0(),
            rs: None,
            imm: Some(FloatRoundingMode::RDN.to_imm12()),
        },
        "fsrmi a0,2",
    ));

    /*
        todo:: FSFLAGSI rd,+64
        $ ./as_and_dump.sh
        a.s: Assembler messages:
        a.s:1: Error: Instruction fsflagsi requires absolute expression
        a.s:1: Error: illegal operands `fsflagsi rd,+64'
        C:\SysGCC\risc-v\bin\riscv64-unknown-elf-objdump.exe: 'a.out': No such file
        gnu tool chain have this error,figure it out...
    */

    // insns.push(TestUnit::new(
    //     Inst::FloatFlagOperation {
    //         op: FloatFlagOp::Fsflagsi,
    //         rd: writable_a0(),
    //         rs: None,
    //         imm: Some(FFlags::new(FloatRoundingMode::RDN).to_imm12()),
    //     },
    //     "fsflagsi a0,64",
    // ));
    {
        /*
        notice!!
            some generate code
            if you modify "insns"
            please remove all this block source code and regenerated this.
         */
        // generated code to speed up the test unit,otherwise you need invode riscv-gun tool chain every time.
        insns[0].code = Some(263219);
        insns[1].code = Some(104924179);
        insns[2].code = Some(491575);
        insns[3].code = Some(491543);
        insns[4].code = Some(105186663);
        insns[5].code = Some(105219331);
        insns[6].code = Some(105235715);
        insns[7].code = Some(105223427);
        insns[8].code = Some(105239811);
        insns[9].code = Some(105227523);
        insns[10].code = Some(105243907);
        insns[11].code = Some(105231619);
        insns[12].code = Some(105227527);
        insns[13].code = Some(105231623);
        insns[14].code = Some(111215139);
        insns[15].code = Some(111219235);
        insns[16].code = Some(111223331);
        insns[17].code = Some(111227427);
        insns[18].code = Some(111223335);
        insns[19].code = Some(111227431);
        insns[20].code = Some(105186579);
        insns[21].code = Some(105194771);
        insns[22].code = Some(105198867);
        insns[23].code = Some(105202963);
        insns[24].code = Some(105215251);
        insns[25].code = Some(5575955);
        insns[26].code = Some(5592339);
        insns[27].code = Some(1079334163);
        insns[28].code = Some(126158107);
        insns[29].code = Some(5575963);
        insns[30].code = Some(5592347);
        insns[31].code = Some(1079334171);
        insns[32].code = Some(1079334171);
        insns[33].code = Some(11863347);
        insns[34].code = Some(1085605171);
        insns[35].code = Some(11867443);
        insns[36].code = Some(11871539);
        insns[37].code = Some(11875635);
        insns[38].code = Some(11879731);
        insns[39].code = Some(11883827);
        insns[40].code = Some(1085625651);
        insns[41].code = Some(11887923);
        insns[42].code = Some(11892019);
        insns[43].code = Some(11863355);
        insns[44].code = Some(1085605179);
        insns[45].code = Some(11867451);
        insns[46].code = Some(11883835);
        insns[47].code = Some(1085625659);
        insns[48].code = Some(45417779);
        insns[49].code = Some(45421875);
        insns[50].code = Some(45425971);
        insns[51].code = Some(45430067);
        insns[52].code = Some(45434163);
        insns[53].code = Some(45438259);
        insns[54].code = Some(45442355);
        insns[55].code = Some(45446451);
        insns[56].code = Some(45417787);
        insns[57].code = Some(45434171);
        insns[58].code = Some(45442363);
        insns[59].code = Some(45446459);
        insns[60].code = Some(11892051);
        insns[61].code = Some(146109779);
        insns[62].code = Some(280327507);
        insns[63].code = Some(414545235);
        insns[64].code = Some(548734291);
        insns[65].code = Some(548738387);
        insns[66].code = Some(548742483);
        insns[67].code = Some(682952019);
        insns[68].code = Some(682956115);
        insns[69].code = Some(2696226131);
        insns[70].code = Some(2696222035);
        insns[71].code = Some(2696217939);
        insns[72].code = Some(45446483);
        insns[73].code = Some(179664211);
        insns[74].code = Some(313881939);
        insns[75].code = Some(448099667);
        insns[76].code = Some(582288723);
        insns[77].code = Some(582292819);
        insns[78].code = Some(582296915);
        insns[79].code = Some(716506451);
        insns[80].code = Some(716510547);
        insns[81].code = Some(2729780563);
        insns[82].code = Some(2729776467);
        insns[83].code = Some(2729772371);
        insns[84].code = Some(1476785491);
        insns[85].code = Some(3221615955);
        insns[86].code = Some(3222664531);
        insns[87].code = Some(3758458195);
        insns[88].code = Some(3758462291);
        insns[89].code = Some(3490018643);
        insns[90].code = Some(3491067219);
        insns[91].code = Some(4026860883);
        insns[92].code = Some(3223680339);
        insns[93].code = Some(3224728915);
        insns[94].code = Some(3492115795);
        insns[95].code = Some(3493164371);
        insns[96].code = Some(1510339923);
        insns[97].code = Some(3255170387);
        insns[98].code = Some(3256218963);
        insns[99].code = Some(3792012627);
        insns[100].code = Some(3792016723);
        insns[101].code = Some(1075148115);
        insns[102].code = Some(3524592979);
        insns[103].code = Some(4060415315);
        insns[104].code = Some(3257234771);
        insns[105].code = Some(3258283347);
        insns[106].code = Some(3525670227);
        insns[107].code = Some(3526718803);
        insns[108].code = Some(2293593411);
        insns[109].code = Some(2293593415);
        insns[110].code = Some(2293593419);
        insns[111].code = Some(2293593423);
        insns[112].code = Some(2327147843);
        insns[113].code = Some(2327147847);
        insns[114].code = Some(2327147851);
        insns[115].code = Some(2327147855);
        insns[116].code = Some(335914287);
        insns[117].code = Some(449160495);
        insns[118].code = Some(147170607);
        insns[119].code = Some(12952879);
        insns[120].code = Some(549823791);
        insns[121].code = Some(1623565615);
        insns[122].code = Some(1086694703);
        insns[123].code = Some(2160436527);
        insns[124].code = Some(2697307439);
        insns[125].code = Some(3234178351);
        insns[126].code = Some(3771049263);
        insns[127].code = Some(335918383);
        insns[128].code = Some(449164591);
        insns[129].code = Some(147174703);
        insns[130].code = Some(12956975);
        insns[131].code = Some(549827887);
        insns[132].code = Some(1623569711);
        insns[133].code = Some(1086698799);
        insns[134].code = Some(2160440623);
        insns[135].code = Some(2697311535);
        insns[136].code = Some(3234182447);
        insns[137].code = Some(3771053359);
        insns[138].code = Some(267386895);
        insns[139].code = Some(4111);
        insns[140].code = Some(115);
        insns[141].code = Some(1048691);
        insns[142].code = Some(3155315);
        insns[143].code = Some(2106739);
        insns[144].code = Some(1058163);
        insns[145].code = Some(3511667);
        insns[146].code = Some(2463091);
        insns[147].code = Some(1414515);
        insns[148].code = Some(2184563);
    }
    let flags = settings::Flags::new(settings::builder());
    let emit_info = EmitInfo::new(flags);
    let mut missing_code = vec![];
    for (index, ref mut unit) in insns.into_iter().enumerate() {
        println!("Riscv64: {:?}, {}", unit.inst, unit.assembly);
        // Check the printed text is as expected.
        let actual_printing = unit
            .inst
            .print_with_state(&mut EmitState::default(), &mut AllocationConsumer::new(&[]));
        assert_eq!(unit.assembly, actual_printing);
        if unit.code.is_none() {
            let code = assemble(unit.assembly);
            missing_code.push((index, code));
            unit.code = Some(code);
        }
        let mut buffer = MachBuffer::new();
        unit.inst
            .emit(&[], &mut buffer, &emit_info, &mut Default::default());
        let buffer = buffer.finish();
        if buffer.data() != unit.code.unwrap().to_le_bytes() {
            {
                let gnu = DebugRTypeIns::from_bs(&unit.code.unwrap().to_le_bytes());
                let my = DebugRTypeIns::from_bs(buffer.data());
                println!("gnu:{:?}", gnu);
                println!("my :{:?}", my);
                // println!("gnu:{:b}", gnu.funct7);
                // println!("my :{:b}", my.funct7);
            }

            {
                let gnu = DebugITypeIns::from_bs(&unit.code.unwrap().to_le_bytes());
                let my = DebugITypeIns::from_bs(buffer.data());
                println!("gnu:{:?}", gnu);
                println!("my :{:?}", my);
                // println!("gnu:{:b}", gnu.funct7);
                // println!("my :{:b}", my.funct7);
            }
            assert_eq!(buffer.data(), unit.code.unwrap().to_le_bytes());
        }
    }
    if missing_code.len() > 0 {
        println!("// generated code to speed up the test unit,otherwise you need invode riscv-gun tool chain every time.");
        for i in missing_code {
            println!("insns[{}].code = Some({});", i.0, i.1);
        }
        println!("");
    }
}

#[cfg(windows)]
fn get_riscv_tool_chain_name() -> (String, String) {
    (
        String::from("riscv64-unknown-elf-as"),
        String::from("riscv64-unknown-elf-objdump"),
    )
}

#[cfg(linux)]
fn get_riscv_tool_chain_name() -> (String, String) {}
/*
    todo:: make this can be run on windows
*/
fn assemble(code: &str) -> u32 {
    use std::process::Command;
    let (as_name, objdump_name) = get_riscv_tool_chain_name();
    std::env::set_current_dir(std::env::temp_dir()).expect("set_current_dir {}");
    let file_name = "riscv_tmp.s";
    use std::io::Write;
    let mut file = std::fs::File::create(file_name).unwrap();
    file.write_all(code.as_bytes()).expect("write error {}");
    let mut cmd = Command::new(as_name.as_str());
    cmd.arg(file_name);
    let _output = cmd.output().expect("exec as failed , {}");
    let output_file = "a.out";
    let mut cmd = Command::new(objdump_name.as_str());
    cmd.arg("-d").arg(output_file);

    let output = cmd.output().expect("exec objdump failed , {}");
    /*
        a.out:     file format elf64-littleriscv

    Disassembly of section .text:

    0000000000000000 <.text>:
       0:   fe010113                addi    sp,sp,-32
        */
    let output = output.stdout;
    // println!(
    //     "##############################{}",
    //     String::from_iter(output.clone().into_iter().map(|c| c as char))
    // );
    // need parse this
    // right row only generate one instruction.
    // so it is easy
    for mut i in 0..output.len() {
        // match   0:
        let mut _match = || -> bool {
            if output[i] == ('0' as u8) && output[i + 1] == (':' as u8) {
                // skip 0:
                i += 2;
                true
            } else {
                false
            }
        };
        if _match() {
            // skip all white space or \t
            loop {
                if output[i] != 32 && output[i] != 9 {
                    break;
                }
                i += 1;
            }
            let mut byte_string: String = "".into();
            loop {
                if (output[i] >= ('0' as u8) && output[i] <= ('9' as u8))
                    || (output[i] >= ('a' as u8) && output[i] <= ('f' as u8))
                {
                    byte_string.push(output[i] as char);
                    i += 1;
                } else {
                    break;
                }
            }
            return u32::from_str_radix(byte_string.as_str(), 16).unwrap();
        }
    }
    unreachable!()
}

#[derive(Debug)]
pub(crate) struct DebugRTypeIns {
    op_code: u32,
    rd: u32,
    funct3: u32,
    rs1: u32,
    rs2: u32,
    funct7: u32,
}

impl DebugRTypeIns {
    pub(crate) fn from_bs(x: &[u8]) -> Self {
        let a = [x[0], x[1], x[2], x[3]];
        Self::from_u32(u32::from_le_bytes(a))
    }

    pub(crate) fn from_u32(x: u32) -> Self {
        let op_code = x & 0b111_1111;
        let x = x >> 7;
        let rd = x & 0b1_1111;
        let x = x >> 5;
        let funct3 = x & 0b111;
        let x = x >> 3;
        let rs1 = x & 0b1_1111;
        let x = x >> 5;
        let rs2 = x & 0b1_1111;
        let x = x >> 5;
        let funct7 = x & 0b111_1111;
        Self {
            op_code,
            rd,
            funct3,
            rs1,
            rs2,
            funct7,
        }
    }
}

#[derive(Debug)]
pub(crate) struct DebugITypeIns {
    op_code: u32,
    rd: u32,
    funct3: u32,
    rs: u32,
    imm12: u32,
}

impl DebugITypeIns {
    pub(crate) fn from_bs(x: &[u8]) -> Self {
        let a = [x[0], x[1], x[2], x[3]];
        Self::from_u32(u32::from_le_bytes(a))
    }

    pub(crate) fn from_u32(x: u32) -> Self {
        let op_code = x & 0b111_1111;
        let x = x >> 7;
        let rd = x & 0b1_1111;
        let x = x >> 5;
        let funct3 = x & 0b111;
        let x = x >> 3;
        let rs = x & 0b1_1111;
        let x = x >> 5;

        let imm12 = x & 0b1111_1111_1111;
        Self {
            op_code,
            rd,
            funct3,
            rs,
            imm12,
        }
    }
}
