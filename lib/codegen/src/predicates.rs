//! Predicate functions for testing instruction fields.
//!
//! This module defines functions that are used by the instruction predicates defined by
//! `lib/codegen/meta-python/cdsl/predicates.py` classes.
//!
//! The predicates the operate on integer fields use `Into<i64>` as a shared trait bound. This
//! bound is implemented by all the native integer types as well as `Imm64`.
//!
//! Some of these predicates may be unused in certain ISA configurations, so we suppress the
//! dead code warning.

use ir;

/// Check that a 64-bit floating point value is zero.
#[allow(dead_code)]
pub fn is_zero_64_bit_float<T: Into<ir::immediates::Ieee64>>(x: T) -> bool {
    let x64 = x.into();
    x64.bits() == 0
}

/// Check that a 32-bit floating point value is zero.
#[allow(dead_code)]
pub fn is_zero_32_bit_float<T: Into<ir::immediates::Ieee32>>(x: T) -> bool {
    let x32 = x.into();
    x32.bits() == 0
}

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

#[allow(dead_code)]
pub fn is_colocated_func(func_ref: ir::FuncRef, func: &ir::Function) -> bool {
    func.dfg.ext_funcs[func_ref].colocated
}

#[allow(dead_code)]
pub fn is_colocated_data(global_value: ir::GlobalValue, func: &ir::Function) -> bool {
    match func.global_values[global_value] {
        ir::GlobalValueData::Sym { colocated, .. } => colocated,
        _ => panic!("is_colocated_data only makes sense for data with symbolic addresses"),
    }
}

#[allow(dead_code)]
pub fn has_length_of(value_list: &ir::ValueList, num: usize, func: &ir::Function) -> bool {
    value_list.len(&func.dfg.value_lists) == num
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
