//! Predicate functions for testing instruction fields.
//!
//! This module defines functions that are used by the instruction predicates defined by
//! `lib/cretonne/meta/cdsl/predicates.py` classes.
//!
//! The predicates the operate on integer fields use `Into<i64>` as a shared trait bound. This
//! bound is implemented by all the native integer types as well as `Imm64`.
//!
//! Some of these predicates may be unused in certain ISA configurations, so we suppress the
//! dead_code warning.

/// Check that `x` is the same as `y`.
#[allow(dead_code)]
pub fn is_equal<T: Eq + Copy, O: Into<T> + Copy>(x: T, y: O) -> bool {
    x == y.into()
}

/// Check that `x` can be represented as a `wd`-bit signed integer with `sc` low zero bits.
#[allow(dead_code)]
pub fn is_signed_int<T: Into<i64>>(x: T, wd: u8, sc: u8) -> bool {
    let s = x.into();
    s == (s >> sc << (64 - wd + sc) >> (64 - wd))
}

/// Check that `x` can be represented as a `wd`-bit unsigned integer with `sc` low zero bits.
#[allow(dead_code)]
pub fn is_unsigned_int<T: Into<i64>>(x: T, wd: u8, sc: u8) -> bool {
    let u = x.into() as u64;
    // Bit-mask of the permitted bits.
    let m = (1 << wd) - (1 << sc);
    u == (u & m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cvt_u32() {
        let x1 = 0u32;
        let x2 = 1u32;
        let x3 = 0xffff_fff0u32;

        assert!(is_signed_int(x1, 1, 0));
        assert!(is_signed_int(x1, 2, 1));
        assert!(is_signed_int(x2, 2, 0));
        assert!(!is_signed_int(x2, 2, 1));

        // `u32` doesn't sign-extend when converted to `i64`.
        assert!(!is_signed_int(x3, 8, 0));

        assert!(is_unsigned_int(x1, 1, 0));
        assert!(is_unsigned_int(x1, 8, 4));
        assert!(is_unsigned_int(x2, 1, 0));
        assert!(!is_unsigned_int(x2, 8, 4));
        assert!(!is_unsigned_int(x3, 1, 0));
        assert!(is_unsigned_int(x3, 32, 4));
    }

    #[test]
    fn cvt_imm64() {
        use ir::immediates::Imm64;

        let x1 = Imm64::new(-8);
        let x2 = Imm64::new(8);

        assert!(is_signed_int(x1, 16, 2));
        assert!(is_signed_int(x2, 16, 2));
        assert!(!is_signed_int(x1, 16, 4));
        assert!(!is_signed_int(x2, 16, 4));
    }
}
