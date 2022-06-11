#[allow(unused)]
use crate::ir::LibCall;
use crate::isa::riscv64::inst::*;
use crate::settings;
use alloc::vec::Vec;
use std::process::Command;

/*
    todo:: more instruction.
*/
#[test]
fn test_riscv64_binemit() {
    struct TestUnit {
        inst: Inst,
        assembly: &'static str,
        code: Option<u32>,
        option_for_as: Option<Vec<String>>,
    }

    impl TestUnit {
        fn new(i: Inst, ass: &'static str) -> Self {
            Self {
                inst: i,
                assembly: ass,
                code: None,
                option_for_as: None,
            }
        }
        fn new_with_gcc_option(
            i: Inst,
            ass: &'static str,
            option_for_as: Option<Vec<String>>,
        ) -> Self {
            Self {
                inst: i,
                assembly: ass,
                code: None,
                option_for_as,
            }
        }
    }

    let mut insns = Vec::<TestUnit>::with_capacity(500);

    insns.push(TestUnit::new(
        Inst::Mov {
            rd: writable_fa0(),
            rm: fa1(),
            ty: F32,
        },
        "fmv.s fa0,fa1",
    ));

    insns.push(TestUnit::new(
        Inst::Mov {
            rd: writable_fa0(),
            rm: fa1(),
            ty: F64,
        },
        "fmv.d fa0,fa1",
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Brev8,
            rd: writable_a1(),
            rs: a0(),
            imm12: AluOPRRI::Brev8.funct12(None).1,
        },
        "brev8 a1,a0",
        Some(vec![gcc_aluoprri_march_arg(AluOPRRI::Brev8)]),
    ));
    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Rev8,
            rd: writable_a1(),
            rs: a0(),
            imm12: AluOPRRI::Rev8.funct12(None).1,
        },
        "rev8 a1,a0",
        Some(vec![gcc_aluoprri_march_arg(AluOPRRI::Rev8)]),
    ));

    //
    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Bclri,
            rd: writable_a1(),
            rs: a0(),
            imm12: AluOPRRI::Bclri.funct12(Some(5)).1,
        },
        "bclri a1,a0,5",
        Some(vec![gcc_aluoprri_march_arg(AluOPRRI::Bclri)]),
    ));
    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Bexti,
            rd: writable_a1(),
            rs: a0(),
            imm12: AluOPRRI::Bclri.funct12(Some(5)).1,
        },
        "bexti a1,a0,5",
        Some(vec![gcc_aluoprri_march_arg(AluOPRRI::Bexti)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Binvi,
            rd: writable_a1(),
            rs: a0(),
            imm12: AluOPRRI::Binvi.funct12(Some(5)).1,
        },
        "binvi a1,a0,5",
        Some(vec![gcc_aluoprri_march_arg(AluOPRRI::Binvi)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Bseti,
            rd: writable_a1(),
            rs: a0(),
            imm12: AluOPRRI::Bseti.funct12(Some(5)).1,
        },
        "bseti a1,a0,5",
        Some(vec![gcc_aluoprri_march_arg(AluOPRRI::Bseti)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Rori,
            rd: writable_a1(),
            rs: a0(),
            imm12: AluOPRRI::Rori.funct12(Some(5)).1,
        },
        "rori a1,a0,5",
        Some(vec![gcc_aluoprri_march_arg(AluOPRRI::Rori)]),
    ));
    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Roriw,
            rd: writable_a1(),
            rs: a0(),
            imm12: AluOPRRI::Roriw.funct12(Some(5)).1,
        },
        "roriw a1,a0,5",
        Some(vec![gcc_aluoprri_march_arg(AluOPRRI::Roriw)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::SlliUw,
            rd: writable_a1(),
            rs: a0(),
            imm12: AluOPRRI::SlliUw.funct12(Some(5)).1,
        },
        "slli.uw a1,a0,5",
        Some(vec![gcc_aluoprri_march_arg(AluOPRRI::SlliUw)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Clz,
            rd: writable_a1(),
            rs: a0(),
            imm12: AluOPRRI::Clz.funct12(None).1,
        },
        "clz a1,a0",
        Some(vec![gcc_aluoprri_march_arg(AluOPRRI::Clz)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Clzw,
            rd: writable_a1(),
            rs: a0(),
            imm12: AluOPRRI::Clzw.funct12(None).1,
        },
        "clzw a1,a0",
        Some(vec![gcc_aluoprri_march_arg(AluOPRRI::Clzw)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Cpop,
            rd: writable_a1(),
            rs: a0(),
            imm12: AluOPRRI::Cpop.funct12(None).1,
        },
        "cpop a1,a0",
        Some(vec![gcc_aluoprri_march_arg(AluOPRRI::Cpop)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Cpopw,
            rd: writable_a1(),
            rs: a0(),
            imm12: AluOPRRI::Cpopw.funct12(None).1,
        },
        "cpopw a1,a0",
        Some(vec![gcc_aluoprri_march_arg(AluOPRRI::Cpopw)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Ctz,
            rd: writable_a1(),
            rs: a0(),
            imm12: AluOPRRI::Ctz.funct12(None).1,
        },
        "ctz a1,a0",
        Some(vec![gcc_aluoprri_march_arg(AluOPRRI::Ctz)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Ctzw,
            rd: writable_a1(),
            rs: a0(),
            imm12: AluOPRRI::Ctzw.funct12(None).1,
        },
        "ctzw a1,a0",
        Some(vec![gcc_aluoprri_march_arg(AluOPRRI::Ctzw)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Sextb,
            rd: writable_a1(),
            rs: a0(),
            imm12: AluOPRRI::Sextb.funct12(None).1,
        },
        "sext.b a1,a0",
        Some(vec![gcc_aluoprri_march_arg(AluOPRRI::Sextb)]),
    ));
    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Sexth,
            rd: writable_a1(),
            rs: a0(),
            imm12: AluOPRRI::Sexth.funct12(None).1,
        },
        "sext.h a1,a0",
        Some(vec![gcc_aluoprri_march_arg(AluOPRRI::Sexth)]),
    ));
    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Zexth,
            rd: writable_a1(),
            rs: a0(),
            imm12: AluOPRRI::Zexth.funct12(None).1,
        },
        "zext.h a1,a0",
        Some(vec![gcc_aluoprri_march_arg(AluOPRRI::Zexth)]),
    ));
    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRImm12 {
            alu_op: AluOPRRI::Orcb,
            rd: writable_a1(),
            rs: a0(),
            imm12: AluOPRRI::Orcb.funct12(None).1,
        },
        "orc.b a1,a0",
        Some(vec![gcc_aluoprri_march_arg(AluOPRRI::Orcb)]),
    ));

    //
    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Adduw,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "add.uw a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Adduw)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Andn,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "andn a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Andn)]),
    ));
    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Bclr,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "bclr a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Bclr)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Bext,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "bext a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Bext)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Binv,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "binv a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Binv)]),
    ));
    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Bset,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "bset a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Bset)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Clmul,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "clmul a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Clmul)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Clmulh,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "clmulh a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Clmulh)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Clmulr,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "clmulr a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Clmulr)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Max,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "max a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Max)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Maxu,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "maxu a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Maxu)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Min,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "min a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Min)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Minu,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "minu a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Minu)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Orn,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "orn a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Orn)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Rol,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "rol a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Rol)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Rolw,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "rolw a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Rolw)]),
    ));
    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Ror,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "ror a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Ror)]),
    ));
    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Rorw,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "rorw a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Rorw)]),
    ));
    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Sh1add,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "sh1add a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Sh1add)]),
    ));

    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Sh1adduw,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "sh1add.uw a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Sh1adduw)]),
    ));
    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Sh2add,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "sh2add a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Sh2add)]),
    ));
    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Sh2adduw,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "sh2add.uw a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Sh2adduw)]),
    ));
    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Sh3add,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "sh3add a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Sh3add)]),
    ));
    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Sh3adduw,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "sh3add.uw a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Sh3adduw)]),
    ));
    insns.push(TestUnit::new_with_gcc_option(
        Inst::AluRRR {
            alu_op: AluOPRRR::Xnor,
            rd: writable_a1(),
            rs1: a0(),
            rs2: zero_reg(),
        },
        "xnor a1,a0,zero",
        Some(vec![gcc_aluoprrr_march_arg(AluOPRRR::Xnor)]),
    ));

    //
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
            imm: Umm20::from_bits(120),
        },
        "lui zero,120",
    ));
    insns.push(TestUnit::new(
        Inst::Auipc {
            rd: writable_zero_reg(),
            imm: Umm20::from_bits(120),
        },
        "auipc zero,120",
    ));

    insns.push(TestUnit::new(
        Inst::Jalr {
            rd: writable_a0(),
            base: a0(),
            offset: Imm12::from_bits(100),
        },
        "jalr a0,100(a0)",
    ));

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
        Inst::FpuRRR {
            frm: Some(FRM::RNE),
            alu_op: FpuOPRRR::FaddS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fadd.s fa0,fa0,fa1,rne",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: Some(FRM::RTZ),
            alu_op: FpuOPRRR::FsubS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fsub.s fa0,fa0,fa1,rtz",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: Some(FRM::RUP),
            alu_op: FpuOPRRR::FmulS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fmul.s fa0,fa0,fa1,rup",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: None,
            alu_op: FpuOPRRR::FdivS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fdiv.s fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: None,
            alu_op: FpuOPRRR::FsgnjS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fsgnj.s fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: None,
            alu_op: FpuOPRRR::FsgnjnS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fsgnjn.s fa0,fa0,fa1",
    ));

    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: None,
            alu_op: FpuOPRRR::FsgnjxS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fsgnjx.s fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: None,
            alu_op: FpuOPRRR::FminS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fmin.s fa0,fa0,fa1",
    ));

    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: None,
            alu_op: FpuOPRRR::FmaxS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fmax.s fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: None,
            alu_op: FpuOPRRR::FeqS,
            rd: writable_a0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "feq.s a0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: None,
            alu_op: FpuOPRRR::FltS,
            rd: writable_a0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "flt.s a0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: None,
            alu_op: FpuOPRRR::FleS,
            rd: writable_a0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fle.s a0,fa0,fa1",
    ));

    //
    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: None,
            alu_op: FpuOPRRR::FaddD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fadd.d fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: None,
            alu_op: FpuOPRRR::FsubD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fsub.d fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: None,
            alu_op: FpuOPRRR::FmulD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fmul.d fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: None,
            alu_op: FpuOPRRR::FdivD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fdiv.d fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: None,
            alu_op: FpuOPRRR::FsgnjD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fsgnj.d fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: None,
            alu_op: FpuOPRRR::FsgnjnD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fsgnjn.d fa0,fa0,fa1",
    ));

    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: None,
            alu_op: FpuOPRRR::FsgnjxD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fsgnjx.d fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: None,
            alu_op: FpuOPRRR::FminD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fmin.d fa0,fa0,fa1",
    ));

    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: None,
            alu_op: FpuOPRRR::FmaxD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fmax.d fa0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: None,
            alu_op: FpuOPRRR::FeqD,
            rd: writable_a0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "feq.d a0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: None,
            alu_op: FpuOPRRR::FltD,
            rd: writable_a0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "flt.d a0,fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRR {
            frm: None,
            alu_op: FpuOPRRR::FleD,
            rd: writable_a0(),
            rs1: fa0(),
            rs2: fa1(),
        },
        "fle.d a0,fa0,fa1",
    ));

    //
    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: Some(FRM::RNE),
            alu_op: FpuOPRR::FsqrtS,
            rd: writable_fa0(),
            rs: fa1(),
        },
        "fsqrt.s fa0,fa1,rne",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FcvtWS,
            rd: writable_a0(),
            rs: fa1(),
        },
        "fcvt.w.s a0,fa1",
    ));

    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FcvtWuS,
            rd: writable_a0(),
            rs: fa1(),
        },
        "fcvt.wu.s a0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FmvXW,
            rd: writable_a0(),
            rs: fa1(),
        },
        "fmv.x.w a0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FclassS,
            rd: writable_a0(),
            rs: fa1(),
        },
        "fclass.s a0,fa1",
    ));

    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FcvtSw,
            rd: writable_fa0(),
            rs: a0(),
        },
        "fcvt.s.w fa0,a0",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FcvtSwU,
            rd: writable_fa0(),
            rs: a0(),
        },
        "fcvt.s.wu fa0,a0",
    ));

    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FmvWX,
            rd: writable_fa0(),
            rs: a0(),
        },
        "fmv.w.x fa0,a0",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FcvtLS,
            rd: writable_a0(),
            rs: fa0(),
        },
        "fcvt.l.s a0,fa0",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FcvtLuS,
            rd: writable_a0(),
            rs: fa0(),
        },
        "fcvt.lu.s a0,fa0",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,

            alu_op: FpuOPRR::FcvtSL,
            rd: writable_fa0(),
            rs: a0(),
        },
        "fcvt.s.l fa0,a0",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FcvtSLU,
            rd: writable_fa0(),
            rs: a0(),
        },
        "fcvt.s.lu fa0,a0",
    ));

    //
    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FsqrtD,
            rd: writable_fa0(),
            rs: fa1(),
        },
        "fsqrt.d fa0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FcvtWD,
            rd: writable_a0(),
            rs: fa1(),
        },
        "fcvt.w.d a0,fa1",
    ));

    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FcvtWuD,
            rd: writable_a0(),
            rs: fa1(),
        },
        "fcvt.wu.d a0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FmvXD,
            rd: writable_a0(),
            rs: fa1(),
        },
        "fmv.x.d a0,fa1",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FclassD,
            rd: writable_a0(),
            rs: fa1(),
        },
        "fclass.d a0,fa1",
    ));

    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FcvtSD,
            rd: writable_fa0(),
            rs: fa0(),
        },
        "fcvt.s.d fa0,fa0",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FcvtDWU,
            rd: writable_fa0(),
            rs: a0(),
        },
        "fcvt.d.wu fa0,a0",
    ));

    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FmvDX,
            rd: writable_fa0(),
            rs: a0(),
        },
        "fmv.d.x fa0,a0",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FcvtLD,
            rd: writable_a0(),
            rs: fa0(),
        },
        "fcvt.l.d a0,fa0",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FcvtLuD,
            rd: writable_a0(),
            rs: fa0(),
        },
        "fcvt.lu.d a0,fa0",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FcvtDL,
            rd: writable_fa0(),
            rs: a0(),
        },
        "fcvt.d.l fa0,a0",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRR {
            frm: None,
            alu_op: FpuOPRR::FcvtDLu,
            rd: writable_fa0(),
            rs: a0(),
        },
        "fcvt.d.lu fa0,a0",
    ));
    //////////////////////

    insns.push(TestUnit::new(
        Inst::FpuRRRR {
            frm: Some(FRM::RNE),
            alu_op: FpuOPRRRR::FmaddS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
            rs3: fa7(),
        },
        "fmadd.s fa0,fa0,fa1,fa7,rne",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRRR {
            frm: None,
            alu_op: FpuOPRRRR::FmsubS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
            rs3: fa7(),
        },
        "fmsub.s fa0,fa0,fa1,fa7",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRRR {
            frm: None,
            alu_op: FpuOPRRRR::FnmsubS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
            rs3: fa7(),
        },
        "fnmsub.s fa0,fa0,fa1,fa7",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRRR {
            frm: None,
            alu_op: FpuOPRRRR::FnmaddS,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
            rs3: fa7(),
        },
        "fnmadd.s fa0,fa0,fa1,fa7",
    ));

    insns.push(TestUnit::new(
        Inst::FpuRRRR {
            frm: None,
            alu_op: FpuOPRRRR::FmaddD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
            rs3: fa7(),
        },
        "fmadd.d fa0,fa0,fa1,fa7",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRRR {
            frm: None,

            alu_op: FpuOPRRRR::FmsubD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
            rs3: fa7(),
        },
        "fmsub.d fa0,fa0,fa1,fa7",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRRR {
            frm: None,
            alu_op: FpuOPRRRR::FnmsubD,
            rd: writable_fa0(),
            rs1: fa0(),
            rs2: fa1(),
            rs3: fa7(),
        },
        "fnmsub.d fa0,fa0,fa1,fa7",
    ));
    insns.push(TestUnit::new(
        Inst::FpuRRRR {
            frm: None,
            alu_op: FpuOPRRRR::FnmaddD,
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

    {
        /*
        notice!!
            some generate code
            if you modify "insns"
            please remove all this block source code and regenerated this.
         */
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
            let code = assemble(unit.assembly, &unit.option_for_as);
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
                println!("gnu:{:b}", gnu.op_code);
                println!("my :{:b}", my.op_code);
            }
            assert_eq!(buffer.data(), unit.code.unwrap().to_le_bytes());
        }
    }
    if missing_code.len() > 0 {
        println!("// generated code to speed up the test unit,otherwise you need invode riscv-gun tool chain every time.");
        for i in missing_code {
            println!("insns[{}].code = Some({});  ", i.0, i.1);
        }
        println!("");
    }
}

#[cfg(target_os = "windows")]
fn get_riscv_tool_chain_name() -> (String, String) {
    (
        String::from("riscv64-unknown-elf-as"),
        String::from("riscv64-unknown-elf-objdump"),
    )
}

#[cfg(target_os = "linux")]
fn get_riscv_tool_chain_name() -> (String, String) {
    (
        String::from("riscv64-unknown-linux-gnu-as"),
        String::from("riscv64-unknown-linux-gnu-objdump"),
    )
}

/*
    todo:: make this can be run on windows
*/
fn assemble(code: &str, as_option: &Option<Vec<String>>) -> u32 {
    let mut code = String::from(code);
    code.push_str("\n");

    std::env::set_current_dir(std::env::temp_dir()).expect("set_current_dir {}");

    let file_name = "riscv_tmp.s";
    use std::io::Write;
    let mut file = std::fs::File::create(file_name).unwrap();

    file.write_all(code.as_bytes()).expect("write error {}");
    file.sync_all().unwrap();
    let (as_name, objdump_name) = get_riscv_tool_chain_name();
    let mut cmd = Command::new(as_name.as_str());
    as_option.clone().map(|ref a| cmd.args(&a[..]));
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

/*
    need enable to support bitmanip extension.
*/
fn gcc_aluoprrr_march_arg(op: AluOPRRR) -> String {
    let x = match op {
        AluOPRRR::Adduw => "-march=rv64g_zba",
        AluOPRRR::Andn => "-march=rv64g_zbb",
        AluOPRRR::Bclr => "-march=rv64g_zbs",
        AluOPRRR::Bext => "-march=rv64g_zbs",
        AluOPRRR::Binv => "-march=rv64g_zbs",
        AluOPRRR::Bset => "-march=rv64g_zbs",
        AluOPRRR::Clmul => "-march=rv64g_zbc",
        AluOPRRR::Clmulh => "-march=rv64g_zbc",
        AluOPRRR::Clmulr => "-march=rv64g_zbc",
        AluOPRRR::Max => "-march=rv64g_zbb",
        AluOPRRR::Maxu => "-march=rv64g_zbb",
        AluOPRRR::Min => "-march=rv64g_zbb",
        AluOPRRR::Minu => "-march=rv64g_zbb",
        AluOPRRR::Orn => "-march=rv64g_zbb",
        AluOPRRR::Rol => "-march=rv64g_zbb",
        AluOPRRR::Rolw => "-march=rv64g_zbb",
        AluOPRRR::Ror => "-march=rv64g_zbb",
        AluOPRRR::Rorw => "-march=rv64g_zbb",
        AluOPRRR::Sh1add => "-march=rv64g_zba",
        AluOPRRR::Sh1adduw => "-march=rv64g_zba",
        AluOPRRR::Sh2add => "-march=rv64g_zba",
        AluOPRRR::Sh2adduw => "-march=rv64g_zba",
        AluOPRRR::Sh3add => "-march=rv64g_zba",
        AluOPRRR::Sh3adduw => "-march=rv64g_zba",
        AluOPRRR::Xnor => "-march=rv64g_zbb",
        _ => unreachable!(),
    };
    x.into()
}

/*
    need enable to support bitmanip extension.
*/
fn gcc_aluoprri_march_arg(op: AluOPRRI) -> String {
    let x = match op {
        AluOPRRI::Bclri => "-march=rv64g_zbs",
        AluOPRRI::Bexti => "-march=rv64g_zbs",
        AluOPRRI::Binvi => "-march=rv64g_zbs",
        AluOPRRI::Bseti => "-march=rv64g_zbs",
        AluOPRRI::Rori => "-march=rv64g_zbb",
        AluOPRRI::Roriw => "-march=rv64g_zbb",
        AluOPRRI::SlliUw => "-march=rv64g_zba",
        AluOPRRI::Clz => "-march=rv64g_zbb",
        AluOPRRI::Clzw => "-march=rv64g_zbb",
        AluOPRRI::Cpop => "-march=rv64g_zbb",
        AluOPRRI::Cpopw => "-march=rv64g_zbb",
        AluOPRRI::Ctz => "-march=rv64g_zbb",
        AluOPRRI::Ctzw => "-march=rv64g_zbb",
        AluOPRRI::Rev8 => "-march=rv64g_zbb",
        AluOPRRI::Sextb => "-march=rv64g_zbb",
        AluOPRRI::Sexth => "-march=rv64g_zbb",
        AluOPRRI::Zexth => "-march=rv64g_zbb",
        AluOPRRI::Orcb => "-march=rv64g_zbb",
        AluOPRRI::Brev8 => "-march=rv64g_zbkb",
        _ => unreachable!(),
    };
    x.into()
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

    shamt5: u32,
    shamt6: u32,
    funct7: u32,
    funct6: u32,
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
        let shamt5 = imm12 & 0b1_1111;
        let shamt6 = imm12 & 0b11_1111;
        let funct7 = imm12 >> 5;
        let funct6 = funct7 >> 1;
        Self {
            op_code,
            rd,
            funct3,
            rs,
            imm12,
            shamt5,
            shamt6,
            funct7,
            funct6,
        }
    }
    fn print_b(self) {
        println!("opcode:{:b}", self.op_code);
        println!("rd:{}", self.rd);
        println!("funct3:{:b}", self.funct3);
        println!("rs:{}", self.rs);
        println!("shamt5:{:b}", self.shamt5);
        println!("shamt6:{:b}", self.shamt6);
        println!("funct6:{:b}", self.funct6);
        println!("funct7:{:b}", self.funct7);
    }
}

#[test]
fn xxx() {
    let x = 1240847763;
    let x = DebugITypeIns::from_u32(x);
    x.print_b();
}

#[test]
fn riscv64_worst_case_size_instrcution_size() {
    let flags = settings::Flags::new(settings::builder());
    let emit_info = EmitInfo::new(flags);

    /*
        there are all candidates potential generate a lot of bytes.
    */
    let mut candidates: Vec<MInst> = vec![];
    /*
        a load with large offset need more registers.
        load will push register and pop registers.
    */
    candidates.push(Inst::Load {
        rd: writable_zero_reg(),
        op: LoadOP::Ld,
        flags: MemFlags::new(),
        from: AMode::SPOffset(4096 * 100, I64),
    });
    /*
        call may need load extern names.
    */
    // candidates.push(Inst::Call {
    //     info: Box::new(CallInfo {
    //         dest: ExternalName::LibCall(LibCall::IshlI64),
    //         uses: vec![],
    //         defs: vec![],
    //         opcode: crate::ir::Opcode::Call,
    //         caller_callconv: crate::isa::CallConv::SystemV,
    //         callee_callconv: crate::isa::CallConv::SystemV,
    //     }),
    // });
    //
    candidates.push(Inst::Fcmp {
        rd: writable_fa0(),
        cc: FloatCC::UnorderedOrLessThanOrEqual,
        ty: F64,
        rs1: fa1(),
        rs2: fa0(),
    });

    candidates.push(Inst::Select {
        dst: vec![writable_a0(), writable_a1()],
        ty: I128,
        conditon: a0(),
        x: ValueRegs::two(x_reg(1), x_reg(2)),
        y: ValueRegs::two(x_reg(3), x_reg(4)),
    });

    // brtable max size is base one how many "targets" it's has.

    // cas
    candidates.push(Inst::AtomicCas {
        dst: writable_a0(),
        ty: I64,
        t0: writable_a1(),
        e: a0(),
        addr: a1(),
        v: a2(),
    });

    /*
        todo:: I128Arithmetic
    */
    candidates.push(Inst::IntSelect {
        dst: vec![writable_a0(), writable_a0()],
        ty: I128,
        op: IntSelectOP::Imax,
        x: ValueRegs::two(x_reg(1), x_reg(2)),
        y: ValueRegs::two(x_reg(3), x_reg(4)),
    });

    candidates.push(Inst::Cls {
        rs: a0(),
        rd: writable_a1(),
        ty: I8,
    });

    candidates.push(Inst::SelectReg {
        rd: writable_a0(),
        rs1: a2(),
        rs2: a7(),
        condition: IntegerCompare {
            kind: IntCC::Equal,
            rs1: x_reg(5),
            rs2: x_reg(6),
        },
    });
    candidates.push(Inst::SelectIf {
        if_spectre_guard: true,
        rd: vec![writable_a0(), writable_a1()],
        cmp_x: ValueRegs::two(a0(), a1()),
        cmp_y: ValueRegs::two(a0(), a1()),
        cc: IntCC::SignedLessThan,
        cmp_ty: I128,
        x: ValueRegs::two(x_reg(1), x_reg(2)),
        y: ValueRegs::two(x_reg(3), x_reg(4)),
    });

    candidates.push(Inst::SelectIf {
        if_spectre_guard: true,
        rd: vec![writable_a0(), writable_a1()],
        cmp_x: ValueRegs::two(a0(), a1()),
        cmp_y: ValueRegs::two(a0(), a1()),
        cc: IntCC::SignedLessThan,
        cmp_ty: I128,
        x: ValueRegs::two(x_reg(1), x_reg(2)),
        y: ValueRegs::two(x_reg(3), x_reg(4)),
    });
    candidates.push(Inst::FcvtToIntSat {
        rd: writable_a0(),
        rs: fa0(),
        tmp: writable_a1(),
        is_signed: true,
        in_type: F64,
        out_type: I64,
    });

    let mut max: (u32, MInst) = (0, Inst::Nop0);
    for i in candidates {
        let mut buffer = MachBuffer::new();
        i.emit(&[], &mut buffer, &emit_info, &mut Default::default());
        let buffer = buffer.finish();
        let length = buffer.data().len() as u32;
        if length > max.0 {
            let length = buffer.data().len() as u32;
            max = (length, i);
        }
    }
    println!("caculate max size is {} , inst is {:?}", max.0, max.1);
    assert!(max.0 == Inst::worst_case_size());
}

/*
    emit an instruction and see objdump assembly code.
*/
#[test]
fn gcc_ass() {
    let flags = settings::Flags::new(settings::builder());
    let emit_info = EmitInfo::new(flags);
    std::env::set_current_dir(std::env::temp_dir()).expect("set_current_dir {}");
    let file_name = "riscv_tmp.bin";
    use std::io::Write;
    /*
        there are all candidates potential generate a lot of bytes.
    */
    let mut insts: Vec<MInst> = vec![];
    insts.push(Inst::FpuRRR {
        alu_op: FpuOPRRR::FaddD,
        frm: Some(FRM::RNE),
        rd: writable_fa0(),
        rs1: fa0(),
        rs2: fa1(),
    });
    let (_, dump) = get_riscv_tool_chain_name();
    for i in insts {
        let mut buffer = MachBuffer::new();
        i.emit(&[], &mut buffer, &emit_info, &mut Default::default());
        let buffer = buffer.finish();
        let mut file = std::fs::File::create(file_name).unwrap();
        file.write_all(buffer.data()).expect("write error {}");
        file.sync_all().unwrap();
        let mut cmd = Command::new(dump.as_str());
        cmd.args(&["-b", "binary", "-m", "riscv:rv64", "-D"])
            .arg(file_name);
        let output = cmd.output().expect("exec objdump failed , {}");
        println!("{:?} ############################################", i);
        println!(
            "##############################{}",
            String::from_iter(output.stdout.clone().into_iter().map(|c| c as char))
        );
        println!(
            "##############################{}",
            String::from_iter(output.stderr.clone().into_iter().map(|c| c as char))
        );
    }
}
