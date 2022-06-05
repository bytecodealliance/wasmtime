//! Riscv64 ISA definitions: registers.
//!

use crate::machinst::ArgsOrRets;
use crate::settings;

use crate::machinst::{Reg, Writable};

use crate::machinst::RealReg;
use alloc::vec;
use alloc::vec::Vec;

use regalloc2::VReg;
use regalloc2::{MachineEnv, PReg, RegClass};

// first argument of function call
#[inline(always)]
pub fn a0() -> Reg {
    x_reg(10)
}

// second argument of function call
#[inline(always)]
pub fn a1() -> Reg {
    x_reg(11)
}

// third argument of function call
#[inline(always)]
pub fn a2() -> Reg {
    x_reg(12)
}
#[inline(always)]
pub fn a7() -> Reg {
    x_reg(17)
}
#[inline(always)]
pub fn param_or_rets_xregs(args_or_rets: ArgsOrRets) -> Vec<Writable<Reg>> {
    let mut v = vec![];
    let a0 = 10;
    let a_last = if args_or_rets == ArgsOrRets::Args {
        17
    } else {
        11
    };
    for enc in a0..=a_last {
        v.push(Writable::from_reg(x_reg(enc as usize)));
    }
    v
}

#[inline(always)]
pub fn writable_a0() -> Writable<Reg> {
    Writable::from_reg(a0())
}
#[inline(always)]
pub fn writable_a1() -> Writable<Reg> {
    Writable::from_reg(a1())
}
#[inline(always)]
pub fn writable_a2() -> Writable<Reg> {
    Writable::from_reg(a2())
}
#[inline(always)]
pub fn stacklimit_reg() -> Reg {
    spilltmp_reg()
}

/*
used when more register in code emiting.
this should not include special purpose register such as fp sp etc.
*/
pub fn bunch_of_normal_registers() -> Vec<Writable<Reg>> {
    let mut v = vec![];
    /*
        s2 --------> s11
    */
    for enc in x_reg(18).to_real_reg().unwrap().hw_enc()..=x_reg(27).to_real_reg().unwrap().hw_enc()
    {
        v.push(Writable::from_reg(x_reg(enc as usize)));
    }
    v
}
#[inline(always)]
pub fn fa0() -> Reg {
    f_reg(10)
}
#[inline(always)]
pub fn writable_fa0() -> Writable<Reg> {
    Writable::from_reg(fa0())
}
#[inline(always)]
pub fn writable_fa1() -> Writable<Reg> {
    Writable::from_reg(fa1())
}
#[inline(always)]
pub fn fa1() -> Reg {
    f_reg(11)
}
// #[inline(always)]
// pub fn fa2() -> Reg {
//     f_reg(12)
// }
#[inline(always)]
pub fn fa7() -> Reg {
    f_reg(17)
}
#[inline(always)]
pub fn param_or_rets_fregs(args_or_rets: ArgsOrRets) -> Vec<Writable<Reg>> {
    let mut v = vec![];
    let fa0 = 10;
    let fa_last = if args_or_rets == ArgsOrRets::Args {
        17
    } else {
        11
    };
    for enc in fa0..=fa_last {
        v.push(Writable::from_reg(f_reg(enc as usize)));
    }

    v
}

/// Get a reference to the zero-register.
/// #[inline(always)]
pub fn zero_reg() -> Reg {
    x_reg(0)
}

/// Get a writable reference to the zero-register (this discards a result).
/// #[inline(always)]
pub fn writable_zero_reg() -> Writable<Reg> {
    Writable::from_reg(zero_reg())
}
#[inline(always)]
pub fn stack_reg() -> Reg {
    x_reg(2)
}

/// Get a writable reference to the stack-pointer register.
#[inline(always)]
pub fn writable_stack_reg() -> Writable<Reg> {
    Writable::from_reg(stack_reg())
}

/// Get a reference to the link register (x1).
pub fn link_reg() -> Reg {
    x_reg(1)
}

/// Get a writable reference to the link register.
#[inline(always)]
pub fn writable_link_reg() -> Writable<Reg> {
    Writable::from_reg(link_reg())
}

/// Get a reference to the frame pointer (x29).
#[inline(always)]
pub fn fp_reg() -> Reg {
    x_reg(8)
}

/// Get a writable reference to the frame pointer.
#[inline(always)]
pub fn writable_fp_reg() -> Writable<Reg> {
    Writable::from_reg(fp_reg())
}

/// Get a reference to the first temporary, sometimes "spill temporary", register. This register is
/// used to compute the address of a spill slot when a direct offset addressing mode from FP is not
/// sufficient (+/- 2^11 words). We exclude this register from regalloc and reserve it for this
/// purpose for simplicity; otherwise we need a multi-stage analysis where we first determine how
/// many spill slots we have, then perhaps remove the reg from the pool and recompute regalloc.
///
/// We use x16 for this (aka IP0 in the Riscv64 ABI) because it's a scratch register but is
/// slightly special (used for linker veneers). We're free to use it as long as we don't expect it
/// to live through call instructions.
#[inline(always)]
pub fn spilltmp_reg() -> Reg {
    x_reg(31)
}

/// Get a writable reference to the spilltmp reg.
/// todo how to verify when is ok to use this register.
#[inline(always)]
pub fn writable_spilltmp_reg() -> Writable<Reg> {
    Writable::from_reg(spilltmp_reg())
}

/*


*/
pub fn crate_reg_eviroment(_flags: &settings::Flags) -> MachineEnv {
    let preferred_regs_by_class: [Vec<PReg>; 2] = {
        let mut x_register: Vec<PReg> = vec![];
        x_register.push(PReg::new(5, RegClass::Int));
        for i in 6..=7 {
            x_register.push(PReg::new(i, RegClass::Int));
        }
        for i in 10..=17 {
            x_register.push(PReg::new(i, RegClass::Int));
        }
        for i in 28..=29 {
            x_register.push(PReg::new(i, RegClass::Int));
        }

        let mut f_register: Vec<PReg> = vec![];
        for i in 0..=7 {
            f_register.push(PReg::new(i, RegClass::Float));
        }
        for i in 10..=17 {
            f_register.push(PReg::new(i, RegClass::Float));
        }
        for i in 28..=30 {
            f_register.push(PReg::new(i, RegClass::Float));
        }
        [x_register, f_register]
    };

    let non_preferred_regs_by_class: [Vec<PReg>; 2] = {
        let mut x_register: Vec<PReg> = vec![];
        x_register.push(PReg::new(9, RegClass::Int));
        for i in 18..=27 {
            x_register.push(PReg::new(i, RegClass::Int));
        }
        let mut f_register: Vec<PReg> = vec![];
        for i in 8..=9 {
            f_register.push(PReg::new(i, RegClass::Float));
        }
        for i in 18..=27 {
            f_register.push(PReg::new(i, RegClass::Float));
        }
        [x_register, f_register]
    };

    let scratch_by_class: [PReg; 2] =
        [PReg::new(30, RegClass::Int), PReg::new(31, RegClass::Float)];
    let fixed_stack_slots: Vec<PReg> = vec![];

    MachineEnv {
        preferred_regs_by_class,
        non_preferred_regs_by_class,
        scratch_by_class,
        fixed_stack_slots,
    }
}

#[inline(always)]
pub fn x_reg(enc: usize) -> Reg {
    let p_reg = PReg::new(enc, RegClass::Int);
    let v_reg = VReg::new(p_reg.index(), p_reg.class());
    Reg::from(v_reg)
}
#[inline(always)]
pub fn f_reg(enc: usize) -> Reg {
    let p_reg = PReg::new(enc, RegClass::Float);
    let v_reg = VReg::new(p_reg.index(), p_reg.class());
    Reg::from(v_reg)
}

#[inline(always)]
pub(crate) fn real_reg_to_reg(x: RealReg) -> Reg {
    let v_reg = VReg::new(x.hw_enc() as usize, x.class());
    Reg::from(v_reg)
}
