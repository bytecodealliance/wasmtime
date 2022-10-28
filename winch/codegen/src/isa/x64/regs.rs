//! X64 register definition.

use crate::isa::reg::Reg;
use regalloc2::{PReg, RegClass};

const ENC_RAX: u8 = 0;
const ENC_RCX: u8 = 1;
const ENC_RDX: u8 = 2;
const ENC_RBX: u8 = 3;
const ENC_RSP: u8 = 4;
const ENC_RBP: u8 = 5;
const ENC_RSI: u8 = 6;
const ENC_RDI: u8 = 7;
const ENC_R8: u8 = 8;
const ENC_R9: u8 = 9;
const ENC_R10: u8 = 10;
const ENC_R11: u8 = 11;
const ENC_R12: u8 = 12;
const ENC_R13: u8 = 13;
const ENC_R14: u8 = 14;
const ENC_R15: u8 = 15;

fn gpr(enc: u8) -> Reg {
    Reg::new(PReg::new(enc as usize, RegClass::Int))
}

/// Constructors for GPR.

pub(crate) fn rsi() -> Reg {
    gpr(ENC_RSI)
}
pub(crate) fn rdi() -> Reg {
    gpr(ENC_RDI)
}
pub(crate) fn rax() -> Reg {
    gpr(ENC_RAX)
}
pub(crate) fn rcx() -> Reg {
    gpr(ENC_RCX)
}
pub(crate) fn rdx() -> Reg {
    gpr(ENC_RDX)
}
pub(crate) fn r8() -> Reg {
    gpr(ENC_R8)
}
pub(crate) fn r9() -> Reg {
    gpr(ENC_R9)
}
pub(crate) fn r10() -> Reg {
    gpr(ENC_R10)
}
pub(crate) fn r11() -> Reg {
    gpr(ENC_R11)
}
pub(crate) fn r12() -> Reg {
    gpr(ENC_R12)
}
pub(crate) fn r13() -> Reg {
    gpr(ENC_R13)
}
pub(crate) fn r14() -> Reg {
    gpr(ENC_R14)
}
pub(crate) fn rbx() -> Reg {
    gpr(ENC_RBX)
}

pub(crate) fn r15() -> Reg {
    gpr(ENC_R15)
}

pub(crate) fn rsp() -> Reg {
    gpr(ENC_RSP)
}
pub(crate) fn rbp() -> Reg {
    gpr(ENC_RBP)
}

pub(crate) fn scratch() -> Reg {
    r11()
}

fn fpr(enc: u8) -> Reg {
    Reg::new(PReg::new(enc as usize, RegClass::Float))
}

/// Constructors for FPR.

pub(crate) fn xmm0() -> Reg {
    fpr(0)
}
pub(crate) fn xmm1() -> Reg {
    fpr(1)
}
pub(crate) fn xmm2() -> Reg {
    fpr(2)
}
pub(crate) fn xmm3() -> Reg {
    fpr(3)
}
pub(crate) fn xmm4() -> Reg {
    fpr(4)
}
pub(crate) fn xmm5() -> Reg {
    fpr(5)
}
pub(crate) fn xmm6() -> Reg {
    fpr(6)
}
pub(crate) fn xmm7() -> Reg {
    fpr(7)
}
pub(crate) fn xmm8() -> Reg {
    fpr(8)
}
pub(crate) fn xmm9() -> Reg {
    fpr(9)
}
pub(crate) fn xmm10() -> Reg {
    fpr(10)
}
pub(crate) fn xmm11() -> Reg {
    fpr(11)
}
pub(crate) fn xmm12() -> Reg {
    fpr(12)
}
pub(crate) fn xmm13() -> Reg {
    fpr(13)
}
pub(crate) fn xmm14() -> Reg {
    fpr(14)
}
pub(crate) fn xmm15() -> Reg {
    fpr(15)
}

const GPR: u32 = 16;
const ALLOCATABLE_GPR: u32 = (1 << GPR) - 1;
const NON_ALLOCATABLE_GPR: u32 = (1 << ENC_RBP) | (1 << ENC_RSP) | (1 << ENC_R11);

/// Bitmask to represent the available general purpose registers.
pub(crate) const ALL_GPR: u32 = ALLOCATABLE_GPR & !NON_ALLOCATABLE_GPR;

// Temporarily removing the % from the register name
// for debugging purposes only until winch gets disasm
// support.
pub(crate) fn reg_name(reg: Reg, size: u8) -> &'static str {
    match reg.class() {
        RegClass::Int => match (reg.hw_enc() as u8, size) {
            (ENC_RAX, 8) => "rax",
            (ENC_RAX, 4) => "eax",
            (ENC_RAX, 2) => "ax",
            (ENC_RAX, 1) => "al",
            (ENC_RBX, 8) => "rbx",
            (ENC_RBX, 4) => "ebx",
            (ENC_RBX, 2) => "bx",
            (ENC_RBX, 1) => "bl",
            (ENC_RCX, 8) => "rcx",
            (ENC_RCX, 4) => "ecx",
            (ENC_RCX, 2) => "cx",
            (ENC_RCX, 1) => "cl",
            (ENC_RDX, 8) => "rdx",
            (ENC_RDX, 4) => "edx",
            (ENC_RDX, 2) => "dx",
            (ENC_RDX, 1) => "dl",
            (ENC_RSI, 8) => "rsi",
            (ENC_RSI, 4) => "esi",
            (ENC_RSI, 2) => "si",
            (ENC_RSI, 1) => "sil",
            (ENC_RDI, 8) => "rdi",
            (ENC_RDI, 4) => "edi",
            (ENC_RDI, 2) => "di",
            (ENC_RDI, 1) => "dil",
            (ENC_RBP, 8) => "rbp",
            (ENC_RBP, 4) => "ebp",
            (ENC_RBP, 2) => "bp",
            (ENC_RBP, 1) => "bpl",
            (ENC_RSP, 8) => "rsp",
            (ENC_RSP, 4) => "esp",
            (ENC_RSP, 2) => "sp",
            (ENC_RSP, 1) => "spl",
            (ENC_R8, 8) => "r8",
            (ENC_R8, 4) => "r8d",
            (ENC_R8, 2) => "r8w",
            (ENC_R8, 1) => "r8b",
            (ENC_R9, 8) => "r9",
            (ENC_R9, 4) => "r9d",
            (ENC_R9, 2) => "r9w",
            (ENC_R9, 1) => "r9b",
            (ENC_R10, 8) => "r10",
            (ENC_R10, 4) => "r10d",
            (ENC_R10, 2) => "r10w",
            (ENC_R10, 1) => "r10b",
            (ENC_R11, 8) => "r11",
            (ENC_R11, 4) => "r11d",
            (ENC_R11, 2) => "r11w",
            (ENC_R11, 1) => "r11b",
            (ENC_R12, 8) => "r12",
            (ENC_R12, 4) => "r12d",
            (ENC_R12, 2) => "r12w",
            (ENC_R12, 1) => "r12b",
            (ENC_R13, 8) => "r13",
            (ENC_R13, 4) => "r13d",
            (ENC_R13, 2) => "r13w",
            (ENC_R13, 1) => "r13b",
            (ENC_R14, 8) => "r14",
            (ENC_R14, 4) => "r14d",
            (ENC_R14, 2) => "r14w",
            (ENC_R14, 1) => "r14b",
            (ENC_R15, 8) => "r15",
            (ENC_R15, 4) => "r15d",
            (ENC_R15, 2) => "r15w",
            (ENC_R15, 1) => "r15b",
            _ => panic!("Invalid Reg: {:?}", reg),
        },
        RegClass::Float => match reg.hw_enc() {
            0 => "xmm0",
            1 => "xmm1",
            2 => "xmm2",
            3 => "xmm3",
            4 => "xmm4",
            5 => "xmm5",
            6 => "xmm6",
            7 => "xmm7",
            8 => "xmm8",
            9 => "xmm9",
            10 => "xmm10",
            11 => "xmm11",
            12 => "xmm12",
            13 => "xmm13",
            14 => "xmm14",
            15 => "xmm15",
            _ => panic!("Invalid Reg: {:?}", reg),
        },
    }
}
