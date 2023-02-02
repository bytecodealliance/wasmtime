use regalloc2::{PReg, RegClass};

/// A newtype abstraction on top of a physical register.
//
// NOTE
// This is temporary; the intention behind this newtype
// is to keep the usage of PReg contained to this module
// so that the rest of Winch should only need to operate
// on top of the concept of `Reg`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Reg(PReg);

impl Reg {
    /// Create a new register from a physical register.
    pub const fn new(raw: PReg) -> Self {
        Reg(raw)
    }

    /// Create a new general purpose register from encoding.
    pub fn int(enc: usize) -> Self {
        Self::new(PReg::new(enc, RegClass::Int))
    }

    /// Create a new floating point register from encoding.
    #[allow(dead_code)]
    pub fn float(enc: usize) -> Self {
        Self::new(PReg::new(enc, RegClass::Float))
    }

    /// Get the class of the underlying register.
    pub fn class(self) -> RegClass {
        self.0.class()
    }

    /// Get the encoding of the underlying register.
    pub fn hw_enc(self) -> u8 {
        self.0.hw_enc() as u8
    }

    /// Get the physical register representation.
    pub(super) fn inner(&self) -> PReg {
        self.0
    }
}

impl From<Reg> for cranelift_codegen::Reg {
    fn from(reg: Reg) -> Self {
        reg.inner().into()
    }
}

impl std::fmt::Debug for Reg {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
