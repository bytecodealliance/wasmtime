use target_lexicon::{Architecture, Triple};

/// Extension methods for `target_lexicon::Triple`.
pub trait TripleExt {
    /// Helper for returning whether this target is for pulley, wasmtime's
    /// interpreter.
    fn is_pulley(&self) -> bool;
}

impl TripleExt for Triple {
    fn is_pulley(&self) -> bool {
        match self.architecture {
            Architecture::Pulley32 => true,
            Architecture::Pulley64 => true,
            _ => false,
        }
    }
}
