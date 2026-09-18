use crate::filesystem::primitives::FileType;
use std::time::SystemTime;
use std::{fs, io};

/// Metadata information about a file.
///
/// This corresponds to [`std::fs::Metadata`].
///
/// <details>
/// We need to define our own version because the libstd `Metadata` doesn't
/// have a public constructor that we can use.
/// </details>
#[derive(Debug, Clone)]
pub(crate) enum Metadata {
    Std(fs::Metadata),
    #[cfg(unix)]
    Stat(rustix::fs::Stat),
    #[cfg(target_os = "linux")]
    Statx(rustix::fs::Statx),
}

impl From<fs::Metadata> for Metadata {
    fn from(std: fs::Metadata) -> Self {
        Self::Std(std)
    }
}

impl Metadata {
    /// Constructs a new instance of `Self` from the given [`std::fs::File`].
    #[inline]
    pub fn from_file(file: &fs::File) -> io::Result<Self> {
        file.metadata().map(Metadata::Std)
    }

    /// Returns the file type for this metadata.
    ///
    /// This corresponds to [`std::fs::Metadata::file_type`].
    #[inline]
    pub fn file_type(&self) -> FileType {
        match self {
            Metadata::Std(m) => FileType::from(m.file_type()),
            #[cfg(unix)]
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
            Metadata::Std(m) => m.len(),
            #[cfg(unix)]
            Metadata::Stat(m) => u64::try_from(m.st_size).unwrap(),
            #[cfg(target_os = "linux")]
            Metadata::Statx(m) => m.stx_size,
        }
    }

    /// Returns the last modification time listed in this metadata.
    ///
    /// This corresponds to [`std::fs::Metadata::modified`].
    #[inline]
    pub fn modified(&self) -> io::Result<SystemTime> {
        match self {
            Metadata::Std(m) => m.modified(),
            #[cfg(unix)]
            Metadata::Stat(m) => super::unix::system_time_from_rustix(
                m.st_mtime,
                m.st_mtime_nsec.try_into().unwrap(),
            )
            .ok_or_else(time_unavailable),
            #[cfg(target_os = "linux")]
            Metadata::Statx(m) => {
                if m.stx_mask & rustix::fs::StatxFlags::MTIME.bits() != 0 {
                    super::unix::system_time_from_rustix(
                        m.stx_mtime.tv_sec,
                        m.stx_mtime.tv_nsec.into(),
                    )
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
            Metadata::Std(m) => m.accessed(),
            #[cfg(unix)]
            Metadata::Stat(m) => super::unix::system_time_from_rustix(
                m.st_atime,
                m.st_atime_nsec.try_into().unwrap(),
            )
            .ok_or_else(time_unavailable),
            #[cfg(target_os = "linux")]
            Metadata::Statx(m) => {
                if m.stx_mask & rustix::fs::StatxFlags::ATIME.bits() != 0 {
                    super::unix::system_time_from_rustix(
                        m.stx_atime.tv_sec,
                        m.stx_atime.tv_nsec.into(),
                    )
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
            Metadata::Std(m) => m.created(),
            #[cfg(unix)]
            Metadata::Stat(m) => {
                cfg_select! {
                    any(target_os = "freebsd", target_vendor = "apple") => {
                        super::unix::system_time_from_rustix(
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
                    super::unix::system_time_from_rustix(
                        m.stx_btime.tv_sec,
                        m.stx_btime.tv_nsec.into(),
                    )
                    .ok_or_else(time_unavailable)
                } else {
                    Err(time_unavailable())
                }
            }
        }
    }
}

#[cfg(unix)]
fn time_unavailable() -> io::Error {
    io::Error::new(
        io::ErrorKind::Other,
        "time metadata not available on this platform",
    )
}
