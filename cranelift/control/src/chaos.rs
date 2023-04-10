use arbitrary::Arbitrary;

/// The control plane of chaos mode.
/// Please see the [crate-level documentation](crate).
#[derive(Debug, Clone, Default)]
pub struct ControlPlane {
    data: Vec<bool>,
    fuel: Option<u8>,
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
    /// returns false if there is no more available fuel
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
    /// mode. Can be used to binary-search for a perturbation that caused a
    /// bug.
    pub fn set_fuel(&mut self, fuel: u8) {
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
