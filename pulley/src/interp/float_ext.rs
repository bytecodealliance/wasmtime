//! Adapters for float methods to get routed to the `libm` dependency when the
//! `std` feature is disabled and these functions are otherwise not available.

pub trait FloatExt {
    fn trunc(self) -> Self;
}

impl FloatExt for f32 {
    fn trunc(self) -> f32 {
        libm::truncf(self)
    }
}

impl FloatExt for f64 {
    fn trunc(self) -> f64 {
        libm::trunc(self)
    }
}
