use crate::filesystem::primitives::unix::{compute_oflags, to_timespec};
use crate::filesystem::primitives::{
    FollowSymlinks, ImplMetadataExt, Metadata, OpenOptions, OpenOptionsExt, errors, open,
};
use rustix::fs::{
    AtFlags, Mode, OFlags, RawMode, ResolveFlags, Timestamps, openat2, statat, utimensat,
};
use rustix::io::Errno;
use rustix::path::{Arg, DecInt};
use rustix_linux_procfs::proc_self_fd;
use std::fs;
use std::io;
use std::os::fd::AsFd;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::time::SystemTime;

/// Linux 5.6 and later have a syscall `openat2`, with flags that allow it to
/// enforce the sandboxing property we want. See the [LWN article] for an
/// overview and the [`openat2` documentation] for details.
///
/// [LWN article]: https://lwn.net/Articles/796868/
/// [`openat2` documentation]: https://man7.org/linux/man-pages/man2/openat2.2.html
///
/// On older Linux, fall back to `manually::open`.
pub(crate) fn open_fast(
    start: &fs::File,
    path: &Path,
    options: &OpenOptions,
) -> io::Result<Option<fs::File>> {
    // On regular Linux, attempt to use `openat2` to accelerate sandboxed
    // lookups. On Android, the [seccomp policy] prevents us from even
    // detecting whether `openat2` is supported, so don't even try.
    //
    // [seccomp policy]: https://android-developers.googleblog.com/2017/07/seccomp-filter-in-android-o.html
    let result = open_beneath(start, path, options);

    // If we got anything other than a `ENOSYS` error, that's our result.
    match result {
        Err(err) if err.raw_os_error() == Some(rustix::io::Errno::NOSYS.raw_os_error()) => Ok(None),
        Err(err) => Err(err),
        Ok(fd) => Ok(Some(fd)),
    }
}

/// Call the `openat2` system call with `RESOLVE_BENEATH`. If the syscall is
/// unavailable, mark it so for future calls. If `openat2` is unavailable
/// either permanently or temporarily, return `ENOSYS`.
fn open_beneath(start: &fs::File, path: &Path, options: &OpenOptions) -> io::Result<fs::File> {
    static INVALID: AtomicBool = AtomicBool::new(false);
    if INVALID.load(Relaxed) {
        // `openat2` is permanently unavailable.
        return Err(rustix::io::Errno::NOSYS.into());
    }

    let oflags = compute_oflags(options)?;

    // Do two `contains` checks because `TMPFILE` may be represented with
    // multiple flags and we need to ensure they're all set.
    let mode = if oflags.contains(OFlags::CREATE) || oflags.contains(OFlags::TMPFILE) {
        Mode::from_bits((options.ext.mode & 0o7777) as RawMode).unwrap()
    } else {
        Mode::empty()
    };

    // We know `openat2` needs a `&CStr` internally; to avoid allocating on
    // each iteration of the loop below, allocate the `CString` now.
    path.into_with_c_str(|path_c_str| {
        // `openat2` fails with `EAGAIN` if a rename happens anywhere on the host
        // while it's running, so use a loop to retry it a few times. But not too many
        // times, because there's no limit on how often this can happen. The actual
        // number here is currently an arbitrarily chosen guess.
        for _ in 0..4 {
            match openat2(
                start,
                path_c_str,
                oflags,
                mode,
                ResolveFlags::BENEATH | ResolveFlags::NO_MAGICLINKS,
            ) {
                Ok(file) => {
                    let file = fs::File::from(file);

                    return Ok(file);
                }
                Err(err) => match err {
                    // A rename or similar happened. Try again.
                    rustix::io::Errno::AGAIN => continue,

                    // `EPERM` is used by some `seccomp` sandboxes to indicate
                    // that `openat2` is unimplemented:
                    // <https://github.com/systemd/systemd/blob/e2357b1c8a87b610066b8b2a59517bcfb20b832e/src/shared/seccomp-util.c#L2066>
                    //
                    // However, `EPERM` may also indicate a failed `O_NOATIME`
                    // or a file seal prevented the operation, and it's complex
                    // to detect those cases, so exit the loop and use the
                    // fallback.
                    rustix::io::Errno::PERM => break,

                    // `ENOSYS` means `openat2` is permanently unavailable;
                    // mark it so and exit the loop.
                    rustix::io::Errno::NOSYS => {
                        INVALID.store(true, Relaxed);
                        break;
                    }

                    _ => return Err(err),
                },
            }
        }

        Err(rustix::io::Errno::NOSYS)
    })
    .map_err(|err| match err {
        rustix::io::Errno::XDEV => errors::escape_attempt(),
        err => err.into(),
    })
}

// In theory we could optimize `link` using `openat2` with `O_PATH` and
// `linkat` with `AT_EMPTY_PATH`, however that requires `CAP_DAC_READ_SEARCH`,
// so it isn't very widely applicable.

/// Open the path with `O_PATH`. Use `read(true)` even though we don't need
/// `read` permissions, because Rust's libstd requires an access mode, and Linux
/// ignores `O_RDONLY` with `O_PATH`.
pub(crate) fn stat_fast(
    start: &fs::File,
    path: &Path,
    follow: FollowSymlinks,
) -> io::Result<Option<Metadata>> {
    let result = open_beneath(
        start,
        path,
        OpenOptions::new()
            .read(true)
            .follow(follow)
            .custom_flags(OFlags::PATH.bits() as i32),
    );

    match result {
        // Like `file.metadata()`, but works with `O_PATH` descriptors on old
        // (pre 3.6) versions of Linux too.
        Ok(file) => {
            // Record whether we've seen an `EBADF` from an `fstat` on an
            // `O_PATH` file descriptor, meaning we're on a Linux that doesn't
            // support it.
            static FSTAT_PATH_BADF: AtomicBool = AtomicBool::new(false);

            if !FSTAT_PATH_BADF.load(Relaxed) {
                match Metadata::from_file(&file) {
                    Ok(metadata) => return Ok(Some(metadata)),
                    Err(err) => match Errno::from_io_error(&err) {
                        // Before Linux 3.6, `fstat` with `O_PATH` returned
                        // `EBADF`.
                        Some(Errno::BADF) => FSTAT_PATH_BADF.store(true, Relaxed),
                        _ => return Err(err),
                    },
                }
            }

            // If `fstat` with `O_PATH` isn't supported, use `statat` with
            // `AT_EMPTY_PATH`.
            Ok(Some(
                statat(file, "", AtFlags::EMPTY_PATH).map(ImplMetadataExt::from_rustix)?,
            ))
        }
        Err(err) => match Errno::from_io_error(&err) {
            // `ENOSYS` from `open_beneath` means `openat2` is unavailable
            // and we should use a fallback.
            Some(Errno::NOSYS) => Ok(None),
            _ => Err(err),
        },
    }
}

pub(crate) fn set_times_fallback(
    start: &fs::File,
    path: &Path,
    atime: Option<SystemTime>,
    mtime: Option<SystemTime>,
) -> io::Result<bool> {
    let opath = open(
        start,
        path,
        OpenOptions::new()
            .read(true)
            .custom_flags(OFlags::PATH.bits() as i32),
    )?;

    let times = Timestamps {
        last_access: to_timespec(atime)?,
        last_modification: to_timespec(mtime)?,
    };

    // Don't pass `AT_SYMLINK_NOFOLLOW`, because we do actually want to follow
    // the first symlink. We don't want to follow any subsequent symlinks, but
    // omitting `O_NOFOLLOW` above ensures that the destination of the link
    // isn't a symlink.
    utimensat(
        proc_self_fd()?.as_fd(),
        DecInt::from_fd(&opath).as_ref(),
        &times,
        AtFlags::empty(),
    )?;
    Ok(true)
}
