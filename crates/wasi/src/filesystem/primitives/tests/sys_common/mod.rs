mod symlink_junction;

pub mod io;

pub use symlink_junction::*;

macro_rules! check {
    ($e:expr) => {
        match $e {
            Ok(t) => t,
            Err(e) => panic!("{} failed with: {}", stringify!($e), e),
        }
    };
}

#[cfg(windows)]
macro_rules! error {
    ($e:expr, $s:expr) => {
        match $e {
            Ok(_) => panic!("Unexpected success. Should've been: {:?}", $s),
            Err(ref err) => {
                assert!(
                    err.raw_os_error() == Some($s),
                    "`{}` did not have a code of `{}`",
                    err,
                    $s
                )
            }
        }
    };
}

#[cfg(any(unix, target_os = "wasi"))]
macro_rules! error {
    ($e:expr, $s:expr) => {
        error_contains!($e, $s)
    };
}

macro_rules! error_contains {
    ($e:expr, $s:expr) => {
        match $e {
            Ok(_) => panic!("Unexpected success. Should've been: {:?}", $s),
            Err(ref err) => {
                assert!(
                    err.to_string().contains($s),
                    "`{}` did not contain `{}`",
                    err,
                    $s
                )
            }
        }
    };
}

// The following is derived from Rust's
// src/tools/cargo/crates/cargo-test-support/src/lib.rs at revision
// a78a62fc996ba16f7a111c99520b23f77029f4eb.

#[cfg(windows)]
pub fn symlink_supported() -> bool {
    let dir = tempfile::tempdir().unwrap();

    let src = dir.path().join("symlink_src");
    std::fs::write(&src, "").unwrap();
    let dst = dir.path().join("symlink_dst");
    let result = match std::os::windows::fs::symlink_file(&src, &dst) {
        Ok(_) => {
            std::fs::remove_file(&dst).unwrap();
            true
        }
        Err(e) => {
            eprintln!(
                "symlinks not supported: {e:?}\n\
                 Windows 10 users should enable developer mode.",
            );
            false
        }
    };
    std::fs::remove_file(&src).unwrap();
    return result;
}

#[cfg(not(windows))]
pub fn symlink_supported() -> bool {
    true
}
