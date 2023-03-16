#[derive(Debug, Clone)]
pub struct ChaosEngine {
    /// prevent direct instantiation (use `noop` or `todo` instead)
    _private: (),
}

impl ChaosEngine {
    pub fn noop() -> Self {
        Self { _private: () }
    }

    pub fn todo() -> Self {
        Self::noop()
    }
}
