#[derive(Debug, Clone)]
pub struct ControlPlane {
    /// prevent direct instantiation (use `noop` or `todo` instead)
    _private: (),
}

impl ControlPlane {
    pub fn no_chaos() -> Self {
        Self { _private: () }
    }

    // get_decision function is not implemented here because it should only be called within
    // conditional compiled code blocks. In that case the other implementation would be used anyway.
}

impl Default for ControlPlane {
    fn default() -> Self {
        Self::no_chaos()
    }
}
