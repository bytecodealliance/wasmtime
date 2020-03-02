use std::ffi::{CString, OsStr, OsString};
use std::os::unix::ffi::{OsStrExt, OsStringExt};

/// A string in the format OS APIs prefer to consume. For Unix-style
/// platforms, this is similar to `OsString`, but we also need the
/// strings to be NUL-terminated, so we use `CString`.
pub(crate) type HostString = CString;

/// Convert an `OsString` to a `HostString`.
pub(crate) fn hoststring_from_osstring(os: OsString) -> HostString {
    let vec = os.into_vec();
    assert!(!vec.contains(&b'\0'));
    unsafe { HostString::from_vec_unchecked(vec) }
}

/// Test whether the given `HostString` ends with a slash.
pub(crate) fn hoststring_ends_with_slash(host: &HostString) -> bool {
    host.to_bytes().ends_with(b"/")
}

/// Test whether the given `OsStr` ends with a slash.
pub(crate) fn osstr_ends_with_slash(os: &OsStr) -> bool {
    os.as_bytes().ends_with(b"/")
}
