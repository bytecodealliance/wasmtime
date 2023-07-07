//! X64 register definition.

use crate::isa::{reg::Reg, CallingConvention};
use regalloc2::{PReg, RegClass};
use smallvec::{smallvec, SmallVec};

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
pub(crate) fn r12() -> Reg {
    gpr(ENC_R12)
}
pub(crate) fn r13() -> Reg {
    gpr(ENC_R13)
}
/// Used as a pinned register to hold
/// the `VMContext`.
/// Non-allocatable in Winch's default
/// ABI, and callee-saved in SystemV and
/// Fastcall.
pub(crate) fn r14() -> Reg {
    gpr(ENC_R14)
}

pub(crate) fn vmctx() -> Reg {
    r14()
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

/// Used as the scratch register.
/// Non-allocatable in Winch's default
/// ABI.
pub(crate) fn r11() -> Reg {
    gpr(ENC_R11)
}

pub(crate) fn scratch() -> Reg {
    r11()
}

/// This register is used as a scratch register, in the context of trampolines only,
/// where we assume that callee-saved registers are given the correct handling
/// according to the system ABI. r12 is chosen given that it's a callee-saved,
/// non-argument register.
///
/// In the context of all other internal functions, this register is not excluded
/// from register allocation, so no extra assumptions should be made regarding
/// its availability.
pub(crate) fn argv() -> Reg {
    r12()
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
const NON_ALLOCATABLE_GPR: u32 = (1 << ENC_RBP) | (1 << ENC_RSP) | (1 << ENC_R11) | (1 << ENC_R14);

/// Bitmask to represent the available general purpose registers.
pub(crate) const ALL_GPR: u32 = ALLOCATABLE_GPR & !NON_ALLOCATABLE_GPR;

/// Returns the callee-saved registers according to a particular calling
/// convention.
///
/// This function will return the set of registers that need to be saved
/// according to the system ABI and that are known not to be saved during the
/// prologue emission.
pub(crate) fn callee_saved(call_conv: &CallingConvention) -> SmallVec<[Reg; 9]> {
    use CallingConvention::*;
    match call_conv {
        WasmtimeSystemV => {
            smallvec![rbx(), r12(), r13(), r14(), r15(),]
        }
        // TODO: Once float registers are supported,
        // account for callee-saved float registers.
        WindowsFastcall => {
            smallvec![rbx(), rdi(), rsi(), r12(), r13(), r14(), r15(),]
        }
        _ => unreachable!(),
    }
}
