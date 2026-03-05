/// Memory ordering for atomic operations.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "enable-serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AtomicOrdering {
    /// Sequentially Consistent: strongest ordering.
    SeqCst = 0b000,
    /// Acquire-Release: both Acquire and Release.
    AcqRel = 0b001,
    /// Acquire: ensures subsequent loads are not reordered before this.
    Acquire = 0b010,
    /// Release: ensures prior stores are not reordered after this.
    Release = 0b011,
    /// Relaxed: no ordering guarantees.
    Relaxed = 0b100,
}

impl AtomicOrdering {
    /// Returns a slice with all supported [AtomicOrdering]'s.
    pub fn all() -> &'static [Self] {
        &[
            Self::SeqCst,
            Self::AcqRel,
            Self::Acquire,
            Self::Relaxed,
            Self::Release,
        ]
    }

    pub(crate) const fn to_u8(self) -> u8 {
        match self {
            Self::SeqCst => 0b000,
            Self::AcqRel => 0b001,
            Self::Acquire => 0b010,
            Self::Relaxed => 0b011,
            Self::Release => 0b100,
        }
    }

    pub(crate) const fn from_u8(bits: u8) -> Self {
        match bits {
            0b000 => Self::SeqCst,
            0b001 => Self::AcqRel,
            0b010 => Self::Acquire,
            0b011 => Self::Relaxed,
            0b100 => Self::Release,
            _ => unreachable!(),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_parse() {
        for op in AtomicOrdering::all() {
            let roundtripped = format!("{op}").parse::<AtomicOrdering>().unwrap();
            assert_eq!(*op, roundtripped);
        }
    }
}
