use more_asserts::assert_gt;
pub mod config;

lazy_static::lazy_static! {
    pub static ref TESTCONFIG: config::TestConfig = config::TestConfig::from_env();
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
            if stat.tag != wasi::PREOPENTYPE_DIR {
                continue;
            }
            let mut dst = Vec::with_capacity(stat.u.dir.pr_name_len);
            if wasi::fd_prestat_dir_name(i, dst.as_mut_ptr(), dst.capacity()).is_err() {
                continue;
            }
            dst.set_len(stat.u.dir.pr_name_len);
            if dst == path.as_bytes() {
                let (base, inherit) = fd_get_rights(i);
                return Ok(
                    wasi::path_open(i, 0, ".", wasi::OFLAGS_DIRECTORY, base, inherit, 0)
                        .expect("failed to open dir"),
                );
            }
        }

        Err(format!("failed to find scratch dir"))
    }
}

pub unsafe fn create_file(dir_fd: wasi::Fd, filename: &str) {
    let file_fd =
        wasi::path_open(dir_fd, 0, filename, wasi::OFLAGS_CREAT, 0, 0, 0).expect("creating a file");
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );
    wasi::fd_close(file_fd).expect("closing a file");
}

// Returns: (rights_base, rights_inheriting)
pub unsafe fn fd_get_rights(fd: wasi::Fd) -> (wasi::Rights, wasi::Rights) {
    let fdstat = wasi::fd_fdstat_get(fd).expect("fd_fdstat_get failed");
    (fdstat.fs_rights_base, fdstat.fs_rights_inheriting)
}

pub unsafe fn drop_rights(fd: wasi::Fd, drop_base: wasi::Rights, drop_inheriting: wasi::Rights) {
    let (current_base, current_inheriting) = fd_get_rights(fd);

    let new_base = current_base & !drop_base;
    let new_inheriting = current_inheriting & !drop_inheriting;

    wasi::fd_fdstat_set_rights(fd, new_base, new_inheriting).expect("dropping fd rights");
}

#[macro_export]
macro_rules! assert_errno {
    ($s:expr, windows => $i:expr, $( $rest:tt )+) => {
        let e = $s;
        if $crate::TESTCONFIG.errno_expect_windows() {
            assert_errno!(e, $i);
        } else {
            assert_errno!(e, $($rest)+, $i);
        }
    };
    ($s:expr, macos => $i:expr, $( $rest:tt )+) => {
        let e = $s;
        if $crate::TESTCONFIG.errno_expect_macos() {
            assert_errno!(e, $i);
        } else {
            assert_errno!(e, $($rest)+, $i);
        }
    };
    ($s:expr, unix => $i:expr, $( $rest:tt )+) => {
        let e = $s;
        if $crate::TESTCONFIG.errno_expect_unix() {
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
                Alt(&[ $( wasi::errno_name($i) ),+ ]),
                wasi::errno_name(e),
            )
        }
    };
}
