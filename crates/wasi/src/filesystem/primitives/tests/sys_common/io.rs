use cap_tempfile::tempdir;

#[cfg(feature = "fs_utf8")]
pub use cap_tempfile::utf8::TempDir as TempDirUtf8;
pub use cap_tempfile::TempDir;

#[allow(unused)]
pub fn tmpdir() -> TempDir {
    use cap_tempfile::ambient_authority;

    // It's ok to call `ambient_authority()` here, rather than take an
    // `AmbientAuthority` argument, because this function is only used
    // by tests.
    tempdir(ambient_authority()).expect("expected to be able to create a temporary directory")
}

#[cfg(feature = "fs_utf8")]
#[allow(unused)]
pub fn tmpdir_utf8() -> TempDirUtf8 {
    use cap_tempfile::ambient_authority;
    use cap_tempfile::utf8::tempdir;

    // It's ok to call `ambient_authority()` here, rather than take an
    // `AmbientAuthority` argument, because this function is only used
    // by tests.
    tempdir(ambient_authority()).expect("expected to be able to create a temporary directory")
}
