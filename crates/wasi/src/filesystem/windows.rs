use crate::filesystem::{
    Advice, DescriptorFlags, DescriptorStat, DescriptorType, MetadataHashValue,
};
use cap_primitives::fs::{FileType, FollowSymlinks, Metadata, OpenOptions, OpenOptionsExt};
use std::fs::File;
use std::io::{self, Write};
use std::mem::{self, MaybeUninit};
use std::os::windows::fs::FileExt;
use std::os::windows::io::*;
use std::path::Path;
use std::sync::OnceLock;
use windows_sys::Wdk::Storage::FileSystem::*;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Storage::FileSystem::*;
use windows_sys::Win32::System::IO::*;

pub(crate) fn get_flags(file: &File) -> io::Result<DescriptorFlags> {
    let file = file.as_handle();
    let mode = query_mode_information(file)?;
    let mut ret = DescriptorFlags::empty();
    ret.set(
        DescriptorFlags::REQUESTED_WRITE_SYNC,
        mode & FILE_WRITE_THROUGH != 0,
    );
    Ok(ret)
}

pub(crate) fn advise(file: &File, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
    let _ = (file, offset, len, advice);

    // ... noop for now ...

    Ok(())
}

pub(crate) fn append_cursor_unspecified(file: &File, data: &[u8]) -> io::Result<usize> {
    let file = file.as_handle();
    let access = query_access_information(file)?;

    // If this file doesn't allow writing then it can't be appended to.
    if access & (FILE_WRITE_DATA | FILE_APPEND_DATA) == 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "file not opened with write or append access",
        ));
    }

    // Reopen the file with append
    reopen_file(
        file,
        FILE_GENERIC_WRITE & !FILE_WRITE_DATA,
        // Files on Windows are opened with DELETE, READ, and WRITE share mode
        // by default (see OpenOptions in stdlib) This keeps the same share mode
        // when reopening the file handle
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        0,
    )?
    .write(data)
}

pub(crate) fn write_at_cursor_unspecified(file: &File, data: &[u8], pos: u64) -> io::Result<usize> {
    file.seek_write(data, pos)
}

pub(crate) fn read_at_cursor_unspecified(
    file: &File,
    buf: &mut [u8],
    pos: u64,
) -> io::Result<usize> {
    file.seek_read(buf, pos)
}

fn by_handle_info(file: &File) -> io::Result<BY_HANDLE_FILE_INFORMATION> {
    unsafe {
        let mut info = mem::zeroed::<BY_HANDLE_FILE_INFORMATION>();
        if GetFileInformationByHandle(file.as_raw_handle(), &mut info) == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(info)
    }
}

fn file_identity(file: &File) -> io::Result<(u64, u64)> {
    let info = by_handle_info(file)?;
    Ok((
        u64::from(info.dwVolumeSerialNumber),
        (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
    ))
}

pub(crate) fn metadata_hash(file: &File) -> io::Result<MetadataHashValue> {
    Ok(MetadataHashValue::new(file_identity(file)?))
}

pub(crate) fn metadata_hash_at(
    start: &File,
    path: &Path,
    follow: FollowSymlinks,
) -> io::Result<MetadataHashValue> {
    let file = open_metadata_handle(start, path, follow)?;
    metadata_hash(&file)
}

pub(crate) fn is_same_file(a: &File, b: &File) -> io::Result<bool> {
    Ok(file_identity(a)? == file_identity(b)?)
}

/// Opens `path` relative to `start` for metadata queries only, mirroring
/// what `cap-primitives`' Windows `stat` does internally: no access rights
/// are requested, `FILE_FLAG_BACKUP_SEMANTICS` permits opening directories,
/// and for `FollowSymlinks::No` the trailing symlink itself is opened via
/// `FILE_FLAG_OPEN_REPARSE_POINT` (which `cap-primitives` documents as
/// suppressing its own trailing-symlink handling).
fn open_metadata_handle(start: &File, path: &Path, follow: FollowSymlinks) -> io::Result<File> {
    let mut opts = OpenOptions::new();
    opts.access_mode(0);
    match follow {
        FollowSymlinks::Yes => {
            opts.custom_flags(FILE_FLAG_BACKUP_SEMANTICS);
        }
        FollowSymlinks::No => {
            opts.custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
        }
    }
    cap_primitives::fs::open(start, path, &opts)
}

pub(crate) fn stat(f: &std::fs::File) -> io::Result<DescriptorStat> {
    let meta = Metadata::from_file(f)?;

    // Note that this is intentionally scoped to a separate block to
    // minimize the surface area that is depended on by cap-fs-ext.
    let link_count = {
        use cap_fs_ext_avoid_using_this::MetadataExt;
        meta.nlink()
    };
    Ok(DescriptorStat::new(&meta, link_count))
}

pub(crate) fn stat_at(
    start: &File,
    path: &Path,
    follow: FollowSymlinks,
) -> io::Result<DescriptorStat> {
    let file = open_metadata_handle(start, path, follow)?;
    stat(&file)
}

pub(crate) fn maybe_dir(opts: &mut OpenOptions) {
    opts.custom_flags(FILE_FLAG_BACKUP_SEMANTICS);
    opts.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE);
}

pub(crate) fn descriptor_type(ft: FileType) -> DescriptorType {
    if is_char_device(ft) {
        DescriptorType::CharacterDevice
    } else {
        DescriptorType::Unknown
    }
}

/// Returns whether `ft` is a character device.
///
/// This is a bit of a hack around the lack of documented/public API in
/// `cap-primitives` for exposing this information. The `NUL` file is always a
/// character-device, so its type is cached globally once and then used to
/// compare.
fn is_char_device(ft: FileType) -> bool {
    static CHAR_DEVICE: OnceLock<Option<FileType>> = OnceLock::new();
    let probe = CHAR_DEVICE.get_or_init(|| {
        let nul = File::open("NUL").ok()?;
        let file_type = Metadata::from_file(&nul).ok()?.file_type();
        if file_type.is_file() || file_type.is_dir() || file_type.is_symlink() {
            return None;
        }
        Some(file_type)
    });
    *probe == Some(ft)
}

pub(crate) fn symlink(original: &Path, start: &File, link: &Path) -> io::Result<()> {
    if cap_primitives::fs::stat(start, original, FollowSymlinks::Yes)?.is_dir() {
        cap_primitives::fs::symlink_dir(original, start, link)
    } else {
        cap_primitives::fs::symlink_file(original, start, link)
    }
}

pub(crate) fn remove_file_or_symlink(start: &File, path: &Path) -> io::Result<()> {
    // Note that `FILE_FLAG_OPEN_REPARSE_POINT` here means a trailing symlink
    // is opened as the reparse point itself rather than followed, so no
    // nofollow option is needed.
    let mut opts = OpenOptions::new();
    opts.access_mode(DELETE);
    opts.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS);
    let file = cap_primitives::fs::open(start, path, &opts)?;

    let meta = Metadata::from_file(&file)?;
    if meta.file_type().is_symlink()
        && cap_primitives::fs::MetadataExt::file_attributes(&meta) & FILE_ATTRIBUTE_DIRECTORY
            == FILE_ATTRIBUTE_DIRECTORY
    {
        cap_primitives::fs::remove_dir(start, path)?;
    } else {
        cap_primitives::fs::remove_file(start, path)?;
    }

    // Drop the file after calling `remove_file` or `remove_dir`, since
    // Windows doesn't actually remove the file until after the last open
    // handle is closed, and this protects us from race conditions where
    // other processes replace the file out from underneath us.
    drop(file);

    Ok(())
}

fn query_access_information(handle: BorrowedHandle<'_>) -> io::Result<u32> {
    unsafe {
        Ok(
            nt_query_information_file::<FILE_ACCESS_INFORMATION>(handle, FileAccessInformation)?
                .AccessFlags,
        )
    }
}

fn reopen_file(
    handle: BorrowedHandle<'_>,
    access_mode: u32,
    share_mode: u32,
    flags: u32,
) -> io::Result<File> {
    let new_handle = unsafe { ReOpenFile(handle.as_raw_handle(), access_mode, share_mode, flags) };

    if new_handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }

    Ok(unsafe { File::from_raw_handle(new_handle) })
}

fn query_mode_information(handle: BorrowedHandle<'_>) -> io::Result<u32> {
    unsafe {
        Ok(nt_query_information_file::<FILE_MODE_INFORMATION>(handle, FileModeInformation)?.Mode)
    }
}

unsafe fn nt_query_information_file<T>(
    handle: BorrowedHandle<'_>,
    info: FILE_INFORMATION_CLASS,
) -> io::Result<T> {
    unsafe {
        let mut io_status_block = mem::zeroed::<IO_STATUS_BLOCK>();
        let mut payload = MaybeUninit::<T>::uninit();

        let status = NtQueryInformationFile(
            handle.as_raw_handle(),
            &mut io_status_block,
            payload.as_mut_ptr().cast(),
            mem::size_of_val(&payload).try_into().unwrap(),
            info,
        );

        if status != STATUS_SUCCESS {
            return Err(io::Error::from_raw_os_error(
                RtlNtStatusToDosError(status) as i32
            ));
        }

        Ok(payload.assume_init())
    }
}
