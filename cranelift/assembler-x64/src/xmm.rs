//! Xmm register operands; see [`Xmm`].

use crate::AsReg;

/// An x64 SSE register (e.g., `%xmm0`).
#[derive(Clone, Copy, Debug)]
pub struct Xmm<R: AsReg = u8>(pub(crate) R);

impl<R: AsReg> Xmm<R> {
    /// Create a new [`Xmm`] register.
    pub fn new(reg: R) -> Self {
        Self(reg)
    }

    /// Return the register's hardware encoding; the underlying type `R` _must_
    /// be a real register at this point.
    ///
    /// # Panics
    ///
    /// Panics if the register is not a valid Xmm register.
    pub fn enc(&self) -> u8 {
        let enc = self.0.enc();
        assert!(enc < 16, "invalid register: {enc}");
        enc
    }

    /// Return the register name.
    pub fn to_string(&self) -> &str {
        enc::to_string(self.enc())
    }
}

impl<R: AsReg> AsRef<R> for Xmm<R> {
    fn as_ref(&self) -> &R {
        &self.0
    }
}

impl<R: AsReg> AsMut<R> for Xmm<R> {
    fn as_mut(&mut self) -> &mut R {
        &mut self.0
    }
}

/// Encode xmm registers.
pub mod enc {
    pub const XMM0: u8 = 0;
    pub const XMM1: u8 = 1;
    pub const XMM2: u8 = 2;
    pub const XMM3: u8 = 3;
    pub const XMM4: u8 = 4;
    pub const XMM5: u8 = 5;
    pub const XMM6: u8 = 6;
    pub const XMM7: u8 = 7;
    pub const XMM8: u8 = 8;
    pub const XMM9: u8 = 9;
    pub const XMM10: u8 = 10;
    pub const XMM11: u8 = 11;
    pub const XMM12: u8 = 12;
    pub const XMM13: u8 = 13;
    pub const XMM14: u8 = 14;
    pub const XMM15: u8 = 15;

    /// Return the name of a XMM encoding (`enc`).
    ///
    /// # Panics
    ///
    /// This function will panic if the encoding is not a valid x64 register.
    pub fn to_string(enc: u8) -> &'static str {
        match enc {
            XMM0 => "%xmm0",
            XMM1 => "%xmm1",
            XMM2 => "%xmm2",
            XMM3 => "%xmm3",
            XMM4 => "%xmm4",
            XMM5 => "%xmm5",
            XMM6 => "%xmm6",
            XMM7 => "%xmm7",
            XMM8 => "%xmm8",
            XMM9 => "%xmm9",
            XMM10 => "%xmm10",
            XMM11 => "%xmm11",
            XMM12 => "%xmm12",
            XMM13 => "%xmm13",
            XMM14 => "%xmm14",
            XMM15 => "%xmm15",
            _ => panic!("%invalid{enc}"),
        }
    }
}
