use super::compute_oflags;
use crate::filesystem::primitives::{OpenOptions, OpenUncheckedError, stat_unchecked};
use rustix::fs::{Mode, openat};
use rustix::io;
use std::fs;
use std::path::Path;

/// *Unsandboxed* function similar to `open`, but which does not perform
/// sandboxing.
pub(crate) fn open_unchecked(
    start: &fs::File,
    path: &Path,
    options: &OpenOptions,
) -> Result<fs::File, OpenUncheckedError> {
    let oflags = compute_oflags(options).map_err(OpenUncheckedError::Other)?;

    #[allow(clippy::useless_conversion)]
    #[cfg(not(target_os = "wasi"))]
    let mode = Mode::from_bits_truncate(options.ext.mode as _);
    #[cfg(target_os = "wasi")]
    let mode = Mode::empty();

    let err = match openat(start, path, oflags, mode) {
        Ok(file) => {
            return Ok(fs::File::from(file));
        }
        Err(err) => err,
    };
    match err {
        // `ELOOP` is the POSIX standard and most widely used error code to
        // indicate that a symlink was found when `O_NOFOLLOW` was set.
        #[cfg(not(any(target_os = "freebsd", target_os = "dragonfly", target_os = "netbsd")))]
        io::Errno::LOOP => Err(OpenUncheckedError::Symlink(err.into(), ())),

        // FreeBSD and similar (but not Darwin) use `EMLINK`.
        #[cfg(any(target_os = "freebsd", target_os = "dragonfly"))]
        io::Errno::MLINK => Err(OpenUncheckedError::Symlink(err.into(), ())),

        // NetBSD uses `EFTYPE`.
        #[cfg(target_os = "netbsd")]
        io::Errno::FTYPE => Err(OpenUncheckedError::Symlink(err.into(), ())),

        io::Errno::NOENT => Err(OpenUncheckedError::NotFound(err.into())),
        io::Errno::NOTDIR => {
            if options.dir_required
                && stat_unchecked(start, path, options.follow)
                    .map(|m| m.file_type().is_symlink())
                    .unwrap_or(false)
            {
                Err(OpenUncheckedError::Symlink(err.into(), ()))
            } else {
                Err(OpenUncheckedError::NotFound(err.into()))
            }
        }
        _ => Err(OpenUncheckedError::Other(err.into())),
    }
}
