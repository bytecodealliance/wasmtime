//! AArch64 ISA definitions: registers.

use core::fmt::Write;

use crate::settings;

use crate::isa;
use crate::machinst::{Reg, Writable};

use crate::machinst::RealReg;
use alloc::vec;
use alloc::vec::Vec;

use regalloc2::VReg;
use regalloc2::{MachineEnv, PReg, RegClass};

// first argument of function call
pub fn a0() -> Reg {
    x_reg(18)
}

// second argument of function call
pub fn a1() -> Reg {
    x_reg(19)
}

// third argument of function call
pub fn a2() -> Reg {
    x_reg(20)
}

pub fn a7() -> Reg {
    x_reg(25)
}
pub fn tp() -> Reg {
    x_reg(15)
}

pub fn a0_t0_a7() -> Vec<Writable<Reg>> {
    let mut v = vec![];
    for enc in a0().to_real_reg().unwrap().hw_enc()..=a7().to_real_reg().unwrap().hw_enc() {
        v.push(Writable::from_reg(x_reg(enc as usize)));
    }
    v
}

pub fn wirteable_a0() -> Writable<Reg> {
    Writable::from_reg(a0())
}
pub fn wirteable_a1() -> Writable<Reg> {
    Writable::from_reg(a1())
}

pub fn wirteable_a2() -> Writable<Reg> {
    Writable::from_reg(a2())
}

pub fn stacklimit_reg() -> Reg {
    spilltmp_reg()
}

pub fn s1() -> Reg {
    x_reg(3)
}

pub fn s11() -> Reg {
    x_reg(13)
}

pub fn s1_to_s11() -> Vec<Writable<Reg>> {
    let mut v = vec![];
    for enc in s1().to_real_reg().unwrap().hw_enc()..=s11().to_real_reg().unwrap().hw_enc() {
        v.push(Writable::from_reg(x_reg(enc as usize)));
    }
    v
}

pub fn fa0() -> Reg {
    f_reg(18)
}
pub fn fa7() -> Reg {
    f_reg(25)
}

pub fn fa0_to_fa7() -> Vec<Writable<Reg>> {
    let mut v = vec![];
    for enc in fa0().to_real_reg().unwrap().hw_enc()..=fa7().to_real_reg().unwrap().hw_enc() {
        v.push(Writable::from_reg(f_reg(enc as usize)));
    }
    v
}

/// Get a reference to the zero-register.
pub fn zero_reg() -> Reg {
    x_reg(0)
}

pub fn get_caller_save_register(_call_conv_of_callee: isa::CallConv) -> Vec<Writable<Reg>> {
    unimplemented!();
}
/// Get a writable reference to the zero-register (this discards a result).
pub fn writable_zero_reg() -> Writable<Reg> {
    Writable::from_reg(zero_reg())
}

pub fn stack_reg() -> Reg {
    x_reg(14)
}

/// Get a writable reference to the stack-pointer register.
pub fn writable_stack_reg() -> Writable<Reg> {
    Writable::from_reg(stack_reg())
}

/// Get a reference to the link register (x30).
pub fn link_reg() -> Reg {
    x_reg(1)
}

/// Get a writable reference to the link register.
pub fn writable_link_reg() -> Writable<Reg> {
    Writable::from_reg(link_reg())
}

/// Get a reference to the frame pointer (x29).
pub fn fp_reg() -> Reg {
    x_reg(2)
}

/// Get a writable reference to the frame pointer.
pub fn writable_fp_reg() -> Writable<Reg> {
    Writable::from_reg(fp_reg())
}

/// Get a reference to the first temporary, sometimes "spill temporary", register. This register is
/// used to compute the address of a spill slot when a direct offset addressing mode from FP is not
/// sufficient (+/- 2^11 words). We exclude this register from regalloc and reserve it for this
/// purpose for simplicity; otherwise we need a multi-stage analysis where we first determine how
/// many spill slots we have, then perhaps remove the reg from the pool and recompute regalloc.
///
/// We use x16 for this (aka IP0 in the AArch64 ABI) because it's a scratch register but is
/// slightly special (used for linker veneers). We're free to use it as long as we don't expect it
/// to live through call instructions.
pub fn spilltmp_reg() -> Reg {
    x_reg(12)
}

/// Get a writable reference to the spilltmp reg.
pub fn writable_spilltmp_reg() -> Writable<Reg> {
    Writable::from_reg(spilltmp_reg())
}

/*

    x_register 12 and 13 it's for compiler it self.
    f register 15 for compiler is self.
*/
pub fn crate_reg_eviroment(_flags: &settings::Flags) -> MachineEnv {
    let preferred_regs_by_class: [Vec<PReg>; 2] = {
        let mut x_register: Vec<PReg> = vec![];
        for i in 16..=30 {
            x_register.push(PReg::new(i, RegClass::Int));
        }
        let mut f_register: Vec<PReg> = vec![];
        for i in 16..=31 {
            f_register.push(PReg::new(i, RegClass::Float));
        }
        [x_register, f_register]
    };

    let non_preferred_regs_by_class: [Vec<PReg>; 2] = {
        let mut x_register: Vec<PReg> = vec![];
        /*
            origin  s1-s11
                    x3-x13
        */
        for i in 3..=11 {
            x_register.push(PReg::new(i, RegClass::Int));
        }
        let mut f_register: Vec<PReg> = vec![];
        for i in 0..=14 {
            f_register.push(PReg::new(i, RegClass::Float));
        }
        [x_register, f_register]
    };

    let scratch_by_class: [PReg; 2] =
        [PReg::new(13, RegClass::Int), PReg::new(15, RegClass::Float)];
    let fixed_stack_slots: Vec<PReg> = vec![];

    MachineEnv {
        preferred_regs_by_class,
        non_preferred_regs_by_class,
        scratch_by_class,
        fixed_stack_slots,
    }
}

fn x_reg(enc: usize) -> Reg {
    let p_reg = PReg::new(enc, RegClass::Int);
    let v_reg = VReg::new(p_reg.index(), p_reg.class());
    Reg::from(v_reg)
}

fn f_reg(enc: usize) -> Reg {
    let p_reg = PReg::new(enc, RegClass::Float);
    let v_reg = VReg::new(p_reg.index(), p_reg.class());
    Reg::from(v_reg)
}

pub(crate) fn real_reg_to_reg(x: RealReg) -> Reg {
    let v_reg = VReg::new(x.hw_enc() as usize, x.class());
    Reg::from(v_reg)
}

#[cfg(test)]
mod test {

    // #[test]
    // fn regs_must_be_fine() {
    //     use super::*;
    //     let b = settings::builder();
    //     let flag = settings::Flags::new(b);
    //     let x = create_reg_universe(&flag);
    //     use std::collections::HashSet;
    //     let mut names = HashSet::new();
    //     // check if someone is missing.
    //     let mut x_present = [false; 32];
    //     let mut f_present = [false; 32];
    //     // at lease have one function wrong.
    //     let mut has_wrong_function = None;
    //     for (rel, name) in x.regs {
    //         if rel.class() == RegClass::Int {
    //             x_present[rel.get_hw_encoding()] = true;
    //         } else {
    //             f_present[rel.get_hw_encoding()] = true;
    //         }
    //         // check name duplicate
    //         if names.contains(&name) {
    //             panic!("name {} duplicated", name);
    //         }
    //         names.insert(name.clone());
    //         let mut name_to_functio_map: HashMap<&str, fn() -> Reg> = HashMap::default();

    //         //todo::all
    //         name_to_functio_map.insert("sp", stack_reg);
    //         name_to_functio_map.insert("fp", fp_reg);

    //         // at least has on wrong funciton
    //         if let Some(f) = name_to_functio_map.get(name.as_str()) {
    //             let reg = f();
    //             if (reg.get_index() != rel.get_index())
    //                 || (reg.class() != rel.class())
    //                 || (reg.get_hw_encoding() as usize != rel.get_hw_encoding())
    //             {
    //                 println!(
    //                     "'{}' should be:  Reg::new_real(RegClass::Int, {}, {})",
    //                     name,
    //                     rel.get_hw_encoding(),
    //                     rel.get_index()
    //                 );
    //                 has_wrong_function = Some(name.clone())
    //             }
    //         }
    //     }
    //     for (index, present) in x_present.into_iter().enumerate() {
    //         assert!(present, "x{} is not present\n", index);
    //     }
    //     for (index, present) in f_present.into_iter().enumerate() {
    //         assert!(present, "f{} is not present\n", index);
    //     }
    //     assert!(
    //         has_wrong_function.is_none(),
    //         "function '{}' has wrong implementation.",
    //         has_wrong_function.unwrap()
    //     );
    // }

    // #[test]
    // fn generate_index() {
    //     use super::*;
    //     let b = settings::builder();
    //     let flag = settings::Flags::new(b);
    //     let x = crate_reg_eviroment(&flag);

    //     let mut x_index = [0; 32];
    //     let mut f_index = [0; 32];
    //     x.regs.iter().for_each(|(rel, _)| {
    //         if rel.class() == RegClass::Int {
    //             x_index[rel.get_hw_encoding()] = rel.get_index();
    //         } else {
    //             f_index[rel.get_hw_encoding()] = rel.get_index();
    //         }
    //     });

    //     println!("#[rustfmt::skip]");
    //     println!("static X_INDEX : &[usize]= &[");
    //     for (i, index) in x_index.iter().enumerate() {
    //         if i != 0 && i % 4 == 0 {
    //             println!()
    //         }
    //         print!("\t {},", index);
    //     }
    //     println!("\n];");

    //     println!("#[rustfmt::skip]");
    //     println!("static F_INDEX : &[usize]= &[");
    //     for (i, index) in f_index.iter().enumerate() {
    //         if i != 0 && i % 4 == 0 {
    //             println!()
    //         }
    //         print!("\t {},", index);
    //     }
    //     println!("\n];");
    // }
}
