/// Memory ordering for atomic operations.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AtomicOrdering {
    /// Relaxed: no ordering guarantees.
    Relaxed,
    /// Acquire: ensures subsequent loads are not reordered before this.
    Acquire,
    /// Release: ensures prior stores are not reordered after this.
    Release,
    /// Acquire-Release: both Acquire and Release.
    AcqRel,
    /// Sequentially Consistent: strongest ordering.
    SeqCst,
}

impl Default for AtomicOrdering {
    fn default() -> Self {
        Self::SeqCst
    }
}

impl std::fmt::Display for AtomicOrdering {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            Self::Relaxed => "relaxed",
            Self::Acquire => "acquire",
            Self::Release => "release",
            Self::AcqRel => "acq_rel",
            Self::SeqCst => "seq_cst",
        };
        f.write_str(s)
    }
}

impl std::str::FromStr for AtomicOrdering {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "relaxed" => Ok(Self::Relaxed),
            "acquire" => Ok(Self::Acquire),
            "release" => Ok(Self::Release),
            "acq_rel" => Ok(Self::AcqRel),
            "seq_cst" => Ok(Self::SeqCst),
            _ => Err(()),
        }
    }
}
