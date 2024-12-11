//! Adapters for float methods to get routed to the `libm` dependency when the
//! `std` feature is disabled and these functions are otherwise not available.

pub trait FloatExt {
    fn trunc(self) -> Self;
    fn copysign(self, sign: Self) -> Self;
    fn floor(self) -> Self;
    fn ceil(self) -> Self;
    fn sqrt(self) -> Self;
    fn round(self) -> Self;
    fn abs(self) -> Self;
    fn round_ties_even(self) -> Self;
}

impl FloatExt for f32 {
    fn trunc(self) -> f32 {
        libm::truncf(self)
    }
    fn copysign(self, sign: f32) -> f32 {
        libm::copysignf(self, sign)
    }
    fn floor(self) -> f32 {
        libm::floorf(self)
    }
    fn ceil(self) -> f32 {
        libm::ceilf(self)
    }
    fn sqrt(self) -> f32 {
        libm::sqrtf(self)
    }
    fn round(self) -> f32 {
        libm::roundf(self)
    }
    fn abs(self) -> f32 {
        libm::fabsf(self)
    }
    fn round_ties_even(self) -> f32 {
        let round = self.round();
        if (self - round).abs() != 0.5 {
            return round;
        }
        match round % 2.0 {
            1.0 => self.floor(),
            -1.0 => self.ceil(),
            _ => round,
        }
    }
}

impl FloatExt for f64 {
    fn trunc(self) -> f64 {
        libm::trunc(self)
    }
    fn copysign(self, sign: f64) -> f64 {
        libm::copysign(self, sign)
    }
    fn floor(self) -> f64 {
        libm::floor(self)
    }
    fn ceil(self) -> f64 {
        libm::ceil(self)
    }
    fn sqrt(self) -> f64 {
        libm::sqrt(self)
    }
    fn round(self) -> f64 {
        libm::round(self)
    }
    fn abs(self) -> f64 {
        libm::fabs(self)
    }
    fn round_ties_even(self) -> f64 {
        let round = self.round();
        if (self - round).abs() != 0.5 {
            return round;
        }
        match round % 2.0 {
            1.0 => self.floor(),
            -1.0 => self.ceil(),
            _ => round,
        }
    }
}
