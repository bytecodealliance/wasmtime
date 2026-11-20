use crate::filesystem::primitives::FileType;
use std::time::{Duration, SystemTime};
use std::{fs, io};

/// Metadata information about a file.
///
/// This corresponds to [`std::fs::Metadata`].
//
// NB: We need to define our own version because the libstd `Metadata` doesn't
// have a public constructor that we can use.
#[derive(Debug, Clone)]
pub(crate) enum Metadata {
    Stat(rustix::fs::Stat),
    #[cfg(target_os = "linux")]
    Statx(rustix::fs::Statx),
}

impl Metadata {
    /// Constructs a new instance of `Self` from the given [`std::fs::File`].
    #[inline]
    pub fn from_file(file: &fs::File) -> io::Result<Self> {
        cfg_select! {
            // Favor using `statx`, which `statat_metadata` uses internally,
            // over `fstat` on Linux to avoid issues with old kernels not liking
            // `fstat` on `O_PATH` descriptors.
            any(target_os = "linux", target_os = "android") => {
                super::stat_unchecked::statat_metadata(
                    file,
                    "".as_ref(),
                    rustix::fs::AtFlags::EMPTY_PATH,
                )
            }
            _ => Ok(Metadata::Stat(rustix::fs::fstat(file)?)),
        }
    }

    /// Returns the file type for this metadata.
    ///
    /// This corresponds to [`std::fs::Metadata::file_type`].
    #[inline]
    pub fn file_type(&self) -> FileType {
        match self {
            Metadata::Stat(m) => FileType::from_raw_mode(m.st_mode.into()),
            #[cfg(target_os = "linux")]
            Metadata::Statx(m) => FileType::from_raw_mode(m.stx_mode.into()),
        }
    }

    /// Returns `true` if this metadata is for a directory.
    ///
    /// This corresponds to [`std::fs::Metadata::is_dir`].
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.file_type().is_dir()
    }

    /// Returns the size of the file, in bytes, this metadata is for.
    ///
    /// This corresponds to [`std::fs::Metadata::len`].
    #[inline]
    pub fn len(&self) -> u64 {
        match self {
            Metadata::Stat(m) => u64::try_from(m.st_size).unwrap(),
            #[cfg(target_os = "linux")]
            Metadata::Statx(m) => m.stx_size,
        }
    }

    /// Returns the last modification time listed in this metadata.
    ///
    /// This corresponds to [`std::fs::Metadata::modified`].
    ///
    /// Note that unlike libstd, which treats `statx` as a drop-in replacement
    /// for `stat`, this and the other timestamp accessors respect `stx_mask`.
    /// When a filesystem can't supply a timestamp the kernel clears its bit
    /// and fills in a dummy value (for example the access time on filesystems
    /// with `SB_NOATIME` such as SMB), so an error is returned here instead of
    /// reporting the dummy value.
    #[inline]
    pub fn modified(&self) -> io::Result<SystemTime> {
        match self {
            Metadata::Stat(m) => {
                system_time_from_rustix(m.st_mtime, m.st_mtime_nsec.try_into().unwrap())
                    .ok_or_else(time_unavailable)
            }
            #[cfg(target_os = "linux")]
            Metadata::Statx(m) => {
                if m.stx_mask & rustix::fs::StatxFlags::MTIME.bits() != 0 {
                    system_time_from_rustix(m.stx_mtime.tv_sec, m.stx_mtime.tv_nsec.into())
                        .ok_or_else(time_unavailable)
                } else {
                    Err(time_unavailable())
                }
            }
        }
    }

    /// Returns the last access time of this metadata.
    ///
    /// This corresponds to [`std::fs::Metadata::accessed`].
    #[inline]
    pub fn accessed(&self) -> io::Result<SystemTime> {
        match self {
            Metadata::Stat(m) => {
                system_time_from_rustix(m.st_atime, m.st_atime_nsec.try_into().unwrap())
                    .ok_or_else(time_unavailable)
            }
            #[cfg(target_os = "linux")]
            Metadata::Statx(m) => {
                if m.stx_mask & rustix::fs::StatxFlags::ATIME.bits() != 0 {
                    system_time_from_rustix(m.stx_atime.tv_sec, m.stx_atime.tv_nsec.into())
                        .ok_or_else(time_unavailable)
                } else {
                    Err(time_unavailable())
                }
            }
        }
    }

    /// Returns the creation time listed in this metadata.
    ///
    /// This corresponds to [`std::fs::Metadata::created`].
    #[inline]
    pub fn created(&self) -> io::Result<SystemTime> {
        match self {
            Metadata::Stat(m) => {
                cfg_select! {
                    any(
                        target_os = "freebsd",
                        target_os = "netbsd",
                        target_os = "openbsd",
                        target_vendor = "apple",
                    ) => {
                        system_time_from_rustix(
                            m.st_birthtime,
                            m.st_birthtime_nsec.try_into().unwrap(),
                        )
                        .ok_or_else(time_unavailable)
                    }
                    _ => {
                        // `stat.st_ctime` is the latest status change; we want
                        // the creation.
                        let _ = &m;
                        Err(time_unavailable())
                    }
                }
            }
            #[cfg(target_os = "linux")]
            Metadata::Statx(m) => {
                if m.stx_mask & rustix::fs::StatxFlags::BTIME.bits() != 0 {
                    system_time_from_rustix(m.stx_btime.tv_sec, m.stx_btime.tv_nsec.into())
                        .ok_or_else(time_unavailable)
                } else {
                    Err(time_unavailable())
                }
            }
        }
    }

    /// Returns the ID of the device containing the file.
    #[inline]
    fn dev(&self) -> u64 {
        match self {
            #[allow(unused_comparisons, reason = "platform-specific typedef")]
            Metadata::Stat(m) => {
                // The type of `st_dev` is `dev_t` which is signed on some
                // platforms and unsigned on other platforms. A `u64` is enough
                // to work for all unsigned platforms, and for signed platforms
                // perform a sign extension to `i64` and then view that as an
                // unsigned 64-bit number instead.
                //
                // Note that the `unused_comparisons` is ignored here for
                // platforms where it's unsigned since the first branch here
                // will never be taken.
                if m.st_dev < 0 {
                    i64::try_from(m.st_dev).unwrap().cast_unsigned()
                } else {
                    u64::try_from(m.st_dev).unwrap()
                }
            }
            #[cfg(target_os = "linux")]
            Metadata::Statx(m) => rustix::fs::makedev(m.stx_dev_major, m.stx_dev_minor),
        }
    }

    /// Returns the inode number of the file.
    #[inline]
    fn ino(&self) -> u64 {
        match self {
            Metadata::Stat(m) => m.st_ino,
            #[cfg(target_os = "linux")]
            Metadata::Statx(m) => m.stx_ino,
        }
    }

    /// Returns the number of hard links to the file this metadata is for.
    #[inline]
    pub fn nlink(&self) -> u64 {
        match self {
            Metadata::Stat(m) => m.st_nlink.into(),
            #[cfg(target_os = "linux")]
            Metadata::Statx(m) => m.stx_nlink.into(),
        }
    }

    /// Returns a value which uniquely identifies the file this metadata is
    /// for on this host: its device and inode numbers.
    #[inline]
    pub fn identity(&self) -> (u64, u64) {
        (self.dev(), self.ino())
    }
}

fn system_time_from_rustix(sec: i64, nsec: u64) -> Option<SystemTime> {
    if sec >= 0 {
        SystemTime::UNIX_EPOCH.checked_add(Duration::new(u64::try_from(sec).unwrap(), nsec as _))
    } else {
        SystemTime::UNIX_EPOCH
            .checked_sub(Duration::new(sec.unsigned_abs(), 0))
            .map(|t| t.checked_add(Duration::new(0, nsec as u32)))
            .flatten()
    }
}

fn time_unavailable() -> io::Error {
    io::Error::new(
        io::ErrorKind::Other,
        "time metadata not available on this platform",
    )
}

/// It should be possible to represent times before the Epoch.
/// https://github.com/bytecodealliance/cap-std/issues/328
#[test]
fn negative_time() {
    let system_time = system_time_from_rustix(-1, 1).unwrap();
    let d = SystemTime::UNIX_EPOCH.duration_since(system_time).unwrap();
    assert_eq!(d.as_secs(), 0);
    assert_eq!(d.subsec_nanos(), 999999999);
}
