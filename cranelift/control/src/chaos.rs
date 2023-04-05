use arbitrary::Arbitrary;

/// The control plane of chaos mode.
/// Please see the [crate-level documentation](crate).
#[derive(Debug, Clone, Default)]
pub struct ControlPlane {
    data: Vec<bool>,
}

impl Arbitrary<'_> for ControlPlane {
    fn arbitrary<'a>(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self {
            data: u.arbitrary()?,
        })
    }
}

impl ControlPlane {
    /// Returns a pseudo-random boolean if the control plane was constructed
    /// with `arbitrary`.
    ///
    /// The default value `false` will always be returned if the
    /// pseudo-random data is exhausted or the control plane was constructed
    /// with `default`.
    pub fn get_decision(&mut self) -> bool {
        self.data.pop().unwrap_or_default()
    }
}
