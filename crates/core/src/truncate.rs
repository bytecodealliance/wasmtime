//! Explicit methods to clearly indicate that truncation is desired when used.

/// Explicitly truncate an `i32` to an `i16`.
pub fn truncate_i32_to_i16(a: i32) -> i16 {
    a as i16
}

/// Explicitly truncate an `i32` to an `i8`.
pub fn truncate_i32_to_i8(a: i32) -> i8 {
    a as i8
}
