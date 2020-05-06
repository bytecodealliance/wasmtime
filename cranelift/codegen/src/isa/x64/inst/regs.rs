//! Registers, the Universe thereof, and printing.
//!
//! These are ordered by sequence number, as required in the Universe.  The strange ordering is
//! intended to make callee-save registers available before caller-saved ones.  This is a net win
//! provided that each function makes at least one onward call.  It'll be a net loss for leaf
//! functions, and we should change the ordering in that case, so as to make caller-save regs
//! available first.
//!
//! TODO Maybe have two different universes, one for leaf functions and one for non-leaf functions?
//! Also, they will have to be ABI dependent.  Need to find a way to avoid constructing a universe
//! for each function we compile.

use alloc::vec::Vec;
use std::string::String;

use regalloc::{RealReg, RealRegUniverse, Reg, RegClass, RegClassInfo, NUM_REG_CLASSES};

use crate::machinst::pretty_print::ShowWithRRU;
use crate::settings;

// Hardware encodings for a few registers.

pub const ENC_RBX: u8 = 3;
pub const ENC_RSP: u8 = 4;
pub const ENC_RBP: u8 = 5;
pub const ENC_R12: u8 = 12;
pub const ENC_R13: u8 = 13;
pub const ENC_R14: u8 = 14;
pub const ENC_R15: u8 = 15;

fn gpr(enc: u8, index: u8) -> Reg {
    Reg::new_real(RegClass::I64, enc, index)
}

pub(crate) fn r12() -> Reg {
    gpr(ENC_R12, 0)
}
pub(crate) fn r13() -> Reg {
    gpr(ENC_R13, 1)
}
pub(crate) fn r14() -> Reg {
    gpr(ENC_R14, 2)
}
pub(crate) fn r15() -> Reg {
    gpr(ENC_R15, 3)
}
pub(crate) fn rbx() -> Reg {
    gpr(ENC_RBX, 4)
}
pub(crate) fn rsi() -> Reg {
    gpr(6, 5)
}
pub(crate) fn rdi() -> Reg {
    gpr(7, 6)
}
pub(crate) fn rax() -> Reg {
    gpr(0, 7)
}
pub(crate) fn rcx() -> Reg {
    gpr(1, 8)
}
pub(crate) fn rdx() -> Reg {
    gpr(2, 9)
}
pub(crate) fn r8() -> Reg {
    gpr(8, 10)
}
pub(crate) fn r9() -> Reg {
    gpr(9, 11)
}
pub(crate) fn r10() -> Reg {
    gpr(10, 12)
}
pub(crate) fn r11() -> Reg {
    gpr(11, 13)
}

fn fpr(enc: u8, index: u8) -> Reg {
    Reg::new_real(RegClass::V128, enc, index)
}
fn xmm0() -> Reg {
    fpr(0, 14)
}
fn xmm1() -> Reg {
    fpr(1, 15)
}
fn xmm2() -> Reg {
    fpr(2, 16)
}
fn xmm3() -> Reg {
    fpr(3, 17)
}
fn xmm4() -> Reg {
    fpr(4, 18)
}
fn xmm5() -> Reg {
    fpr(5, 19)
}
fn xmm6() -> Reg {
    fpr(6, 20)
}
fn xmm7() -> Reg {
    fpr(7, 21)
}
fn xmm8() -> Reg {
    fpr(8, 22)
}
fn xmm9() -> Reg {
    fpr(9, 23)
}
fn xmm10() -> Reg {
    fpr(10, 24)
}
fn xmm11() -> Reg {
    fpr(11, 25)
}
fn xmm12() -> Reg {
    fpr(12, 26)
}
fn xmm13() -> Reg {
    fpr(13, 27)
}
fn xmm14() -> Reg {
    fpr(14, 28)
}
fn xmm15() -> Reg {
    fpr(15, 29)
}

pub(crate) fn rsp() -> Reg {
    gpr(ENC_RSP, 30)
}
pub(crate) fn rbp() -> Reg {
    gpr(ENC_RBP, 31)
}

/// Create the register universe for X64.
///
/// The ordering of registers matters, as commented in the file doc comment: assumes the
/// calling-convention is SystemV, at the moment.
pub(crate) fn create_reg_universe_systemv(_flags: &settings::Flags) -> RealRegUniverse {
    let mut regs = Vec::<(RealReg, String)>::new();
    let mut allocable_by_class = [None; NUM_REG_CLASSES];

    // Integer regs.
    let mut base = regs.len();

    // Callee-saved, in the SystemV x86_64 ABI.
    regs.push((r12().to_real_reg(), "%r12".into()));
    regs.push((r13().to_real_reg(), "%r13".into()));
    regs.push((r14().to_real_reg(), "%r14".into()));
    regs.push((r15().to_real_reg(), "%r15".into()));
    regs.push((rbx().to_real_reg(), "%rbx".into()));

    // Caller-saved, in the SystemV x86_64 ABI.
    regs.push((rsi().to_real_reg(), "%rsi".into()));
    regs.push((rdi().to_real_reg(), "%rdi".into()));
    regs.push((rax().to_real_reg(), "%rax".into()));
    regs.push((rcx().to_real_reg(), "%rcx".into()));
    regs.push((rdx().to_real_reg(), "%rdx".into()));
    regs.push((r8().to_real_reg(), "%r8".into()));
    regs.push((r9().to_real_reg(), "%r9".into()));
    regs.push((r10().to_real_reg(), "%r10".into()));
    regs.push((r11().to_real_reg(), "%r11".into()));

    allocable_by_class[RegClass::I64.rc_to_usize()] = Some(RegClassInfo {
        first: base,
        last: regs.len() - 1,
        suggested_scratch: Some(r12().get_index()),
    });

    // XMM registers
    base = regs.len();
    regs.push((xmm0().to_real_reg(), "%xmm0".into()));
    regs.push((xmm1().to_real_reg(), "%xmm1".into()));
    regs.push((xmm2().to_real_reg(), "%xmm2".into()));
    regs.push((xmm3().to_real_reg(), "%xmm3".into()));
    regs.push((xmm4().to_real_reg(), "%xmm4".into()));
    regs.push((xmm5().to_real_reg(), "%xmm5".into()));
    regs.push((xmm6().to_real_reg(), "%xmm6".into()));
    regs.push((xmm7().to_real_reg(), "%xmm7".into()));
    regs.push((xmm8().to_real_reg(), "%xmm8".into()));
    regs.push((xmm9().to_real_reg(), "%xmm9".into()));
    regs.push((xmm10().to_real_reg(), "%xmm10".into()));
    regs.push((xmm11().to_real_reg(), "%xmm11".into()));
    regs.push((xmm12().to_real_reg(), "%xmm12".into()));
    regs.push((xmm13().to_real_reg(), "%xmm13".into()));
    regs.push((xmm14().to_real_reg(), "%xmm14".into()));
    regs.push((xmm15().to_real_reg(), "%xmm15".into()));

    allocable_by_class[RegClass::V128.rc_to_usize()] = Some(RegClassInfo {
        first: base,
        last: regs.len() - 1,
        suggested_scratch: Some(xmm15().get_index()),
    });

    // Other regs, not available to the allocator.
    let allocable = regs.len();
    regs.push((rsp().to_real_reg(), "%rsp".into()));
    regs.push((rbp().to_real_reg(), "%rbp".into()));

    RealRegUniverse {
        regs,
        allocable,
        allocable_by_class,
    }
}

/// If `ireg` denotes an I64-classed reg, make a best-effort attempt to show its name at some
/// smaller size (4, 2 or 1 bytes).
pub fn show_ireg_sized(reg: Reg, mb_rru: Option<&RealRegUniverse>, size: u8) -> String {
    let mut s = reg.show_rru(mb_rru);

    if reg.get_class() != RegClass::I64 || size == 8 {
        // We can't do any better.
        return s;
    }

    if reg.is_real() {
        // Change (eg) "rax" into "eax", "ax" or "al" as appropriate.  This is something one could
        // describe diplomatically as "a kludge", but it's only debug code.
        let remapper = match s.as_str() {
            "%rax" => Some(["%eax", "%ax", "%al"]),
            "%rbx" => Some(["%ebx", "%bx", "%bl"]),
            "%rcx" => Some(["%ecx", "%cx", "%cl"]),
            "%rdx" => Some(["%edx", "%dx", "%dl"]),
            "%rsi" => Some(["%esi", "%si", "%sil"]),
            "%rdi" => Some(["%edi", "%di", "%dil"]),
            "%rbp" => Some(["%ebp", "%bp", "%bpl"]),
            "%rsp" => Some(["%esp", "%sp", "%spl"]),
            "%r8" => Some(["%r8d", "%r8w", "%r8b"]),
            "%r9" => Some(["%r9d", "%r9w", "%r9b"]),
            "%r10" => Some(["%r10d", "%r10w", "%r10b"]),
            "%r11" => Some(["%r11d", "%r11w", "%r11b"]),
            "%r12" => Some(["%r12d", "%r12w", "%r12b"]),
            "%r13" => Some(["%r13d", "%r13w", "%r13b"]),
            "%r14" => Some(["%r14d", "%r14w", "%r14b"]),
            "%r15" => Some(["%r15d", "%r15w", "%r15b"]),
            _ => None,
        };
        if let Some(smaller_names) = remapper {
            match size {
                4 => s = smaller_names[0].into(),
                2 => s = smaller_names[1].into(),
                1 => s = smaller_names[2].into(),
                _ => panic!("show_ireg_sized: real"),
            }
        }
    } else {
        // Add a "l", "w" or "b" suffix to RegClass::I64 vregs used at narrower widths.
        let suffix = match size {
            4 => "l",
            2 => "w",
            1 => "b",
            _ => panic!("show_ireg_sized: virtual"),
        };
        s = s + suffix;
    }

    s
}
