
//! Immediate operands for Cretonne instructions
//!
//! This module defines the types of immediate operands that can appear on Cretonne instructions.
//! Each type here should have a corresponding definition in the `cretonne.immediates` Python
//! module in the meta language.

use std::fmt::{self, Display, Formatter};

/// 64-bit immediate integer operand.
///
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Imm64(i64);

impl Display for Imm64 {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let x = self.0;
        if -10_000 < x && x < 10_000 {
            // Use decimal for small numbers.
            write!(f, "{}", x)
        } else {
            // Hexadecimal with a multiple of 4 digits and group separators:
            //
            //   0xfff0
            //   0x0001_ffff
            //   0xffff_ffff_fff8_4400
            //
            let mut pos = (64 - x.leading_zeros() - 1) & 0xf0;
            try!(write!(f, "0x{:04x}", (x >> pos) & 0xffff));
            while pos > 0 {
                pos -= 16;
                try!(write!(f, "_{:04x}", (x >> pos) & 0xffff));
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_imm64() {
        assert_eq!(format!("{}", Imm64(0)), "0");
        assert_eq!(format!("{}", Imm64(9999)), "9999");
        assert_eq!(format!("{}", Imm64(10000)), "0x2710");
        assert_eq!(format!("{}", Imm64(-9999)), "-9999");
        assert_eq!(format!("{}", Imm64(-10000)), "0xffff_ffff_ffff_d8f0");
        assert_eq!(format!("{}", Imm64(0xffff)), "0xffff");
        assert_eq!(format!("{}", Imm64(0x10000)), "0x0001_0000");
    }
}
