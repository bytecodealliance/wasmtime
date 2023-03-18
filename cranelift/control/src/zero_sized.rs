#[derive(Debug, Clone)]
pub struct ControlPlane {
    /// prevent direct instantiation (use `noop` or `todo` instead)
    _private: (),
}

impl ControlPlane {
    pub fn noop() -> Self {
        Self { _private: () }
    }

    pub fn todo() -> Self {
        Self::noop()
    }
}
