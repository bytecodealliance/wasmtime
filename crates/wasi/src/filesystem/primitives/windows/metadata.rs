use crate::filesystem::primitives::FileType;
use std::os::windows::io::AsRawHandle;
use std::time::{Duration, SystemTime};
use std::{fmt, fs, io, mem};
use windows_sys::Win32::Foundation::FILETIME;
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO,
    FileAttributeTagInfo, GetFileInformationByHandle, GetFileInformationByHandleEx,
};

/// Metadata information about a file.
///
/// This corresponds to [`std::fs::Metadata`].
//
// The `std::fs::Metadata` type doesn't have any public constructors, and its
// public API surface are at this time additionally doesn't support everything
// we want on Windows. Until more functions/such have settled in the standard
// library this needs to duplicate the representation and manually redo what the
// standard library already handles.
#[derive(Clone)]
pub(crate) struct Metadata {
    info: BY_HANDLE_FILE_INFORMATION,
    /// The reparse tag of the file, or 0 if it's not a reparse point.
    reparse_tag: u32,
}

impl Metadata {
    /// Constructs a new instance of `Self` from the given [`std::fs::File`].
    ///
    /// This mirrors what `std::fs::File::metadata` does.
    pub fn from_file(file: &fs::File) -> io::Result<Self> {
        unsafe {
            let mut info = mem::zeroed::<BY_HANDLE_FILE_INFORMATION>();
            if GetFileInformationByHandle(file.as_raw_handle(), &mut info) == 0 {
                return Err(io::Error::last_os_error());
            }
            let mut reparse_tag = 0;
            if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                let mut tag_info = mem::zeroed::<FILE_ATTRIBUTE_TAG_INFO>();
                if GetFileInformationByHandleEx(
                    file.as_raw_handle(),
                    FileAttributeTagInfo,
                    (&raw mut tag_info).cast(),
                    mem::size_of_val(&tag_info).try_into().unwrap(),
                ) == 0
                {
                    return Err(io::Error::last_os_error());
                }
                if tag_info.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                    reparse_tag = tag_info.ReparseTag;
                }
            }
            Ok(Metadata { info, reparse_tag })
        }
    }

    /// Returns the file type for this metadata.
    ///
    /// This corresponds to [`std::fs::Metadata::file_type`].
    #[inline]
    pub fn file_type(&self) -> FileType {
        FileType::new(self.info.dwFileAttributes, self.reparse_tag)
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
        (u64::from(self.info.nFileSizeHigh) << 32) | u64::from(self.info.nFileSizeLow)
    }

    /// Returns the last modification time listed in this metadata.
    ///
    /// This corresponds to [`std::fs::Metadata::modified`].
    #[inline]
    pub fn modified(&self) -> io::Result<SystemTime> {
        system_time_from_filetime(self.info.ftLastWriteTime)
    }

    /// Returns the last access time of this metadata.
    ///
    /// This corresponds to [`std::fs::Metadata::accessed`].
    #[inline]
    pub fn accessed(&self) -> io::Result<SystemTime> {
        system_time_from_filetime(self.info.ftLastAccessTime)
    }

    /// Returns the creation time listed in this metadata.
    ///
    /// This corresponds to [`std::fs::Metadata::created`].
    #[inline]
    pub fn created(&self) -> io::Result<SystemTime> {
        system_time_from_filetime(self.info.ftCreationTime)
    }

    /// Returns the `FILE_ATTRIBUTE_*` flags of this file.
    #[inline]
    pub(crate) fn file_attributes(&self) -> u32 {
        self.info.dwFileAttributes
    }

    /// Returns the number of hard links to the file this metadata is for.
    #[inline]
    pub fn nlink(&self) -> u64 {
        self.info.nNumberOfLinks.into()
    }

    /// Returns a value which uniquely identifies the file this metadata is
    /// for on this host: its volume serial number and file index.
    #[inline]
    pub fn identity(&self) -> (u64, u64) {
        let file_index =
            (u64::from(self.info.nFileIndexHigh) << 32) | u64::from(self.info.nFileIndexLow);
        (self.info.dwVolumeSerialNumber.into(), file_index)
    }
}

/// Converts a `FILETIME`, the number of 100ns intervals since January 1,
/// 1601, to a `SystemTime`.
fn system_time_from_filetime(ft: FILETIME) -> io::Result<SystemTime> {
    const INTERVALS_PER_SEC: u64 = 10_000_000;
    const INTERVALS_TO_UNIX_EPOCH: u64 = 11_644_473_600 * INTERVALS_PER_SEC;

    let intervals = (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime);
    let to_duration = |intervals: u64| {
        Duration::new(
            intervals / INTERVALS_PER_SEC,
            ((intervals % INTERVALS_PER_SEC) * 100).try_into().unwrap(),
        )
    };
    let time = match intervals.checked_sub(INTERVALS_TO_UNIX_EPOCH) {
        Some(after) => SystemTime::UNIX_EPOCH.checked_add(to_duration(after)),
        None => {
            SystemTime::UNIX_EPOCH.checked_sub(to_duration(INTERVALS_TO_UNIX_EPOCH - intervals))
        }
    };
    time.ok_or_else(|| io::Error::new(io::ErrorKind::Other, "file time out of range"))
}

impl fmt::Debug for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Metadata")
            .field("file_attributes", &self.file_attributes())
            .field("reparse_tag", &self.reparse_tag)
            .field("len", &self.len())
            .field("modified", &self.modified())
            .field("accessed", &self.accessed())
            .field("created", &self.created())
            .field("nlink", &self.nlink())
            .field("identity", &self.identity())
            .finish()
    }
}

#[test]
fn filetime_conversion() {
    let ft = |intervals: u64| FILETIME {
        dwLowDateTime: intervals as u32,
        dwHighDateTime: (intervals >> 32) as u32,
    };
    let epoch = 116_444_736_000_000_000;
    assert_eq!(
        system_time_from_filetime(ft(epoch)).unwrap(),
        SystemTime::UNIX_EPOCH
    );
    assert_eq!(
        system_time_from_filetime(ft(epoch + 15)).unwrap(),
        SystemTime::UNIX_EPOCH + Duration::new(0, 1500)
    );
    assert_eq!(
        system_time_from_filetime(ft(epoch - 1)).unwrap(),
        SystemTime::UNIX_EPOCH - Duration::new(0, 100)
    );
}
