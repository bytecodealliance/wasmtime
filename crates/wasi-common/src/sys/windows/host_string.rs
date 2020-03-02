use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};

/// A string in the format OS APIs prefer to consume. For Windows, this is
/// just `OsString`.
pub(crate) type HostString = OsString;

/// Convert an `OsString` to a `HostString`.
pub(crate) fn hoststring_from_osstring(os: OsString) -> HostString {
    os
}

/// Test whether the given `HostString` ends with a slash.
pub(crate) fn hoststring_ends_with_slash(host: &HostString) -> bool {
    osstr_ends_with_slash(host)
}

/// Test whether the given `OsStr` ends with a slash.
pub(crate) fn osstr_ends_with_slash(os: &OsStr) {
    os.encode_wide().last() == Some(u16, '/')
}
