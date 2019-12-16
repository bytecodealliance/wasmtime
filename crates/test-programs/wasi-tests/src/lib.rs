pub mod utils;
pub mod wasi_wrappers;

use wasi_old::wasi_unstable;

/// Opens a fresh file descriptor for `path` where `path` should be a preopened
/// directory. This is intended to be used with `wasi_unstable`, not with
/// `wasi_snapshot_preview1`. This is getting phased out and will likely be
/// deleted soon.
pub fn open_scratch_directory(path: &str) -> Result<wasi_unstable::Fd, String> {
    unsafe {
        for i in 3.. {
            let stat = match wasi_unstable::fd_prestat_get(i) {
                Ok(s) => s,
                Err(_) => break,
            };
            if stat.pr_type != wasi::PREOPENTYPE_DIR {
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

/// Same as `open_scratch_directory` above, except uses `wasi_snapshot_preview1`
/// APIs instead of `wasi_unstable` ones.
///
/// This is intended to replace `open_scratch_directory` once all the tests are
/// updated.
pub fn open_scratch_directory_new(path: &str) -> Result<wasi::Fd, String> {
    unsafe {
        for i in 3.. {
            let stat = match wasi::fd_prestat_get(i) {
                Ok(s) => s,
                Err(_) => break,
            };
            if stat.pr_type != wasi::PREOPENTYPE_DIR {
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
