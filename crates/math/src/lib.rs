//! A minimal helper crate for implementing float-related operations for
//! WebAssembly in terms of the native platform primitives.
//!
//! This crate is intended to assist with solving the portability issues such
//! as:
//!
//! * Functions like `f32::trunc` are not available in `#![no_std]` targets.
//! * The `f32::trunc` function is likely faster than the `libm` fallback.
//! * Behavior of `f32::trunc` differs across platforms, for example it's
//!   different on Windows and glibc on Linux. Additionally riscv64's
//!   implementation of `libm` seems to have different NaN behavior than other
//!   platforms.
//! * Some wasm functions are in the Rust standard library, but not stable yet.
//!
//! There are a few locations throughout the codebase that these functions are
//! needed so they're implemented only in a single location here rather than
//! multiple.

#![no_std]

#[cfg(feature = "std")]
extern crate std;

pub trait WasmFloat {
    fn wasm_trunc(self) -> Self;
    fn wasm_copysign(self, sign: Self) -> Self;
    fn wasm_floor(self) -> Self;
    fn wasm_ceil(self) -> Self;
    fn wasm_sqrt(self) -> Self;
    fn wasm_abs(self) -> Self;
    fn wasm_nearest(self) -> Self;
    fn wasm_minimum(self, other: Self) -> Self;
    fn wasm_maximum(self, other: Self) -> Self;
    fn mul_add(self, b: Self, c: Self) -> Self;
}

impl WasmFloat for f32 {
    #[inline]
    fn wasm_trunc(self) -> f32 {
        #[cfg(feature = "std")]
        if !cfg!(windows) && !cfg!(target_arch = "riscv64") {
            return self.trunc();
        }
        if self.is_nan() {
            return f32::NAN;
        }
        libm::truncf(self)
    }
    #[inline]
    fn wasm_copysign(self, sign: f32) -> f32 {
        #[cfg(feature = "std")]
        if true {
            return self.copysign(sign);
        }
        libm::copysignf(self, sign)
    }
    #[inline]
    fn wasm_floor(self) -> f32 {
        #[cfg(feature = "std")]
        if !cfg!(target_arch = "riscv64") {
            return self.floor();
        }
        if self.is_nan() {
            return f32::NAN;
        }
        libm::floorf(self)
    }
    #[inline]
    fn wasm_ceil(self) -> f32 {
        #[cfg(feature = "std")]
        if !cfg!(target_arch = "riscv64") {
            return self.ceil();
        }
        if self.is_nan() {
            return f32::NAN;
        }
        libm::ceilf(self)
    }
    #[inline]
    fn wasm_sqrt(self) -> f32 {
        #[cfg(feature = "std")]
        if true {
            return self.sqrt();
        }
        libm::sqrtf(self)
    }
    #[inline]
    fn wasm_abs(self) -> f32 {
        #[cfg(feature = "std")]
        if true {
            return self.abs();
        }
        libm::fabsf(self)
    }
    #[inline]
    fn wasm_nearest(self) -> f32 {
        #[cfg(feature = "std")]
        if !cfg!(windows) && !cfg!(target_arch = "riscv64") {
            return self.round_ties_even();
        }
        if self.is_nan() {
            return f32::NAN;
        }
        let round = libm::roundf(self);
        if libm::fabsf(self - round) != 0.5 {
            return round;
        }
        match round % 2.0 {
            1.0 => libm::floorf(self),
            -1.0 => libm::ceilf(self),
            _ => round,
        }
    }
    #[inline]
    fn wasm_maximum(self, other: f32) -> f32 {
        // FIXME: replace this with `a.maximum(b)` when rust-lang/rust#91079 is
        // stabilized
        if self > other {
            self
        } else if other > self {
            other
        } else if self == other {
            if self.is_sign_positive() && other.is_sign_negative() {
                self
            } else {
                other
            }
        } else {
            self + other
        }
    }
    #[inline]
    fn wasm_minimum(self, other: f32) -> f32 {
        // FIXME: replace this with `self.minimum(other)` when
        // rust-lang/rust#91079 is stabilized
        if self < other {
            self
        } else if other < self {
            other
        } else if self == other {
            if self.is_sign_negative() && other.is_sign_positive() {
                self
            } else {
                other
            }
        } else {
            self + other
        }
    }
    #[inline]
    fn mul_add(self, b: f32, c: f32) -> f32 {
        #[cfg(feature = "std")]
        if true {
            return self.mul_add(b, c);
        }
        libm::fmaf(self, b, c)
    }
}

impl WasmFloat for f64 {
    #[inline]
    fn wasm_trunc(self) -> f64 {
        #[cfg(feature = "std")]
        if !cfg!(windows) && !cfg!(target_arch = "riscv64") {
            return self.trunc();
        }
        if self.is_nan() {
            return f64::NAN;
        }
        libm::trunc(self)
    }
    #[inline]
    fn wasm_copysign(self, sign: f64) -> f64 {
        #[cfg(feature = "std")]
        if true {
            return self.copysign(sign);
        }
        libm::copysign(self, sign)
    }
    #[inline]
    fn wasm_floor(self) -> f64 {
        #[cfg(feature = "std")]
        if !cfg!(target_arch = "riscv64") {
            return self.floor();
        }
        if self.is_nan() {
            return f64::NAN;
        }
        libm::floor(self)
    }
    #[inline]
    fn wasm_ceil(self) -> f64 {
        #[cfg(feature = "std")]
        if !cfg!(target_arch = "riscv64") {
            return self.ceil();
        }
        if self.is_nan() {
            return f64::NAN;
        }
        libm::ceil(self)
    }
    #[inline]
    fn wasm_sqrt(self) -> f64 {
        #[cfg(feature = "std")]
        if true {
            return self.sqrt();
        }
        libm::sqrt(self)
    }
    #[inline]
    fn wasm_abs(self) -> f64 {
        #[cfg(feature = "std")]
        if true {
            return self.abs();
        }
        libm::fabs(self)
    }
    #[inline]
    fn wasm_nearest(self) -> f64 {
        #[cfg(feature = "std")]
        if !cfg!(windows) && !cfg!(target_arch = "riscv64") {
            return self.round_ties_even();
        }
        if self.is_nan() {
            return f64::NAN;
        }
        let round = libm::round(self);
        if libm::fabs(self - round) != 0.5 {
            return round;
        }
        match round % 2.0 {
            1.0 => libm::floor(self),
            -1.0 => libm::ceil(self),
            _ => round,
        }
    }
    #[inline]
    fn wasm_maximum(self, other: f64) -> f64 {
        // FIXME: replace this with `a.maximum(b)` when rust-lang/rust#91079 is
        // stabilized
        if self > other {
            self
        } else if other > self {
            other
        } else if self == other {
            if self.is_sign_positive() && other.is_sign_negative() {
                self
            } else {
                other
            }
        } else {
            self + other
        }
    }
    #[inline]
    fn wasm_minimum(self, other: f64) -> f64 {
        // FIXME: replace this with `self.minimum(other)` when
        // rust-lang/rust#91079 is stabilized
        if self < other {
            self
        } else if other < self {
            other
        } else if self == other {
            if self.is_sign_negative() && other.is_sign_positive() {
                self
            } else {
                other
            }
        } else {
            self + other
        }
    }
    #[inline]
    fn mul_add(self, b: f64, c: f64) -> f64 {
        #[cfg(feature = "std")]
        if true {
            return self.mul_add(b, c);
        }
        libm::fma(self, b, c)
    }
}
