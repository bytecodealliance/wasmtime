//! Shims for ControlPlane when chaos mode is disabled. Enables
//! unconditional use of the type and its methods throughout cranelift.

use std::fmt::Display;

/// A shim for ControlPlane when chaos mode is disabled.
/// Please see the [crate-level documentation](crate).
#[derive(Debug, Clone, Default)]
pub struct ControlPlane {
    /// prevent direct instantiation (use `default` instead)
    _private: (),
}

/// A shim for ControlPlane's `Arbitrary` implementation when chaos mode is
/// disabled. It doesn't consume any bytes and always returns a default
/// control plane.
impl arbitrary::Arbitrary<'_> for ControlPlane {
    fn arbitrary(_u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        Ok(Self::default())
    }
}

impl ControlPlane {
    /// Set the maximum number of perturbations to be introduced with chaos
    /// mode. This variant is used when chaos mode is disabled. It doesn't
    /// do anything.
    pub fn set_fuel(&mut self, _fuel: u8) {}

    /// Returns a pseudo-random boolean. This variant is used when chaos
    /// mode is disabled. It always returns `false`.
    #[inline]
    pub fn get_decision(&mut self) -> bool {
        false
    }
}

impl Display for ControlPlane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "; control plane: ()")
    }
}
