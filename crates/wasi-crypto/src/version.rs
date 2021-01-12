use crate::types as guest_types;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Version(u64);

impl Version {
    pub const UNSPECIFIED: Version = Version(0xff00_0000_0000_0000);
    pub const LATEST: Version = Version(0xff00_0000_0000_0000);
    pub const ALL: Version = Version(0xff00_0000_0000_0000);
}

impl From<guest_types::Version> for Version {
    fn from(version: guest_types::Version) -> Self {
        Version(version.into())
    }
}

impl From<Version> for guest_types::Version {
    fn from(version: Version) -> Self {
        version.into()
    }
}
