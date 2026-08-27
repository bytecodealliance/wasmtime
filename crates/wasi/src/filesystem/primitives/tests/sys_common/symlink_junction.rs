// Implementation derived from `symlink_junction` and related code in Rust's
// library/std/src/sys/windows/fs.rs at revision
// 3ffb27ff89db780e88abe829783565a7122be1c5.

#[cfg(feature = "fs_utf8")]
use camino::Utf8Path;
use cap_std::fs::Dir;
use std::io;
use std::path::Path;

#[cfg(not(windows))]
#[allow(dead_code)]
pub fn symlink_junction<P: AsRef<Path>, Q: AsRef<Path>>(
    src: P,
    dst_dir: &Dir,
    dst: Q,
) -> io::Result<()> {
    dst_dir.symlink(src, dst)
}

#[cfg(windows)]
#[allow(dead_code)]
pub fn symlink_junction<P: AsRef<Path>, Q: AsRef<Path>>(
    src: P,
    dst_dir: &Dir,
    dst: Q,
) -> io::Result<()> {
    symlink_junction_inner(src.as_ref(), dst_dir, dst.as_ref())
}

#[cfg(feature = "fs_utf8")]
#[cfg(not(windows))]
#[allow(dead_code)]
pub fn symlink_junction_utf8<P: AsRef<Utf8Path>, Q: AsRef<Utf8Path>>(
    src: P,
    dst_dir: &cap_std::fs_utf8::Dir,
    dst: Q,
) -> io::Result<()> {
    dst_dir.symlink(src, dst)
}

#[cfg(feature = "fs_utf8")]
#[cfg(windows)]
#[allow(dead_code)]
pub fn symlink_junction_utf8<P: AsRef<Utf8Path>, Q: AsRef<Utf8Path>>(
    src: P,
    dst_dir: &cap_std::fs_utf8::Dir,
    dst: Q,
) -> io::Result<()> {
    symlink_junction_inner_utf8(src.as_ref(), dst_dir, dst.as_ref())
}

/// Align the inner value to 8 bytes.
///
/// This is enough for almost all of the buffers we're likely to work with in
/// the Windows APIs we use.
#[cfg(windows)]
#[repr(C, align(8))]
#[derive(Copy, Clone)]
struct Align8<T: ?Sized>(pub T);

#[cfg(windows)]
#[allow(dead_code)]
#[allow(non_snake_case)]
#[repr(C)]
pub struct REPARSE_MOUNTPOINT_DATA_BUFFER {
    pub ReparseTag: u32,
    pub ReparseDataLength: u32,
    pub Reserved: u16,
    pub ReparseTargetLength: u16,
    pub ReparseTargetMaximumLength: u16,
    pub Reserved1: u16,
    pub ReparseTarget: libc::wchar_t,
}

#[cfg(windows)]
#[allow(dead_code)]
pub fn cvt(i: windows_sys::core::BOOL) -> io::Result<windows_sys::core::BOOL> {
    if i == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(i)
    }
}

// Creating a directory junction on windows involves dealing with reparse
// points and the `DeviceIoControl` function, and this code is a skeleton of
// what can be found here:
//
// http://www.flexhex.com/docs/articles/hard-links.phtml
#[cfg(windows)]
#[allow(dead_code)]
fn symlink_junction_inner(original: &Path, dir: &Dir, junction: &Path) -> io::Result<()> {
    use cap_std::fs::{OpenOptions, OpenOptionsExt};
    use std::mem::MaybeUninit;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::AsRawHandle;
    use std::{mem, ptr};
    use windows_sys::Win32::Storage::FileSystem::MAXIMUM_REPARSE_DATA_BUFFER_SIZE;

    dir.create_dir(junction)?;

    let mut opts = OpenOptions::new();
    opts.write(true);
    opts.custom_flags(
        windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT
            | windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS,
    );
    let f = dir.open_with(junction, &opts)?;
    let h = f.as_raw_handle();
    unsafe {
        let mut data =
            Align8([MaybeUninit::<u8>::uninit(); MAXIMUM_REPARSE_DATA_BUFFER_SIZE as usize]);
        let data_ptr = data.0.as_mut_ptr();
        let data_end = data_ptr.add(MAXIMUM_REPARSE_DATA_BUFFER_SIZE as usize);
        let db = data_ptr.cast::<REPARSE_MOUNTPOINT_DATA_BUFFER>();
        // Zero the header to ensure it's fully initialized, including reserved parameters.
        *db = mem::zeroed();
        let reparse_target_slice = {
            let buf_start = ptr::addr_of_mut!((*db).ReparseTarget).cast::<libc::wchar_t>();
            // Compute offset in bytes and then divide so that we round down
            // rather than hit any UB (admittedly this arithmetic should work
            // out so that this isn't necessary)
            // TODO: use `byte_offfset_from` when pointer_byte_offsets is stable
            // let buf_len_bytes = usize::try_from(data_end.byte_offset_from(buf_start)).unwrap();
            let buf_len_bytes =
                usize::try_from(data_end.cast::<u8>().offset_from(buf_start.cast::<u8>())).unwrap();
            let buf_len_wchars = buf_len_bytes / core::mem::size_of::<libc::wchar_t>();
            core::slice::from_raw_parts_mut(buf_start, buf_len_wchars)
        };
        // FIXME: this conversion is very hacky
        let iter = br"\??\"
            .iter()
            .map(|x| *x as u16)
            .chain(original.as_os_str().encode_wide())
            .chain(core::iter::once(0));
        let mut i = 0;
        for c in iter {
            if i >= reparse_target_slice.len() {
                return Err(io::Error::new(
                    // TODO: use io::ErrorKind::InvalidFilename when io_error_more is stabilized
                    io::ErrorKind::Other,
                    "Input filename is too long",
                ));
            }
            reparse_target_slice[i] = c;
            i += 1;
        }
        (*db).ReparseTag = windows_sys::Win32::System::SystemServices::IO_REPARSE_TAG_MOUNT_POINT;
        (*db).ReparseTargetMaximumLength = (i * 2) as u16;
        (*db).ReparseTargetLength = ((i - 1) * 2) as u16;
        (*db).ReparseDataLength = (*db).ReparseTargetLength as u32 + 12;

        let mut ret = 0;
        cvt(windows_sys::Win32::System::IO::DeviceIoControl(
            h as _,
            windows_sys::Win32::System::Ioctl::FSCTL_SET_REPARSE_POINT,
            data_ptr.cast(),
            (*db).ReparseDataLength + 8,
            ptr::null_mut(),
            0,
            &mut ret,
            ptr::null_mut(),
        ))
        .map(drop)
    }
}

/// Same as above, but for fs_utf8.
#[cfg(feature = "fs_utf8")]
#[cfg(windows)]
#[allow(dead_code)]
fn symlink_junction_inner_utf8(
    original: &Utf8Path,
    dir: &cap_std::fs_utf8::Dir,
    junction: &Utf8Path,
) -> io::Result<()> {
    use cap_std::fs::{OpenOptions, OpenOptionsExt};
    use std::mem::MaybeUninit;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::AsRawHandle;
    use std::{mem, ptr};
    use windows_sys::Win32::Storage::FileSystem::MAXIMUM_REPARSE_DATA_BUFFER_SIZE;

    dir.create_dir(junction)?;

    let mut opts = OpenOptions::new();
    opts.write(true);
    opts.custom_flags(
        windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT
            | windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS,
    );
    let f = dir.open_with(junction, &opts)?;
    let h = f.as_raw_handle();
    unsafe {
        let mut data =
            Align8([MaybeUninit::<u8>::uninit(); MAXIMUM_REPARSE_DATA_BUFFER_SIZE as usize]);
        let data_ptr = data.0.as_mut_ptr();
        let data_end = data_ptr.add(MAXIMUM_REPARSE_DATA_BUFFER_SIZE as usize);
        let db = data_ptr.cast::<REPARSE_MOUNTPOINT_DATA_BUFFER>();
        // Zero the header to ensure it's fully initialized, including reserved parameters.
        *db = mem::zeroed();
        let reparse_target_slice = {
            let buf_start = ptr::addr_of_mut!((*db).ReparseTarget).cast::<libc::wchar_t>();
            // Compute offset in bytes and then divide so that we round down
            // rather than hit any UB (admittedly this arithmetic should work
            // out so that this isn't necessary)
            // TODO: use `byte_offfset_from` when pointer_byte_offsets is stable
            // let buf_len_bytes = usize::try_from(data_end.byte_offset_from(buf_start)).unwrap();
            let buf_len_bytes =
                usize::try_from(data_end.cast::<u8>().offset_from(buf_start.cast::<u8>())).unwrap();
            let buf_len_wchars = buf_len_bytes / core::mem::size_of::<libc::wchar_t>();
            core::slice::from_raw_parts_mut(buf_start, buf_len_wchars)
        };
        // FIXME: this conversion is very hacky
        let iter = br"\??\"
            .iter()
            .map(|x| *x as u16)
            .chain(original.as_os_str().encode_wide())
            .chain(core::iter::once(0));
        let mut i = 0;
        for c in iter {
            if i >= reparse_target_slice.len() {
                return Err(io::Error::new(
                    // TODO: use io::ErrorKind::InvalidFilename when io_error_more is stabilized
                    io::ErrorKind::Other,
                    "Input filename is too long",
                ));
            }
            reparse_target_slice[i] = c;
            i += 1;
        }
        (*db).ReparseTag = windows_sys::Win32::System::SystemServices::IO_REPARSE_TAG_MOUNT_POINT;
        (*db).ReparseTargetMaximumLength = (i * 2) as u16;
        (*db).ReparseTargetLength = ((i - 1) * 2) as u16;
        (*db).ReparseDataLength = (*db).ReparseTargetLength as u32 + 12;

        let mut ret = 0;
        cvt(windows_sys::Win32::System::IO::DeviceIoControl(
            h as _,
            windows_sys::Win32::System::Ioctl::FSCTL_SET_REPARSE_POINT,
            data_ptr.cast(),
            (*db).ReparseDataLength + 8,
            ptr::null_mut(),
            0,
            &mut ret,
            ptr::null_mut(),
        ))
        .map(drop)
    }
}
