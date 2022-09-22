/// X64 register definition
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

fn gpr(enc: u8) -> PReg {
    PReg::new(enc as usize, RegClass::Int)
}

/// Constructors for GPR

pub(crate) fn rsi() -> PReg {
    gpr(ENC_RSI)
}
pub(crate) fn rdi() -> PReg {
    gpr(ENC_RDI)
}
pub(crate) fn rax() -> PReg {
    gpr(ENC_RAX)
}
pub(crate) fn rcx() -> PReg {
    gpr(ENC_RCX)
}
pub(crate) fn rdx() -> PReg {
    gpr(ENC_RDX)
}
pub(crate) fn r8() -> PReg {
    gpr(ENC_R8)
}
pub(crate) fn r9() -> PReg {
    gpr(ENC_R9)
}
pub(crate) fn r10() -> PReg {
    gpr(ENC_R10)
}
pub(crate) fn r11() -> PReg {
    gpr(ENC_R11)
}
pub(crate) fn r12() -> PReg {
    gpr(ENC_R12)
}
pub(crate) fn r13() -> PReg {
    gpr(ENC_R13)
}
pub(crate) fn r14() -> PReg {
    gpr(ENC_R14)
}
pub(crate) fn rbx() -> PReg {
    gpr(ENC_RBX)
}

pub(crate) fn r15() -> PReg {
    gpr(ENC_R15)
}

pub(crate) fn rsp() -> PReg {
    gpr(ENC_RSP)
}
pub(crate) fn rbp() -> PReg {
    gpr(ENC_RBP)
}

fn fpr(enc: u8) -> PReg {
    PReg::new(enc as usize, RegClass::Float)
}

/// Constructors for FPR

pub(crate) fn xmm0() -> PReg {
    fpr(0)
}
pub(crate) fn xmm1() -> PReg {
    fpr(1)
}
pub(crate) fn xmm2() -> PReg {
    fpr(2)
}
pub(crate) fn xmm3() -> PReg {
    fpr(3)
}
pub(crate) fn xmm4() -> PReg {
    fpr(4)
}
pub(crate) fn xmm5() -> PReg {
    fpr(5)
}
pub(crate) fn xmm6() -> PReg {
    fpr(6)
}
pub(crate) fn xmm7() -> PReg {
    fpr(7)
}
pub(crate) fn xmm8() -> PReg {
    fpr(8)
}
pub(crate) fn xmm9() -> PReg {
    fpr(9)
}
pub(crate) fn xmm10() -> PReg {
    fpr(10)
}
pub(crate) fn xmm11() -> PReg {
    fpr(11)
}
pub(crate) fn xmm12() -> PReg {
    fpr(12)
}
pub(crate) fn xmm13() -> PReg {
    fpr(13)
}
pub(crate) fn xmm14() -> PReg {
    fpr(14)
}
pub(crate) fn xmm15() -> PReg {
    fpr(15)
}

// Temporatily removing the % from the register name
// for debugging purposes only until winch gets disasm
// support
pub(crate) fn reg_name(preg: PReg) -> &'static str {
    match preg.class() {
        RegClass::Int => match preg.hw_enc() as u8 {
            ENC_RAX => "rax",
            ENC_RBX => "rbx",
            ENC_RCX => "rcx",
            ENC_RDX => "rdx",
            ENC_RSI => "rsi",
            ENC_RDI => "rdi",
            ENC_RBP => "rbp",
            ENC_RSP => "rsp",
            ENC_R8 => "r8",
            ENC_R9 => "r9",
            ENC_R10 => "r10",
            ENC_R11 => "r11",
            ENC_R12 => "r12",
            ENC_R13 => "r13",
            ENC_R14 => "r14",
            ENC_R15 => "r15",
            _ => panic!("Invalid PReg: {:?}", preg),
        },
        RegClass::Float => match preg.hw_enc() {
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
            _ => panic!("Invalid PReg: {:?}", preg),
        },
    }
}
