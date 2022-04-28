//! AArch64 ISA definitions: registers.

use crate::settings;

use crate::isa;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use regalloc::RealReg;
use regalloc::{RealRegUniverse, Reg, RegClass, RegClassInfo, Writable, NUM_REG_CLASSES};

// first argument of function call
pub fn a0() -> Reg {
    x_reg(10)
}

// second argument of function call
pub fn a1() -> Reg {
    x_reg(11)
}

// third argument of function call
pub fn a2() -> Reg {
    x_reg(12)
}

pub fn a7() -> Reg {
    x_reg(17)
}

pub fn a0_t0_a7() -> Vec<Writable<Reg>> {
    let mut v = vec![];
    for (enc, index) in
        (a0().get_hw_encoding()..=a7().get_hw_encoding()).zip(a0().get_index()..=a7().get_index())
    {
        v.push(Writable::from_reg(Reg::new_real(
            RegClass::I64,
            enc,
            index as u8,
        )));
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

pub fn s2() -> Reg {
    x_reg(18)
}

pub fn s11() -> Reg {
    x_reg(27)
}

pub fn s2_to_s11() -> Vec<Writable<Reg>> {
    let mut v = vec![];
    for (enc, index) in
        (s2().get_hw_encoding()..=s11().get_hw_encoding()).zip(s2().get_index()..=s11().get_index())
    {
        v.push(Writable::from_reg(Reg::new_real(
            RegClass::I64,
            enc,
            index as u8,
        )));
    }
    v
}

pub fn fa0() -> Reg {
    f_reg(10)
}
pub fn fa7() -> Reg {
    f_reg(17)
}

pub fn fa0_to_fa7() -> Vec<Writable<Reg>> {
    let mut v = vec![];
    for (enc, index) in (fa0().get_hw_encoding()..=fa7().get_hw_encoding())
        .zip(fa0().get_index()..=fa7().get_index())
    {
        v.push(Writable::from_reg(Reg::new_real(
            RegClass::F64,
            enc,
            index as u8,
        )));
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
    x_reg(2)
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
    x_reg(8)
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
    x_reg(31)
}

/// Get a writable reference to the spilltmp reg.
pub fn writable_spilltmp_reg() -> Writable<Reg> {
    Writable::from_reg(spilltmp_reg())
}

/// Create the register universe for AArch64.
pub fn create_reg_universe(_flags: &settings::Flags) -> RealRegUniverse {
    let mut regs: Vec<(RealReg, String)> = vec![];
    let mut allocable_by_class = [None; NUM_REG_CLASSES];

    fn new_real_reg(class: RegClass, hardware_enconding: usize, index: usize) -> RealReg {
        Reg::new_real(class, hardware_enconding as u8, index as u8)
            .as_real_reg()
            .unwrap()
    }

    // x5 t0 Temporary/alternate link register
    regs.push((new_real_reg(RegClass::I64, 5, regs.len()), "x5".into()));
    // x6–7 t1-2 Temporaries
    regs.push((new_real_reg(RegClass::I64, 6, regs.len()), "t1".into()));
    regs.push((new_real_reg(RegClass::I64, 7, regs.len()), "t2".into()));

    //x9 s1 Saved register
    regs.push((new_real_reg(RegClass::I64, 9, regs.len()), "s1".into()));

    // x18–27 s2–11 Saved registers
    for i in 18..=27 {
        regs.push((
            new_real_reg(RegClass::I64, i, regs.len()),
            format!("s{}", i - 16),
        ));
    }
    // x28–30 t3–5 Temporaries  x31 is for compiler it self
    for i in 28..=30 {
        regs.push((
            new_real_reg(RegClass::I64, i, regs.len()),
            format!("t{}", i - 25),
        ));
    }
    // x10–11 a0–1 Function arguments/return values
    regs.push((new_real_reg(RegClass::I64, 10, regs.len()), "a0".into()));
    regs.push((new_real_reg(RegClass::I64, 11, regs.len()), "a1".into()));
    // x12–17 a2–7 Function arguments Caller
    for i in 12..=17 {
        regs.push((
            new_real_reg(RegClass::I64, i, regs.len()),
            format!("a{}", i - 10),
        ));
    }

    // this all interger temp
    allocable_by_class[RegClass::I32 as usize] = Some(RegClassInfo {
        first: 0,
        last: regs.len() - 1,
        suggested_scratch: None,
    });
    allocable_by_class[RegClass::I64 as usize] = Some(RegClassInfo {
        first: 0,
        last: regs.len() - 1,
        suggested_scratch: None,
    });

    // f0–7 ft0–7 FP temporaries
    let float_start = regs.len();
    for i in 0..=7 {
        regs.push((
            new_real_reg(RegClass::F64, i, regs.len()),
            format!("ft{}", i),
        ));
    }

    // f8–9 fs0–1 FP saved registers
    for i in 8..=9 {
        regs.push((
            new_real_reg(RegClass::F64, i, regs.len()),
            format!("fs{}", i - 8),
        ));
    }

    // f18–27 fs2–11 FP saved registers
    for i in 18..=27 {
        regs.push((
            new_real_reg(RegClass::F64, i, regs.len()),
            format!("fs{}", i - 16),
        ));
    }

    // f28–31 ft8–11 FP temporaries
    for i in 28..=31 {
        regs.push((
            new_real_reg(RegClass::F64, i, regs.len()),
            format!("ft{}", i - 20),
        ));
    }

    // f10–11 fa0–1 FP arguments/return values
    regs.push((new_real_reg(RegClass::F64, 10, regs.len()), "fa0".into()));
    regs.push((new_real_reg(RegClass::F64, 11, regs.len()), "fa1".into()));

    // f12–17 fa2–7 FP arguments
    for i in 12..=17 {
        regs.push((
            new_real_reg(RegClass::F64, i, regs.len()),
            format!("fa{}", i - 10),
        ));
    }

    allocable_by_class[RegClass::F32 as usize] = Some(RegClassInfo {
        first: float_start,
        last: regs.len() - 1,
        suggested_scratch: None,
    });
    allocable_by_class[RegClass::F64 as usize] = Some(RegClassInfo {
        first: float_start,
        last: regs.len() - 1,
        suggested_scratch: None,
    });
    let allocable = regs.len();

    // x0 zero Hard-wired zero
    regs.push((new_real_reg(RegClass::I64, 0, regs.len()), "zero".into()));
    // x1 ra Return address
    regs.push((new_real_reg(RegClass::I64, 1, regs.len()), "ra".into()));
    //x8 s0/fp Saved register/frame pointer Callee
    regs.push((new_real_reg(RegClass::I64, 8, regs.len()), "fp".into()));
    // x2 sp Stack pointer
    regs.push((new_real_reg(RegClass::I64, 2, regs.len()), "sp".into()));
    // x3 gp Global pointer
    regs.push((new_real_reg(RegClass::I64, 3, regs.len()), "gp".into()));
    // x4 tp Thread pointer
    regs.push((new_real_reg(RegClass::I64, 4, regs.len()), "tp".into()));

    // x31 for compiler it self
    regs.push((new_real_reg(RegClass::I64, 31, regs.len()), "x31".into()));

    // Assert sanity: the indices in the register structs must match their
    // actual indices in the array.
    for (i, reg) in regs.iter().enumerate() {
        assert_eq!(i, reg.0.get_index());
    }
    RealRegUniverse {
        regs,
        allocable,
        allocable_by_class,
    }
}

fn x_reg(enc: u8) -> Reg {
    Reg::new_real(RegClass::I64, enc, X_INDEX[enc as usize] as u8)
}
fn f_reg(enc: u8) -> Reg {
    Reg::new_real(RegClass::I64, enc, F_INDEX[enc as usize] as u8)
}

/*
    genarated by generate_index
    don't edit
*/
#[rustfmt::skip]
static X_INDEX : &[usize]= &[
         57,     58,     60,     61,
         62,     0,      1,      2,
         59,     3,      17,     18,
         19,     20,     21,     22,
         23,     24,     4,      5,
         6,      7,      8,      9,
         10,     11,     12,     13,
         14,     15,     16,     63,
];
#[rustfmt::skip]
static F_INDEX : &[usize]= &[
         25,     26,     27,     28,
         29,     30,     31,     32,
         33,     34,     49,     50,
         51,     52,     53,     54,
         55,     56,     35,     36,
         37,     38,     39,     40,
         41,     42,     43,     44,
         45,     46,     47,     48,
];

struct RegInRealRegUniverse {
    class: RegClass,
    enc: u8,
    index: u8,
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    #[test]
    fn regs_must_be_fine() {
        use super::*;
        let b = settings::builder();
        let flag = settings::Flags::new(b);
        let x = create_reg_universe(&flag);
        use std::collections::HashSet;
        let mut names = HashSet::new();
        // check if someone is missing.
        let mut x_present = [false; 32];
        let mut f_present = [false; 32];
        // at lease have one function wrong.
        let mut has_wrong_function = None;
        for (rel, name) in x.regs {
            if rel.get_class() == RegClass::I64 {
                x_present[rel.get_hw_encoding()] = true;
            } else {
                f_present[rel.get_hw_encoding()] = true;
            }
            // check name duplicate
            if names.contains(&name) {
                panic!("name {} duplicated", name);
            }
            names.insert(name.clone());
            let mut name_to_functio_map: HashMap<&str, fn() -> Reg> = HashMap::default();

            //todo::all
            name_to_functio_map.insert("sp", stack_reg);
            name_to_functio_map.insert("fp", fp_reg);

            // at least has on wrong funciton
            if let Some(f) = name_to_functio_map.get(name.as_str()) {
                let reg = f();
                if (reg.get_index() != rel.get_index())
                    || (reg.get_class() != rel.get_class())
                    || (reg.get_hw_encoding() as usize != rel.get_hw_encoding())
                {
                    println!(
                        "'{}' should be:  Reg::new_real(RegClass::I64, {}, {})",
                        name,
                        rel.get_hw_encoding(),
                        rel.get_index()
                    );
                    has_wrong_function = Some(name.clone())
                }
            }
        }
        for (index, present) in x_present.into_iter().enumerate() {
            assert!(present, "x{} is not present\n", index);
        }
        for (index, present) in f_present.into_iter().enumerate() {
            assert!(present, "f{} is not present\n", index);
        }
        assert!(
            has_wrong_function.is_none(),
            "function '{}' has wrong implementation.",
            has_wrong_function.unwrap()
        );
    }

    #[test]
    fn generate_index() {
        use super::*;
        let b = settings::builder();
        let flag = settings::Flags::new(b);
        let x = create_reg_universe(&flag);

        let mut x_index = [0; 32];
        let mut f_index = [0; 32];
        x.regs.iter().for_each(|(rel, _)| {
            if rel.get_class() == RegClass::I64 {
                x_index[rel.get_hw_encoding()] = rel.get_index();
            } else {
                f_index[rel.get_hw_encoding()] = rel.get_index();
            }
        });

        println!("#[rustfmt::skip]");
        println!("static X_INDEX : &[usize]= &[");
        for (i, index) in x_index.iter().enumerate() {
            if i != 0 && i % 4 == 0 {
                println!()
            }
            print!("\t {},", index);
        }
        println!("\n];");

        println!("#[rustfmt::skip]");
        println!("static F_INDEX : &[usize]= &[");
        for (i, index) in f_index.iter().enumerate() {
            if i != 0 && i % 4 == 0 {
                println!()
            }
            print!("\t {},", index);
        }
        println!("\n];");
    }
}
