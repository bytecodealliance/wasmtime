use std::fmt::Display;

use arbitrary::Unstructured;

/// The control plane of chaos mode.
/// Please see the [crate-level documentation](crate).
#[derive(Debug, Clone, Default)]
pub struct ControlPlane {
    data: Vec<bool>,
    fuel: Option<u8>,
}

impl ControlPlane {
    /// Generate a new control plane using arbitrary and a given [fuel
    /// limit](crate#fuel-limit). The fuel limit should be determined by the
    /// cranelift setting `control_plane_fuel`.
    pub fn new(u: &mut Unstructured, fuel: u8) -> arbitrary::Result<Self> {
        Ok(Self {
            data: u.arbitrary()?,
            fuel: (fuel != 0).then_some(fuel),
        })
    }

    /// Tries to consume fuel, returning `true` if successful (or if
    /// fuel-limiting is disabled).
    fn consume_fuel(&mut self) -> bool {
        match self.fuel {
            None => true,               // fuel deactivated
            Some(f) if f == 0 => false, // no more fuel
            Some(ref mut f) => {
                *f -= 1;
                true
            }
        }
    }

    /// Returns a pseudo-random boolean if the control plane was constructed
    /// with `arbitrary`.
    ///
    /// The default value `false` will always be returned if the
    /// pseudo-random data is exhausted or the control plane was constructed
    /// with `default`.
    pub fn get_decision(&mut self) -> bool {
        self.consume_fuel() && self.data.pop().unwrap_or_default()
    }
}

impl Display for ControlPlane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "; control plane:")?;
        write!(f, ";    data:")?;
        for b in &self.data {
            // TODO will be replaced by hex (or base64 ?) encoded data
            // once we switch to from `Vec<bool>` to `Vec<u8>`.
            write!(f, " {b}")?;
        }
        writeln!(f, "")?;
        writeln!(f, ";    fuel: {:?}", self.fuel)
    }
}
