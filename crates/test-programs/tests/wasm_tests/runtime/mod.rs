pub mod cap_std_sync;
pub mod tokio;

// Configure the test suite environment.
// Test programs use these environment variables to determine what behavior
// is expected: different errnos are expected on windows, mac, and other unixes,
// and other filesystem operations are supported or not.
pub fn test_suite_environment() -> &'static [(&str, &str)] {
    #[cfg(windows)]
    {
        &[
            ("ERRNO_MODE_WINDOWS", "1"),
            // Windows does not support dangling links or symlinks in the filesystem.
            ("NO_DANGLING_FILESYSTEM", "1"),
            // Windows does not support fd_allocate.
            ("NO_FD_ALLOCATE", "1"),
            // Windows does not support renaming a directory to an empty directory -
            // empty directory must be deleted.
            ("NO_RENAME_DIR_TO_EMPTY_DIR", "1"),
        ]
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        &[("ERRNO_MODE_UNIX", "1")]
    }
    #[cfg(target_os = "macos")]
    {
        &[
            ("ERRNO_MODE_MACOS", "1"),
            // MacOS does not support fd_allocate
            ("NO_FD_ALLOCATE", "1"),
        ]
    }
}
