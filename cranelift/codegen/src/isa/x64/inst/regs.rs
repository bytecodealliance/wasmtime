//! Register definitions for regalloc2.
//!
//! We define 16 GPRs, with indices equal to the hardware encoding,
//! and 16 XMM registers.
//!
//! Note also that we make use of pinned VRegs to refer to PRegs.

use crate::machinst::Reg;
use alloc::string::String;
use alloc::string::ToString;
use cranelift_assembler_x64::{gpr, xmm};
use regalloc2::{PReg, RegClass, VReg};

// Constructors for Regs.

const fn gpr(enc: u8) -> Reg {
    let preg = gpr_preg(enc);
    Reg::from_virtual_reg(VReg::new(preg.index(), RegClass::Int))
}
pub(crate) const fn gpr_preg(enc: u8) -> PReg {
    PReg::new(enc as usize, RegClass::Int)
}

pub(crate) const fn rax() -> Reg {
    gpr(gpr::enc::RAX)
}
pub(crate) const fn rcx() -> Reg {
    gpr(gpr::enc::RCX)
}
pub(crate) const fn rdx() -> Reg {
    gpr(gpr::enc::RDX)
}
pub(crate) const fn rbx() -> Reg {
    gpr(gpr::enc::RBX)
}
pub(crate) const fn rsp() -> Reg {
    gpr(gpr::enc::RSP)
}
pub(crate) const fn rbp() -> Reg {
    gpr(gpr::enc::RBP)
}
pub(crate) const fn rsi() -> Reg {
    gpr(gpr::enc::RSI)
}
pub(crate) const fn rdi() -> Reg {
    gpr(gpr::enc::RDI)
}
pub(crate) const fn r8() -> Reg {
    gpr(gpr::enc::R8)
}
pub(crate) const fn r9() -> Reg {
    gpr(gpr::enc::R9)
}
pub(crate) const fn r10() -> Reg {
    gpr(gpr::enc::R10)
}
pub(crate) const fn r11() -> Reg {
    gpr(gpr::enc::R11)
}
pub(crate) const fn r12() -> Reg {
    gpr(gpr::enc::R12)
}
pub(crate) const fn r13() -> Reg {
    gpr(gpr::enc::R13)
}
pub(crate) const fn r14() -> Reg {
    gpr(gpr::enc::R14)
}
pub(crate) const fn r15() -> Reg {
    gpr(gpr::enc::R15)
}

/// The pinned register on this architecture.
/// It must be the same as Spidermonkey's HeapReg, as found in this file.
/// https://searchfox.org/mozilla-central/source/js/src/jit/x64/Assembler-x64.h#99
pub(crate) const fn pinned_reg() -> Reg {
    r15()
}

const fn fpr(enc: u8) -> Reg {
    let preg = fpr_preg(enc);
    Reg::from_virtual_reg(VReg::new(preg.index(), RegClass::Float))
}

pub(crate) const fn fpr_preg(enc: u8) -> PReg {
    PReg::new(enc as usize, RegClass::Float)
}

pub(crate) const fn xmm0() -> Reg {
    fpr(xmm::enc::XMM0)
}
pub(crate) const fn xmm1() -> Reg {
    fpr(xmm::enc::XMM1)
}
pub(crate) const fn xmm2() -> Reg {
    fpr(xmm::enc::XMM2)
}
pub(crate) const fn xmm3() -> Reg {
    fpr(xmm::enc::XMM3)
}
pub(crate) const fn xmm4() -> Reg {
    fpr(xmm::enc::XMM4)
}
pub(crate) const fn xmm5() -> Reg {
    fpr(xmm::enc::XMM5)
}
pub(crate) const fn xmm6() -> Reg {
    fpr(xmm::enc::XMM6)
}
pub(crate) const fn xmm7() -> Reg {
    fpr(xmm::enc::XMM7)
}
pub(crate) const fn xmm8() -> Reg {
    fpr(xmm::enc::XMM8)
}
pub(crate) const fn xmm9() -> Reg {
    fpr(xmm::enc::XMM9)
}
pub(crate) const fn xmm10() -> Reg {
    fpr(xmm::enc::XMM10)
}
pub(crate) const fn xmm11() -> Reg {
    fpr(xmm::enc::XMM11)
}
pub(crate) const fn xmm12() -> Reg {
    fpr(xmm::enc::XMM12)
}
pub(crate) const fn xmm13() -> Reg {
    fpr(xmm::enc::XMM13)
}
pub(crate) const fn xmm14() -> Reg {
    fpr(xmm::enc::XMM14)
}
pub(crate) const fn xmm15() -> Reg {
    fpr(xmm::enc::XMM15)
}

// N.B.: this is not an `impl PrettyPrint for Reg` because it is
// specific to x64; other backends have analogous functions. The
// disambiguation happens statically by virtue of higher-level,
// x64-specific, types calling the right `pretty_print_reg`. (In other
// words, we can't pretty-print a `Reg` all by itself in a build that
// may have multiple backends; but we can pretty-print one as part of
// an x64 Inst or x64 RegMemImm.)
pub fn pretty_print_reg(reg: Reg, size: u8) -> String {
    if let Some(rreg) = reg.to_real_reg() {
        let enc = rreg.hw_enc();
        let name = match rreg.class() {
            RegClass::Int => {
                let size = match size {
                    8 => gpr::Size::Quadword,
                    4 => gpr::Size::Doubleword,
                    2 => gpr::Size::Word,
                    1 => gpr::Size::Byte,
                    _ => unreachable!("invalid size"),
                };
                gpr::enc::to_string(enc, size)
            }
            RegClass::Float => xmm::enc::to_string(enc),
            RegClass::Vector => unreachable!(),
        };
        name.to_string()
    } else {
        let mut name = format!("%{reg:?}");
        // Add size suffixes to GPR virtual registers at narrower widths.
        if reg.class() == RegClass::Int && size != 8 {
            name.push_str(match size {
                4 => "l",
                2 => "w",
                1 => "b",
                _ => unreachable!("invalid size"),
            });
        }
        name
    }
}
