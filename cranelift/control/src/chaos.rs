use std::fmt::Display;

use arbitrary::Arbitrary;

/// The control plane of chaos mode.
/// Please see the [crate-level documentation](crate).
#[derive(Debug, Clone, Default)]
pub struct ControlPlane {
    data: Vec<bool>,
    fuel: Option<u32>,
}

impl Arbitrary<'_> for ControlPlane {
    fn arbitrary<'a>(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self {
            data: u.arbitrary()?,
            fuel: None,
        })
    }
}

impl ControlPlane {
    /// Returns `true` if fuel wasn't activated in the first place, `false`
    /// if there is no more fuel available.
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

    /// Set the maximum number of perturbations to be introduced with chaos
    /// mode. Can be used to binary-search for a perturbation that triggered
    /// a bug.
    pub fn set_fuel(&mut self, fuel: u32) {
        self.fuel = Some(fuel)
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
        writeln!(f, "control plane:")?;
        write!(f, "    data:")?;
        for b in &self.data {
            // TODO will be replaced by hex (or base64 ?) encoded data
            // once we switch to from `Vec<bool>` to `Vec<u8>`.
            write!(f, " {b}")?;
        }
        writeln!(f, "")?;
        writeln!(f, "    fuel: {:?}", self.fuel)
    }
}
