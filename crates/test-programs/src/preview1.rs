use std::{sync::OnceLock, time::Duration};

pub fn config() -> &'static TestConfig {
    static TESTCONFIG: OnceLock<TestConfig> = OnceLock::new();
    TESTCONFIG.get_or_init(TestConfig::from_env)
}

// The `wasi` crate version 0.9.0 and beyond, doesn't
// seem to define these constants, so we do it ourselves.
pub const STDIN_FD: wasi::Fd = 0x0;
pub const STDOUT_FD: wasi::Fd = 0x1;
pub const STDERR_FD: wasi::Fd = 0x2;

/// Opens a fresh file descriptor for `path` where `path` should be a preopened
/// directory.
pub fn open_scratch_directory(path: &str) -> Result<wasi::Fd, String> {
    unsafe {
        for i in 3.. {
            let stat = match wasi::fd_prestat_get(i) {
                Ok(s) => s,
                Err(_) => break,
            };
            if stat.tag != wasi::PREOPENTYPE_DIR.raw() {
                continue;
            }
            let mut dst = Vec::with_capacity(stat.u.dir.pr_name_len);
            if wasi::fd_prestat_dir_name(i, dst.as_mut_ptr(), dst.capacity()).is_err() {
                continue;
            }
            dst.set_len(stat.u.dir.pr_name_len);
            if dst == path.as_bytes() {
                return Ok(wasi::path_open(i, 0, ".", wasi::OFLAGS_DIRECTORY, 0, 0, 0)
                    .expect("failed to open dir"));
            }
        }

        Err(format!("failed to find scratch dir"))
    }
}

pub unsafe fn create_file(dir_fd: wasi::Fd, filename: &str) {
    let file_fd =
        wasi::path_open(dir_fd, 0, filename, wasi::OFLAGS_CREAT, 0, 0, 0).expect("creating a file");
    assert!(file_fd > STDERR_FD, "file descriptor range check",);
    wasi::fd_close(file_fd).expect("closing a file");
}

// Small workaround to get the crate's macros, through the
// `#[macro_export]` attribute below, also available from this module.
pub use crate::{assert_errno, assert_fs_time_eq};

#[macro_export]
macro_rules! assert_errno {
    ($s:expr, windows => $i:expr, $( $rest:tt )+) => {
        let e = $s;
        if $crate::preview1::config().errno_expect_windows() {
            assert_errno!(e, $i);
        } else {
            assert_errno!(e, $($rest)+, $i);
        }
    };
    ($s:expr, macos => $i:expr, $( $rest:tt )+) => {
        let e = $s;
        if $crate::preview1::config().errno_expect_macos() {
            assert_errno!(e, $i);
        } else {
            assert_errno!(e, $($rest)+, $i);
        }
    };
    ($s:expr, unix => $i:expr, $( $rest:tt )+) => {
        let e = $s;
        if $crate::preview1::config().errno_expect_unix() {
            assert_errno!(e, $i);
        } else {
            assert_errno!(e, $($rest)+, $i);
        }
    };
    ($s:expr, $( $i:expr ),+) => {
        let e = $s;
        {
            // Pretty printing infrastructure
            struct Alt<'a>(&'a [&'static str]);
            impl<'a> std::fmt::Display for Alt<'a> {
                fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    let l = self.0.len();
                    if l == 0 {
                        unreachable!()
                    } else if l == 1 {
                        f.write_str(self.0[0])
                    } else if l == 2 {
                        f.write_str(self.0[0])?;
                        f.write_str(" or ")?;
                        f.write_str(self.0[1])
                    } else {
                        for (ix, s) in self.0.iter().enumerate() {
                            if ix == l - 1 {
                                f.write_str("or ")?;
                                f.write_str(s)?;
                            } else {
                                f.write_str(s)?;
                                f.write_str(", ")?;
                            }
                        }
                        Ok(())
                    }
                }
            }
            assert!( $( e == $i || )+ false,
                "expected errno {}; got {}",
                Alt(&[ $( $i.name() ),+ ]),
                e.name()
            )
        }
    };
}

#[macro_export]
macro_rules! assert_fs_time_eq {
    ($l:expr, $r:expr, $n:literal) => {
        let diff = if $l > $r { $l - $r } else { $r - $l };
        assert!(diff < $crate::preview1::config().fs_time_precision(), $n);
    };
}

pub struct TestConfig {
    errno_mode: ErrnoMode,
    fs_time_precision: u64,
    no_dangling_filesystem: bool,
    no_rename_dir_to_empty_dir: bool,
    no_fdflags_sync_support: bool,
}

enum ErrnoMode {
    Unix,
    MacOS,
    Windows,
    Permissive,
}

impl TestConfig {
    pub fn from_env() -> Self {
        let errno_mode = if std::env::var("ERRNO_MODE_UNIX").is_ok() {
            ErrnoMode::Unix
        } else if std::env::var("ERRNO_MODE_MACOS").is_ok() {
            ErrnoMode::MacOS
        } else if std::env::var("ERRNO_MODE_WINDOWS").is_ok() {
            ErrnoMode::Windows
        } else {
            ErrnoMode::Permissive
        };
        let fs_time_precision = match std::env::var("FS_TIME_PRECISION") {
            Ok(p) => p.parse().unwrap(),
            Err(_) => 100,
        };
        let no_dangling_filesystem = std::env::var("NO_DANGLING_FILESYSTEM").is_ok();
        let no_rename_dir_to_empty_dir = std::env::var("NO_RENAME_DIR_TO_EMPTY_DIR").is_ok();
        let no_fdflags_sync_support = std::env::var("NO_FDFLAGS_SYNC_SUPPORT").is_ok();
        TestConfig {
            errno_mode,
            fs_time_precision,
            no_dangling_filesystem,
            no_rename_dir_to_empty_dir,
            no_fdflags_sync_support,
        }
    }
    pub fn errno_expect_unix(&self) -> bool {
        match self.errno_mode {
            ErrnoMode::Unix | ErrnoMode::MacOS => true,
            _ => false,
        }
    }
    pub fn errno_expect_macos(&self) -> bool {
        match self.errno_mode {
            ErrnoMode::MacOS => true,
            _ => false,
        }
    }
    pub fn errno_expect_windows(&self) -> bool {
        match self.errno_mode {
            ErrnoMode::Windows => true,
            _ => false,
        }
    }
    pub fn fs_time_precision(&self) -> Duration {
        Duration::from_nanos(self.fs_time_precision)
    }
    pub fn support_dangling_filesystem(&self) -> bool {
        !self.no_dangling_filesystem
    }
    pub fn support_rename_dir_to_empty_dir(&self) -> bool {
        !self.no_rename_dir_to_empty_dir
    }
    pub fn support_fdflags_sync(&self) -> bool {
        !self.no_fdflags_sync_support
    }
}
