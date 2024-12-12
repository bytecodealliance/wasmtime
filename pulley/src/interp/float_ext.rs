//! Adapters for float methods to get routed to the `libm` dependency when the
//! `std` feature is disabled and these functions are otherwise not available.

pub trait FloatExt {
    fn trunc(self) -> Self;
    fn copysign(self, sign: Self) -> Self;
}

impl FloatExt for f32 {
    fn trunc(self) -> f32 {
        libm::truncf(self)
    }

    fn copysign(self, sign: f32) -> f32 {
        libm::copysignf(self, sign)
    }
}

impl FloatExt for f64 {
    fn trunc(self) -> f64 {
        libm::trunc(self)
    }

    fn copysign(self, sign: f64) -> f64 {
        libm::copysign(self, sign)
    }
}
