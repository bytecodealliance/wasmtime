//! ARM32 ISA definitions: registers.

use crate::machinst::Reg;

use alloc::string::String;

pub fn pretty_print_reg(reg: Reg) -> String {
    match reg.to_real_reg() {
        Some(real) => match real.hw_enc() {
            0 => "r0".into(),
            1 => "r1".into(),
            2 => "r2".into(),
            3 => "r3".into(),
            4 => "r4".into(),
            5 => "r5".into(),
            6 => "r6".into(),
            7 => "r7".into(),
            8 => "r8".into(),
            9 => "r9".into(),
            10 => "r10".into(),
            11 => "r11".into(),
            12 => "r12".into(),
            13 => "sp".into(),
            14 => "lr".into(),
            15 => "pc".into(),
            _ => unreachable!(),
        },
        None => {
            format!("{reg:?}")
        }
    }
}
