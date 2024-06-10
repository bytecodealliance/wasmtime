//! Riscv64 ISA definitions: registers.
//!

use crate::machinst::{Reg, Writable};

use alloc::vec;
use alloc::vec::Vec;

use regalloc2::{PReg, RegClass, VReg};

// first argument of function call
#[inline]
pub fn a0() -> Reg {
    x_reg(10)
}

// second argument of function call
#[inline]
#[allow(dead_code)]
pub fn a1() -> Reg {
    x_reg(11)
}

// third argument of function call
#[inline]
#[allow(dead_code)]
pub fn a2() -> Reg {
    x_reg(12)
}

#[inline]
#[allow(dead_code)]
pub fn writable_a0() -> Writable<Reg> {
    Writable::from_reg(a0())
}
#[inline]
#[allow(dead_code)]
pub fn writable_a1() -> Writable<Reg> {
    Writable::from_reg(a1())
}
#[inline]
#[allow(dead_code)]
pub fn writable_a2() -> Writable<Reg> {
    Writable::from_reg(a2())
}

#[inline]
#[allow(dead_code)]
pub fn fa0() -> Reg {
    f_reg(10)
}
#[inline]
#[allow(dead_code)]
pub fn writable_fa0() -> Writable<Reg> {
    Writable::from_reg(fa0())
}
#[inline]
#[allow(dead_code)]
pub fn writable_fa1() -> Writable<Reg> {
    Writable::from_reg(fa1())
}
#[inline]
pub fn fa1() -> Reg {
    f_reg(11)
}

/// Get a reference to the zero-register.
#[inline]
pub fn zero_reg() -> Reg {
    x_reg(0)
}

/// Get a writable reference to the zero-register (this discards a result).
#[inline]
pub fn writable_zero_reg() -> Writable<Reg> {
    Writable::from_reg(zero_reg())
}
#[inline]
pub fn stack_reg() -> Reg {
    x_reg(2)
}

/// Get a writable reference to the stack-pointer register.
#[inline]
pub fn writable_stack_reg() -> Writable<Reg> {
    Writable::from_reg(stack_reg())
}

/// Get a reference to the link register (x1).
pub fn link_reg() -> Reg {
    x_reg(1)
}

/// Get a writable reference to the link register.
#[inline]
pub fn writable_link_reg() -> Writable<Reg> {
    Writable::from_reg(link_reg())
}

/// Get a reference to the frame pointer (x8).
#[inline]
pub fn fp_reg() -> Reg {
    x_reg(8)
}

/// Get a writable reference to the frame pointer.
#[inline]
pub fn writable_fp_reg() -> Writable<Reg> {
    Writable::from_reg(fp_reg())
}

/// Get a reference to the first temporary, sometimes "spill temporary",
/// register. This register is used in various ways as a temporary.
#[inline]
pub fn spilltmp_reg() -> Reg {
    x_reg(31)
}

/// Get a writable reference to the spilltmp reg.
#[inline]
pub fn writable_spilltmp_reg() -> Writable<Reg> {
    Writable::from_reg(spilltmp_reg())
}

///spilltmp2
#[inline]
pub fn spilltmp_reg2() -> Reg {
    x_reg(30)
}

/// Get a writable reference to the spilltmp2 reg.
#[inline]
pub fn writable_spilltmp_reg2() -> Writable<Reg> {
    Writable::from_reg(spilltmp_reg2())
}

#[inline]
pub fn x_reg(enc: usize) -> Reg {
    let p_reg = PReg::new(enc, RegClass::Int);
    let v_reg = VReg::new(p_reg.index(), p_reg.class());
    Reg::from(v_reg)
}
pub const fn px_reg(enc: usize) -> PReg {
    PReg::new(enc, RegClass::Int)
}

#[inline]
pub fn f_reg(enc: usize) -> Reg {
    let p_reg = PReg::new(enc, RegClass::Float);
    let v_reg = VReg::new(p_reg.index(), p_reg.class());
    Reg::from(v_reg)
}
pub const fn pf_reg(enc: usize) -> PReg {
    PReg::new(enc, RegClass::Float)
}

#[allow(dead_code)]
pub(crate) fn x_reg_range(start: usize, end: usize) -> Vec<Writable<Reg>> {
    let mut regs = vec![];
    for i in start..=end {
        regs.push(Writable::from_reg(x_reg(i)));
    }
    regs
}

pub const fn pv_reg(enc: usize) -> PReg {
    PReg::new(enc, RegClass::Vector)
}
