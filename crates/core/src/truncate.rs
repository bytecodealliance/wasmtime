//! Explicit methods to clearly indicate that truncation is desired when used.

#![allow(
    clippy::cast_possible_truncation,
    reason = "methods for intentional truncation"
)]

/// Explicitly truncate an `i32` to an `i16`.
#[inline]
pub fn truncate_i32_to_i16(a: i32) -> i16 {
    a as i16
}

/// Explicitly truncate an `i32` to an `i8`.
#[inline]
pub fn truncate_i32_to_i8(a: i32) -> i8 {
    a as i8
}

/// Explicitly truncate a `u64` to an `u32`.
#[inline]
pub fn truncate_u64_to_u32(a: u64) -> u32 {
    a as u32
}
