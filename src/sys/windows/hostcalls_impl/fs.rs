#![allow(non_camel_case_types)]
#![allow(unused)]
use super::fs_helpers::*;
use crate::ctx::WasiCtx;
use crate::fdentry::FdEntry;
use crate::helpers::systemtime_to_timestamp;
use crate::sys::{errno_from_ioerror, errno_from_host};
use crate::sys::fdentry_impl::determine_type_rights;
use crate::sys::host_impl;
use crate::{host, Result};
use std::convert::TryInto;
use std::fs::{File, Metadata};
use std::io::{self, Seek, SeekFrom};
use std::os::windows::fs::FileExt;
use std::os::windows::prelude::{AsRawHandle, FromRawHandle};

fn read_at(mut file: &File, buf: &mut [u8], offset: u64) -> io::Result<usize> {
    // get current cursor position
    let cur_pos = file.seek(SeekFrom::Current(0))?;
    // perform a seek read by a specified offset
    let nread = file.seek_read(buf, offset)?;
    // rewind the cursor back to the original position
    file.seek(SeekFrom::Start(cur_pos))?;
    Ok(nread)
}

fn write_at(mut file: &File, buf: &[u8], offset: u64) -> io::Result<usize> {
    // get current cursor position
    let cur_pos = file.seek(SeekFrom::Current(0))?;
    // perform a seek write by a specified offset
    let nwritten = file.seek_write(buf, offset)?;
    // rewind the cursor back to the original position
    file.seek(SeekFrom::Start(cur_pos))?;
    Ok(nwritten)
}

pub(crate) fn fd_pread(
    file: &File,
    buf: &mut [u8],
    offset: host::__wasi_filesize_t,
) -> Result<usize> {
    read_at(file, buf, offset)
        .map_err(|err| err.raw_os_error().map_or(host::__WASI_EIO, errno_from_host))
}

pub(crate) fn fd_pwrite(file: &File, buf: &[u8], offset: host::__wasi_filesize_t) -> Result<usize> {
    write_at(file, buf, offset)
        .map_err(|err| err.raw_os_error().map_or(host::__WASI_EIO, errno_from_host))
}

pub(crate) fn fd_fdstat_get(fd: &File) -> Result<host::__wasi_fdflags_t> {
    use winx::file::AccessRight;
    match winx::file::get_file_access_rights(fd.as_raw_handle())
        .map(AccessRight::from_bits_truncate)
    {
        Ok(rights) => Ok(host_impl::fdflags_from_win(rights)),
        Err(e) => Err(host_impl::errno_from_win(e)),
    }
}

pub(crate) fn fd_fdstat_set_flags(fd: &File, fdflags: host::__wasi_fdflags_t) -> Result<()> {
    unimplemented!("fd_fdstat_set_flags")
}

pub(crate) fn fd_advise(
    file: &File,
    advice: host::__wasi_advice_t,
    offset: host::__wasi_filesize_t,
    len: host::__wasi_filesize_t,
) -> Result<()> {
    unimplemented!("fd_advise")
}

pub(crate) fn path_create_directory(dirfd: &File, path: &str) -> Result<()> {
    unimplemented!("path_create_directory")
}

pub(crate) fn path_link(
    old_dirfd: &File,
    new_dirfd: &File,
    old_path: &str,
    new_path: &str,
) -> Result<()> {
    unimplemented!("path_link")
}

pub(crate) fn path_open(
    ctx: &WasiCtx,
    dirfd: host::__wasi_fd_t,
    dirflags: host::__wasi_lookupflags_t,
    path: &str,
    oflags: host::__wasi_oflags_t,
    read: bool,
    write: bool,
    mut needed_base: host::__wasi_rights_t,
    mut needed_inheriting: host::__wasi_rights_t,
    fs_flags: host::__wasi_fdflags_t,
) -> Result<FdEntry> {
    use winx::file::{AccessRight, CreationDisposition, FlagsAndAttributes, ShareMode};

    let mut win_rights = AccessRight::READ_CONTROL;
    if read {
        win_rights.insert(AccessRight::FILE_GENERIC_READ);
    }
    if write {
        win_rights.insert(AccessRight::FILE_GENERIC_WRITE);
    }

    // convert open flags
    let (win_create_disp, mut win_flags_attrs) = host_impl::win_from_oflags(oflags);
    if win_create_disp == CreationDisposition::CREATE_NEW {
        needed_base |= host::__WASI_RIGHT_PATH_CREATE_FILE;
    } else if win_create_disp == CreationDisposition::CREATE_ALWAYS {
        needed_base |= host::__WASI_RIGHT_PATH_CREATE_FILE;
    } else if win_create_disp == CreationDisposition::TRUNCATE_EXISTING {
        needed_base |= host::__WASI_RIGHT_PATH_FILESTAT_SET_SIZE;
    }

    // convert file descriptor flags
    let win_fdflags_res = host_impl::win_from_fdflags(fs_flags);
    win_rights.insert(win_fdflags_res.0);
    win_flags_attrs.insert(win_fdflags_res.1);
    if win_rights.contains(AccessRight::SYNCHRONIZE) {
        needed_inheriting |= host::__WASI_RIGHT_FD_DATASYNC;
        needed_inheriting |= host::__WASI_RIGHT_FD_SYNC;
    }

    let dirfe = ctx.get_fd_entry(dirfd, needed_base, needed_inheriting)?;
    let dirfd = dirfe.fd_object.descriptor.as_file()?;
    let (dir, path) = path_get(
        dirfd,
        dirflags,
        path,
        !win_flags_attrs.contains(FlagsAndAttributes::FILE_FLAG_BACKUP_SEMANTICS),
    )?;

    let new_handle = winx::file::openat(
        dir.as_raw_handle(),
        path.as_str(),
        win_rights,
        win_create_disp,
        win_flags_attrs,
    )
    .map_err(host_impl::errno_from_win)?;

    // Determine the type of the new file descriptor and which rights contradict with this type
    let file = unsafe { File::from_raw_handle(new_handle) };
    determine_type_rights(&file).and_then(|(_ty, max_base, max_inheriting)| {
        FdEntry::from(file).map(|mut fe| {
            fe.rights_base &= max_base;
            fe.rights_inheriting &= max_inheriting;
            fe
        })
    })
}

pub(crate) fn fd_readdir(
    fd: &File,
    host_buf: &mut [u8],
    cookie: host::__wasi_dircookie_t,
) -> Result<usize> {
    unimplemented!("fd_readdir")
}

pub(crate) fn path_readlink(dirfd: &File, path: &str, buf: &mut [u8]) -> Result<usize> {
    unimplemented!("path_readlink")
}

pub(crate) fn path_rename(
    old_dirfd: &File,
    old_path: &str,
    new_dirfd: &File,
    new_path: &str,
) -> Result<()> {
    unimplemented!("path_rename")
}

pub(crate) fn num_hardlinks(file: &File, _metadata: &Metadata) -> io::Result<u64> {
    Ok(winx::file::get_fileinfo(file)?.nNumberOfLinks.into())
}

pub(crate) fn device_id(file: &File, _metadata: &Metadata) -> io::Result<u64> {
    Ok(winx::file::get_fileinfo(file)?.dwVolumeSerialNumber.into())
}

pub(crate) fn file_serial_no(file: &File, _metadata: &Metadata) -> io::Result<u64> {
    let info = winx::file::get_fileinfo(file)?;
    let high = info.nFileIndexHigh;
    let low = info.nFileIndexLow;
    let no = ((high as u64) << 32) | (low as u64);
    Ok(no)
}

pub(crate) fn change_time(file: &File, _metadata: &Metadata) -> io::Result<i64> {
    winx::file::change_time(file)
}

pub(crate) fn fd_filestat_get_impl(file: &std::fs::File) -> Result<host::__wasi_filestat_t> {
    let metadata = file.metadata().map_err(errno_from_ioerror)?;
    Ok(host::__wasi_filestat_t {
        st_dev: device_id(file, &metadata).map_err(errno_from_ioerror)?,
        st_ino: file_serial_no(file, &metadata).map_err(errno_from_ioerror)?,
        st_nlink: num_hardlinks(file, &metadata)
            .map_err(errno_from_ioerror)?
            .try_into()
            .map_err(|_| host::__WASI_EOVERFLOW)?, // u64 doesn't fit into u32
        st_size: metadata.len(),
        st_atim: metadata
            .accessed()
            .map_err(errno_from_ioerror)
            .and_then(systemtime_to_timestamp)?,
        st_ctim: change_time(file, &metadata)
            .map_err(errno_from_ioerror)?
            .try_into()
            .map_err(|_| host::__WASI_EOVERFLOW)?, // i64 doesn't fit into u64
        st_mtim: metadata
            .modified()
            .map_err(errno_from_ioerror)
            .and_then(systemtime_to_timestamp)?,
        st_filetype: filetype(&metadata).map_err(errno_from_ioerror)?,
    })
}

fn filetype(metadata: &Metadata) -> io::Result<host::__wasi_filetype_t> {
    let ftype = metadata.file_type();
    let ret = if ftype.is_file() {
        host::__WASI_FILETYPE_REGULAR_FILE
    } else if ftype.is_dir() {
        host::__WASI_FILETYPE_DIRECTORY
    } else if ftype.is_symlink() {
        host::__WASI_FILETYPE_SYMBOLIC_LINK
    } else {
        host::__WASI_FILETYPE_UNKNOWN
    };

    Ok(ret)
}

pub(crate) fn fd_filestat_set_times(
    fd: &File,
    st_atim: host::__wasi_timestamp_t,
    mut st_mtim: host::__wasi_timestamp_t,
    fst_flags: host::__wasi_fstflags_t,
) -> Result<()> {
    unimplemented!("fd_filestat_set_times")
}

pub(crate) fn fd_filestat_set_size(fd: &File, st_size: host::__wasi_filesize_t) -> Result<()> {
    unimplemented!("fd_filestat_set_size")
}

pub(crate) fn path_filestat_get(
    dirfd: &File,
    dirflags: host::__wasi_lookupflags_t,
    path: &str,
) -> Result<host::__wasi_filestat_t> {
    unimplemented!("path_filestat_get")
}

pub(crate) fn path_filestat_set_times(
    dirfd: &File,
    dirflags: host::__wasi_lookupflags_t,
    path: &str,
    st_atim: host::__wasi_timestamp_t,
    mut st_mtim: host::__wasi_timestamp_t,
    fst_flags: host::__wasi_fstflags_t,
) -> Result<()> {
    unimplemented!("path_filestat_set_times")
}

pub(crate) fn path_symlink(dirfd: &File, old_path: &str, new_path: &str) -> Result<()> {
    unimplemented!("path_symlink")
}

pub(crate) fn path_unlink_file(dirfd: &File, path: &str) -> Result<()> {
    unimplemented!("path_unlink_file")
}

pub(crate) fn path_remove_directory(dirfd: &File, path: &str) -> Result<()> {
    unimplemented!("path_remove_directory")
}
