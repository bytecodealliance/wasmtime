use std::time::SystemTime;

/// A value for specifying a time.
#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum SystemTimeSpec {
    /// An absolute time value.
    Absolute(SystemTime),
}

impl SystemTimeSpec {
    /// Constructs a new instance of [`fs_set_times::SystemTimeSpec`] from the
    /// given `Self`.
    #[inline]
    pub const fn into_std(self) -> fs_set_times::SystemTimeSpec {
        match self {
            Self::Absolute(time) => fs_set_times::SystemTimeSpec::Absolute(time),
        }
    }
}

impl From<SystemTime> for SystemTimeSpec {
    #[inline]
    fn from(time: SystemTime) -> Self {
        Self::Absolute(time)
    }
}
