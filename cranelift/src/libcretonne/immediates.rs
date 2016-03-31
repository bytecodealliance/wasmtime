
//! Immediate operands for Cretonne instructions
//!
//! This module defines the types of immediate operands that can appear on Cretonne instructions.
//! Each type here should have a corresponding definition in the `cretonne.immediates` Python
//! module in the meta language.

use std::fmt::{self, Display, Formatter};
use std::mem;

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


/// An IEEE binary32 immediate floating point value.
///
/// All bit patterns are allowed.
pub struct Ieee32(f32);

/// An IEEE binary64 immediate floating point value.
///
/// All bit patterns are allowed.
pub struct Ieee64(f64);

// Format a floating point number in a way that is reasonably human-readable, and that can be
// converted back to binary without any rounding issues. The hexadecimal formatting of normal and
// subnormal numbers is compatible with C99 and the printf "%a" format specifier. The NaN and Inf
// formats are not supported by C99.
//
// The encoding parameters are:
//
// w - exponent field width in bits
// t - trailing significand field width in bits
//
fn format_float(bits: u64, w: u8, t: u8, f: &mut Formatter) -> fmt::Result {
    assert!(w > 0 && w <= 16, "Invalid exponent range");
    assert!(1 + w + t <= 64, "Too large IEEE format for u64");

    let max_e_bits = (1u64 << w) - 1;
    let t_bits = bits & ((1u64 << t) - 1); // Trailing significand.
    let e_bits = (bits >> t) & max_e_bits; // Biased exponent.
    let sign_bit = (bits >> w + t) & 1;

    let bias: i32 = (1 << (w - 1)) - 1;
    let e = e_bits as i32 - bias; // Unbiased exponent.
    let emin = 1 - bias; // Minimum exponent.

    // How many hexadecimal digits are needed for the trailing significand?
    let digits = (t + 3) / 4;
    // Trailing significand left-aligned in `digits` hexadecimal digits.
    let left_t_bits = t_bits << (4 * digits - t);

    // All formats share the leading sign.
    if sign_bit != 0 {
        try!(write!(f, "-"));
    }

    if e_bits == 0 {
        if t_bits == 0 {
            // Zero.
            write!(f, "0.0")
        } else {
            // Subnormal.
            write!(f, "0x0.{0:01$x}p{2}", left_t_bits, digits as usize, emin)
        }
    } else if e_bits == max_e_bits {
        if t_bits == 0 {
            // Infinity.
            write!(f, "Inf")
        } else {
            // NaN.
            let payload = t_bits & ((1 << (t - 1)) - 1);
            if t_bits & (1 << (t - 1)) != 0 {
                // Quiet NaN.
                if payload != 0 {
                    write!(f, "qNaN:0x{:x}", payload)
                } else {
                    write!(f, "qNaN")
                }
            } else {
                // Signaling NaN.
                write!(f, "sNaN:0x{:x}", payload)
            }
        }
    } else {
        // Normal number.
        write!(f, "0x1.{0:01$x}p{2}", left_t_bits, digits as usize, e)
    }
}

impl Ieee32 {
    pub fn new(x: f32) -> Ieee32 {
        Ieee32(x)
    }

    /// Construct Ieee32 immediate from raw bits.
    pub fn new_from_bits(x: u32) -> Ieee32 {
        Ieee32(unsafe { mem::transmute(x) })
    }
}

impl Display for Ieee32 {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let bits: u32 = unsafe { mem::transmute(self.0) };
        format_float(bits as u64, 8, 23, f)
    }
}

impl Ieee64 {
    pub fn new(x: f64) -> Ieee64 {
        Ieee64(x)
    }

    /// Construct Ieee64 immediate from raw bits.
    pub fn new_from_bits(x: u64) -> Ieee64 {
        Ieee64(unsafe { mem::transmute(x) })
    }
}

impl Display for Ieee64 {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let bits: u64 = unsafe { mem::transmute(self.0) };
        format_float(bits, 11, 52, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{f32, f64};

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

    #[test]
    fn format_ieee32() {
        assert_eq!(format!("{}", Ieee32::new(0.0)), "0.0");
        assert_eq!(format!("{}", Ieee32::new(-0.0)), "-0.0");
        assert_eq!(format!("{}", Ieee32::new(1.0)), "0x1.000000p0");
        assert_eq!(format!("{}", Ieee32::new(1.5)), "0x1.800000p0");
        assert_eq!(format!("{}", Ieee32::new(0.5)), "0x1.000000p-1");
        assert_eq!(format!("{}", Ieee32::new(f32::EPSILON)), "0x1.000000p-23");
        assert_eq!(format!("{}", Ieee32::new(f32::MIN)), "-0x1.fffffep127");
        assert_eq!(format!("{}", Ieee32::new(f32::MAX)), "0x1.fffffep127");
        // Smallest positive normal number.
        assert_eq!(format!("{}", Ieee32::new(f32::MIN_POSITIVE)),
                   "0x1.000000p-126");
        // Subnormals.
        assert_eq!(format!("{}", Ieee32::new(f32::MIN_POSITIVE / 2.0)),
                   "0x0.800000p-126");
        assert_eq!(format!("{}", Ieee32::new(f32::MIN_POSITIVE * f32::EPSILON)),
                   "0x0.000002p-126");
        assert_eq!(format!("{}", Ieee32::new(f32::INFINITY)), "Inf");
        assert_eq!(format!("{}", Ieee32::new(f32::NEG_INFINITY)), "-Inf");
        assert_eq!(format!("{}", Ieee32::new(f32::NAN)), "qNaN");
        assert_eq!(format!("{}", Ieee32::new(-f32::NAN)), "-qNaN");
        // Construct some qNaNs with payloads.
        assert_eq!(format!("{}", Ieee32::new_from_bits(0x7fc00001)), "qNaN:0x1");
        assert_eq!(format!("{}", Ieee32::new_from_bits(0x7ff00001)),
                   "qNaN:0x300001");
        // Signaling NaNs.
        assert_eq!(format!("{}", Ieee32::new_from_bits(0x7f800001)), "sNaN:0x1");
        assert_eq!(format!("{}", Ieee32::new_from_bits(0x7fa00001)),
                   "sNaN:0x200001");
    }

    #[test]
    fn format_ieee64() {
        assert_eq!(format!("{}", Ieee64::new(0.0)), "0.0");
        assert_eq!(format!("{}", Ieee64::new(-0.0)), "-0.0");
        assert_eq!(format!("{}", Ieee64::new(1.0)), "0x1.0000000000000p0");
        assert_eq!(format!("{}", Ieee64::new(1.5)), "0x1.8000000000000p0");
        assert_eq!(format!("{}", Ieee64::new(0.5)), "0x1.0000000000000p-1");
        assert_eq!(format!("{}", Ieee64::new(f64::EPSILON)),
                   "0x1.0000000000000p-52");
        assert_eq!(format!("{}", Ieee64::new(f64::MIN)),
                   "-0x1.fffffffffffffp1023");
        assert_eq!(format!("{}", Ieee64::new(f64::MAX)),
                   "0x1.fffffffffffffp1023");
        // Smallest positive normal number.
        assert_eq!(format!("{}", Ieee64::new(f64::MIN_POSITIVE)),
                   "0x1.0000000000000p-1022");
        // Subnormals.
        assert_eq!(format!("{}", Ieee64::new(f64::MIN_POSITIVE / 2.0)),
                   "0x0.8000000000000p-1022");
        assert_eq!(format!("{}", Ieee64::new(f64::MIN_POSITIVE * f64::EPSILON)),
                   "0x0.0000000000001p-1022");
        assert_eq!(format!("{}", Ieee64::new(f64::INFINITY)), "Inf");
        assert_eq!(format!("{}", Ieee64::new(f64::NEG_INFINITY)), "-Inf");
        assert_eq!(format!("{}", Ieee64::new(f64::NAN)), "qNaN");
        assert_eq!(format!("{}", Ieee64::new(-f64::NAN)), "-qNaN");
        // Construct some qNaNs with payloads.
        assert_eq!(format!("{}", Ieee64::new_from_bits(0x7ff8000000000001)),
                   "qNaN:0x1");
        assert_eq!(format!("{}", Ieee64::new_from_bits(0x7ffc000000000001)),
                   "qNaN:0x4000000000001");
        // Signaling NaNs.
        assert_eq!(format!("{}", Ieee64::new_from_bits(0x7ff0000000000001)),
                   "sNaN:0x1");
        assert_eq!(format!("{}", Ieee64::new_from_bits(0x7ff4000000000001)),
                   "sNaN:0x4000000000001");
    }
}
