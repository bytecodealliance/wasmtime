#![allow(non_camel_case_types)]
use super::fs_helpers::path_get;
use crate::ctx::WasiCtx;
use crate::fdentry::{Descriptor, FdEntry};
use crate::helpers::*;
use crate::host::Dirent;
use crate::memory::*;
use crate::sandboxed_tty_writer::SandboxedTTYWriter;
use crate::sys::hostcalls_impl::fs_helpers::path_open_rights;
use crate::sys::{host_impl, hostcalls_impl};
use crate::wasi::{self, WasiError, WasiResult};
use crate::{helpers, host, wasi32};
use filetime::{set_file_handle_times, FileTime};
use log::trace;
use std::convert::TryInto;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::ops::DerefMut;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(crate) unsafe fn fd_close(
    wasi_ctx: &mut WasiCtx,
    _memory: &mut [u8],
    fd: wasi::__wasi_fd_t,
) -> WasiResult<()> {
    trace!("fd_close(fd={:?})", fd);

    if let Ok(fe) = wasi_ctx.get_fd_entry(fd) {
        // can't close preopened files
        if fe.preopen_path.is_some() {
            return Err(WasiError::ENOTSUP);
        }
    }

    wasi_ctx.remove_fd_entry(fd)?;
    Ok(())
}

pub(crate) unsafe fn fd_datasync(
    wasi_ctx: &WasiCtx,
    _memory: &mut [u8],
    fd: wasi::__wasi_fd_t,
) -> WasiResult<()> {
    trace!("fd_datasync(fd={:?})", fd);

    let file = wasi_ctx
        .get_fd_entry(fd)?
        .as_descriptor(wasi::__WASI_RIGHTS_FD_DATASYNC, 0)?;

    match file {
        Descriptor::OsHandle(fd) => fd.sync_data().map_err(Into::into),
        Descriptor::VirtualFile(virt) => virt.datasync(),
        other => other.as_os_handle().sync_data().map_err(Into::into),
    }
}

pub(crate) unsafe fn fd_pread(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasi::__wasi_fd_t,
    iovs_ptr: wasi32::uintptr_t,
    iovs_len: wasi32::size_t,
    offset: wasi::__wasi_filesize_t,
    nread: wasi32::uintptr_t,
) -> WasiResult<()> {
    trace!(
        "fd_pread(fd={:?}, iovs_ptr={:#x?}, iovs_len={:?}, offset={}, nread={:#x?})",
        fd,
        iovs_ptr,
        iovs_len,
        offset,
        nread
    );

    let file = wasi_ctx
        .get_fd_entry(fd)?
        .as_descriptor(wasi::__WASI_RIGHTS_FD_READ | wasi::__WASI_RIGHTS_FD_SEEK, 0)?
        .as_file()?;

    let iovs = dec_iovec_slice(memory, iovs_ptr, iovs_len)?;

    if offset > i64::max_value() as u64 {
        return Err(WasiError::EIO);
    }
    let buf_size = iovs
        .iter()
        .map(|iov| {
            let cast_iovlen: wasi32::size_t = iov
                .buf_len
                .try_into()
                .expect("iovec are bounded by wasi max sizes");
            cast_iovlen
        })
        .fold(Some(0u32), |len, iov| len.and_then(|x| x.checked_add(iov)))
        .ok_or(WasiError::EINVAL)?;
    let mut buf = vec![0; buf_size as usize];
    let host_nread = match file {
        Descriptor::OsHandle(fd) => hostcalls_impl::fd_pread(&fd, &mut buf, offset)?,
        Descriptor::VirtualFile(virt) => virt.pread(&mut buf, offset)?,
        _ => {
            unreachable!(
                "implementation error: fd should have been checked to not be a stream already"
            );
        }
    };

    let mut buf_offset = 0;
    let mut left = host_nread;
    for iov in &iovs {
        if left == 0 {
            break;
        }
        let vec_len = std::cmp::min(iov.buf_len, left);
        std::slice::from_raw_parts_mut(iov.buf as *mut u8, vec_len)
            .copy_from_slice(&buf[buf_offset..buf_offset + vec_len]);
        buf_offset += vec_len;
        left -= vec_len;
    }

    trace!("     | *nread={:?}", host_nread);

    enc_usize_byref(memory, nread, host_nread)
}

pub(crate) unsafe fn fd_pwrite(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasi::__wasi_fd_t,
    iovs_ptr: wasi32::uintptr_t,
    iovs_len: wasi32::size_t,
    offset: wasi::__wasi_filesize_t,
    nwritten: wasi32::uintptr_t,
) -> WasiResult<()> {
    trace!(
        "fd_pwrite(fd={:?}, iovs_ptr={:#x?}, iovs_len={:?}, offset={}, nwritten={:#x?})",
        fd,
        iovs_ptr,
        iovs_len,
        offset,
        nwritten
    );

    let file = wasi_ctx
        .get_fd_entry(fd)?
        .as_descriptor(
            wasi::__WASI_RIGHTS_FD_WRITE | wasi::__WASI_RIGHTS_FD_SEEK,
            0,
        )?
        .as_file()?;
    let iovs = dec_ciovec_slice(memory, iovs_ptr, iovs_len)?;

    if offset > i64::max_value() as u64 {
        return Err(WasiError::EIO);
    }
    let buf_size = iovs
        .iter()
        .map(|iov| {
            let cast_iovlen: wasi32::size_t = iov
                .buf_len
                .try_into()
                .expect("iovec are bounded by wasi max sizes");
            cast_iovlen
        })
        .fold(Some(0u32), |len, iov| len.and_then(|x| x.checked_add(iov)))
        .ok_or(WasiError::EINVAL)?;
    let mut buf = Vec::with_capacity(buf_size as usize);
    for iov in &iovs {
        buf.extend_from_slice(std::slice::from_raw_parts(
            iov.buf as *const u8,
            iov.buf_len,
        ));
    }
    let host_nwritten = match file {
        Descriptor::OsHandle(fd) => hostcalls_impl::fd_pwrite(&fd, &buf, offset)?,
        Descriptor::VirtualFile(virt) => virt.pwrite(buf.as_mut(), offset)?,
        _ => {
            unreachable!(
                "implementation error: fd should have been checked to not be a stream already"
            );
        }
    };

    trace!("     | *nwritten={:?}", host_nwritten);

    enc_usize_byref(memory, nwritten, host_nwritten)
}

pub(crate) unsafe fn fd_read(
    wasi_ctx: &mut WasiCtx,
    memory: &mut [u8],
    fd: wasi::__wasi_fd_t,
    iovs_ptr: wasi32::uintptr_t,
    iovs_len: wasi32::size_t,
    nread: wasi32::uintptr_t,
) -> WasiResult<()> {
    trace!(
        "fd_read(fd={:?}, iovs_ptr={:#x?}, iovs_len={:?}, nread={:#x?})",
        fd,
        iovs_ptr,
        iovs_len,
        nread
    );

    let mut iovs = dec_iovec_slice(memory, iovs_ptr, iovs_len)?;
    let mut iovs: Vec<io::IoSliceMut> = iovs
        .iter_mut()
        .map(|vec| host::iovec_to_host_mut(vec))
        .collect();

    let maybe_host_nread = match wasi_ctx
        .get_fd_entry_mut(fd)?
        .as_descriptor_mut(wasi::__WASI_RIGHTS_FD_READ, 0)?
    {
        Descriptor::OsHandle(file) => file.read_vectored(&mut iovs).map_err(Into::into),
        Descriptor::VirtualFile(virt) => virt.read_vectored(&mut iovs),
        Descriptor::Stdin => io::stdin().read_vectored(&mut iovs).map_err(Into::into),
        _ => return Err(WasiError::EBADF),
    };

    let host_nread = maybe_host_nread?;

    trace!("     | *nread={:?}", host_nread);

    enc_usize_byref(memory, nread, host_nread)
}

pub(crate) unsafe fn fd_renumber(
    wasi_ctx: &mut WasiCtx,
    _memory: &mut [u8],
    from: wasi::__wasi_fd_t,
    to: wasi::__wasi_fd_t,
) -> WasiResult<()> {
    trace!("fd_renumber(from={:?}, to={:?})", from, to);

    if !wasi_ctx.contains_fd_entry(from) {
        return Err(WasiError::EBADF);
    }

    // Don't allow renumbering over a pre-opened resource.
    // TODO: Eventually, we do want to permit this, once libpreopen in
    // userspace is capable of removing entries from its tables as well.
    let from_fe = wasi_ctx.get_fd_entry(from)?;
    if from_fe.preopen_path.is_some() {
        return Err(WasiError::ENOTSUP);
    }
    if let Ok(to_fe) = wasi_ctx.get_fd_entry(to) {
        if to_fe.preopen_path.is_some() {
            return Err(WasiError::ENOTSUP);
        }
    }

    let fe = wasi_ctx.remove_fd_entry(from)?;
    wasi_ctx.insert_fd_entry_at(to, fe);

    Ok(())
}

pub(crate) unsafe fn fd_seek(
    wasi_ctx: &mut WasiCtx,
    memory: &mut [u8],
    fd: wasi::__wasi_fd_t,
    offset: wasi::__wasi_filedelta_t,
    whence: wasi::__wasi_whence_t,
    newoffset: wasi32::uintptr_t,
) -> WasiResult<()> {
    trace!(
        "fd_seek(fd={:?}, offset={:?}, whence={}, newoffset={:#x?})",
        fd,
        offset,
        wasi::whence_to_str(whence),
        newoffset
    );

    let rights = if offset == 0 && whence == wasi::__WASI_WHENCE_CUR {
        wasi::__WASI_RIGHTS_FD_TELL
    } else {
        wasi::__WASI_RIGHTS_FD_SEEK | wasi::__WASI_RIGHTS_FD_TELL
    };
    let file = wasi_ctx
        .get_fd_entry_mut(fd)?
        .as_descriptor_mut(rights, 0)?
        .as_file_mut()?;

    let pos = match whence {
        wasi::__WASI_WHENCE_CUR => SeekFrom::Current(offset),
        wasi::__WASI_WHENCE_END => SeekFrom::End(offset),
        wasi::__WASI_WHENCE_SET => SeekFrom::Start(offset as u64),
        _ => return Err(WasiError::EINVAL),
    };
    let host_newoffset = match file {
        Descriptor::OsHandle(fd) => fd.seek(pos)?,
        Descriptor::VirtualFile(virt) => virt.seek(pos)?,
        _ => {
            unreachable!(
                "implementation error: fd should have been checked to not be a stream already"
            );
        }
    };

    trace!("     | *newoffset={:?}", host_newoffset);

    enc_filesize_byref(memory, newoffset, host_newoffset)
}

pub(crate) unsafe fn fd_tell(
    wasi_ctx: &mut WasiCtx,
    memory: &mut [u8],
    fd: wasi::__wasi_fd_t,
    newoffset: wasi32::uintptr_t,
) -> WasiResult<()> {
    trace!("fd_tell(fd={:?}, newoffset={:#x?})", fd, newoffset);

    let file = wasi_ctx
        .get_fd_entry_mut(fd)?
        .as_descriptor_mut(wasi::__WASI_RIGHTS_FD_TELL, 0)?
        .as_file_mut()?;

    let host_offset = match file {
        Descriptor::OsHandle(fd) => fd.seek(SeekFrom::Current(0))?,
        Descriptor::VirtualFile(virt) => virt.seek(SeekFrom::Current(0))?,
        _ => {
            unreachable!(
                "implementation error: fd should have been checked to not be a stream already"
            );
        }
    };

    trace!("     | *newoffset={:?}", host_offset);

    enc_filesize_byref(memory, newoffset, host_offset)
}

pub(crate) unsafe fn fd_fdstat_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasi::__wasi_fd_t,
    fdstat_ptr: wasi32::uintptr_t, // *mut wasi::__wasi_fdstat_t
) -> WasiResult<()> {
    trace!("fd_fdstat_get(fd={:?}, fdstat_ptr={:#x?})", fd, fdstat_ptr);

    let mut fdstat = dec_fdstat_byref(memory, fdstat_ptr)?;
    let wasi_file = wasi_ctx.get_fd_entry(fd)?.as_descriptor(0, 0)?;

    let fs_flags = match wasi_file {
        Descriptor::OsHandle(wasi_fd) => hostcalls_impl::fd_fdstat_get(&wasi_fd)?,
        Descriptor::VirtualFile(virt) => virt.fdstat_get(),
        other => hostcalls_impl::fd_fdstat_get(&other.as_os_handle())?,
    };

    let fe = wasi_ctx.get_fd_entry(fd)?;
    fdstat.fs_filetype = fe.file_type;
    fdstat.fs_rights_base = fe.rights_base;
    fdstat.fs_rights_inheriting = fe.rights_inheriting;
    fdstat.fs_flags = fs_flags;

    trace!("     | *buf={:?}", fdstat);

    enc_fdstat_byref(memory, fdstat_ptr, fdstat)
}

pub(crate) unsafe fn fd_fdstat_set_flags(
    wasi_ctx: &mut WasiCtx,
    _memory: &mut [u8],
    fd: wasi::__wasi_fd_t,
    fdflags: wasi::__wasi_fdflags_t,
) -> WasiResult<()> {
    trace!("fd_fdstat_set_flags(fd={:?}, fdflags={:#x?})", fd, fdflags);

    let descriptor = wasi_ctx
        .get_fd_entry_mut(fd)?
        .as_descriptor_mut(wasi::__WASI_RIGHTS_FD_FDSTAT_SET_FLAGS, 0)?;

    match descriptor {
        Descriptor::OsHandle(handle) => {
            let set_result =
                hostcalls_impl::fd_fdstat_set_flags(&handle, fdflags)?.map(Descriptor::OsHandle);

            if let Some(new_descriptor) = set_result {
                *descriptor = new_descriptor;
            }
        }
        Descriptor::VirtualFile(handle) => {
            handle.fdstat_set_flags(fdflags)?;
        }
        _ => {
            let set_result =
                hostcalls_impl::fd_fdstat_set_flags(&descriptor.as_os_handle(), fdflags)?
                    .map(Descriptor::OsHandle);

            if let Some(new_descriptor) = set_result {
                *descriptor = new_descriptor;
            }
        }
    };

    Ok(())
}

pub(crate) unsafe fn fd_fdstat_set_rights(
    wasi_ctx: &mut WasiCtx,
    _memory: &mut [u8],
    fd: wasi::__wasi_fd_t,
    fs_rights_base: wasi::__wasi_rights_t,
    fs_rights_inheriting: wasi::__wasi_rights_t,
) -> WasiResult<()> {
    trace!(
        "fd_fdstat_set_rights(fd={:?}, fs_rights_base={:#x?}, fs_rights_inheriting={:#x?})",
        fd,
        fs_rights_base,
        fs_rights_inheriting
    );

    let fe = wasi_ctx.get_fd_entry_mut(fd)?;
    if fe.rights_base & fs_rights_base != fs_rights_base
        || fe.rights_inheriting & fs_rights_inheriting != fs_rights_inheriting
    {
        return Err(WasiError::ENOTCAPABLE);
    }
    fe.rights_base = fs_rights_base;
    fe.rights_inheriting = fs_rights_inheriting;

    Ok(())
}

pub(crate) unsafe fn fd_sync(
    wasi_ctx: &WasiCtx,
    _memory: &mut [u8],
    fd: wasi::__wasi_fd_t,
) -> WasiResult<()> {
    trace!("fd_sync(fd={:?})", fd);

    let file = wasi_ctx
        .get_fd_entry(fd)?
        .as_descriptor(wasi::__WASI_RIGHTS_FD_SYNC, 0)?
        .as_file()?;
    match file {
        Descriptor::OsHandle(fd) => fd.sync_all().map_err(Into::into),
        Descriptor::VirtualFile(virt) => virt.sync(),
        _ => {
            unreachable!(
                "implementation error: fd should have been checked to not be a stream already"
            );
        }
    }
}

pub(crate) unsafe fn fd_write(
    wasi_ctx: &mut WasiCtx,
    memory: &mut [u8],
    fd: wasi::__wasi_fd_t,
    iovs_ptr: wasi32::uintptr_t,
    iovs_len: wasi32::size_t,
    nwritten: wasi32::uintptr_t,
) -> WasiResult<()> {
    trace!(
        "fd_write(fd={:?}, iovs_ptr={:#x?}, iovs_len={:?}, nwritten={:#x?})",
        fd,
        iovs_ptr,
        iovs_len,
        nwritten
    );

    let iovs = dec_ciovec_slice(memory, iovs_ptr, iovs_len)?;
    let iovs: Vec<io::IoSlice> = iovs.iter().map(|vec| host::ciovec_to_host(vec)).collect();

    // perform unbuffered writes
    let entry = wasi_ctx.get_fd_entry_mut(fd)?;
    let isatty = entry.isatty();
    let desc = entry.as_descriptor_mut(wasi::__WASI_RIGHTS_FD_WRITE, 0)?;
    let host_nwritten = match desc {
        Descriptor::OsHandle(file) => {
            if isatty {
                SandboxedTTYWriter::new(file.deref_mut()).write_vectored(&iovs)?
            } else {
                file.write_vectored(&iovs)?
            }
        }
        Descriptor::VirtualFile(virt) => {
            if isatty {
                unimplemented!("writes to virtual tty");
            } else {
                virt.write_vectored(&iovs)?
            }
        }
        Descriptor::Stdin => return Err(WasiError::EBADF),
        Descriptor::Stdout => {
            // lock for the duration of the scope
            let stdout = io::stdout();
            let mut stdout = stdout.lock();
            let nwritten = if isatty {
                SandboxedTTYWriter::new(&mut stdout).write_vectored(&iovs)?
            } else {
                stdout.write_vectored(&iovs)?
            };
            stdout.flush()?;
            nwritten
        }
        // Always sanitize stderr, even if it's not directly connected to a tty,
        // because stderr is meant for diagnostics rather than binary output,
        // and may be redirected to a file which could end up being displayed
        // on a tty later.
        Descriptor::Stderr => SandboxedTTYWriter::new(&mut io::stderr()).write_vectored(&iovs)?,
    };

    trace!("     | *nwritten={:?}", host_nwritten);

    enc_usize_byref(memory, nwritten, host_nwritten)
}

pub(crate) unsafe fn fd_advise(
    wasi_ctx: &mut WasiCtx,
    _memory: &mut [u8],
    fd: wasi::__wasi_fd_t,
    offset: wasi::__wasi_filesize_t,
    len: wasi::__wasi_filesize_t,
    advice: wasi::__wasi_advice_t,
) -> WasiResult<()> {
    trace!(
        "fd_advise(fd={:?}, offset={}, len={}, advice={:?})",
        fd,
        offset,
        len,
        advice
    );

    let file = wasi_ctx
        .get_fd_entry_mut(fd)?
        .as_descriptor_mut(wasi::__WASI_RIGHTS_FD_ADVISE, 0)?
        .as_file_mut()?;

    match file {
        Descriptor::OsHandle(fd) => hostcalls_impl::fd_advise(&fd, advice, offset, len),
        Descriptor::VirtualFile(virt) => virt.advise(advice, offset, len),
        _ => {
            unreachable!(
                "implementation error: fd should have been checked to not be a stream already"
            );
        }
    }
}

pub(crate) unsafe fn fd_allocate(
    wasi_ctx: &WasiCtx,
    _memory: &mut [u8],
    fd: wasi::__wasi_fd_t,
    offset: wasi::__wasi_filesize_t,
    len: wasi::__wasi_filesize_t,
) -> WasiResult<()> {
    trace!("fd_allocate(fd={:?}, offset={}, len={})", fd, offset, len);

    let file = wasi_ctx
        .get_fd_entry(fd)?
        .as_descriptor(wasi::__WASI_RIGHTS_FD_ALLOCATE, 0)?
        .as_file()?;

    match file {
        Descriptor::OsHandle(fd) => {
            let metadata = fd.metadata()?;

            let current_size = metadata.len();
            let wanted_size = offset.checked_add(len).ok_or(WasiError::E2BIG)?;
            // This check will be unnecessary when rust-lang/rust#63326 is fixed
            if wanted_size > i64::max_value() as u64 {
                return Err(WasiError::E2BIG);
            }

            if wanted_size > current_size {
                fd.set_len(wanted_size).map_err(Into::into)
            } else {
                Ok(())
            }
        }
        Descriptor::VirtualFile(virt) => virt.allocate(offset, len),
        _ => {
            unreachable!(
                "implementation error: fd should have been checked to not be a stream already"
            );
        }
    }
}

pub(crate) unsafe fn path_create_directory(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasi::__wasi_fd_t,
    path_ptr: wasi32::uintptr_t,
    path_len: wasi32::size_t,
) -> WasiResult<()> {
    trace!(
        "path_create_directory(dirfd={:?}, path_ptr={:#x?}, path_len={})",
        dirfd,
        path_ptr,
        path_len,
    );

    let path = dec_slice_of_u8(memory, path_ptr, path_len).and_then(helpers::path_from_slice)?;

    trace!("     | (path_ptr,path_len)='{}'", path);

    let rights = wasi::__WASI_RIGHTS_PATH_OPEN | wasi::__WASI_RIGHTS_PATH_CREATE_DIRECTORY;
    let fe = wasi_ctx.get_fd_entry(dirfd)?;
    let resolved = path_get(fe, rights, 0, 0, path, false)?;

    resolved.path_create_directory()
}

pub(crate) unsafe fn path_link(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    old_dirfd: wasi::__wasi_fd_t,
    old_flags: wasi::__wasi_lookupflags_t,
    old_path_ptr: wasi32::uintptr_t,
    old_path_len: wasi32::size_t,
    new_dirfd: wasi::__wasi_fd_t,
    new_path_ptr: wasi32::uintptr_t,
    new_path_len: wasi32::size_t,
) -> WasiResult<()> {
    trace!(
        "path_link(old_dirfd={:?}, old_flags={:?}, old_path_ptr={:#x?}, old_path_len={}, new_dirfd={:?}, new_path_ptr={:#x?}, new_path_len={})",
        old_dirfd,
        old_flags,
        old_path_ptr,
        old_path_len,
        new_dirfd,
        new_path_ptr,
        new_path_len,
    );

    let old_path = dec_slice_of_u8(memory, old_path_ptr, old_path_len).and_then(path_from_slice)?;
    let new_path = dec_slice_of_u8(memory, new_path_ptr, new_path_len).and_then(path_from_slice)?;

    trace!("     | (old_path_ptr,old_path_len)='{}'", old_path);
    trace!("     | (new_path_ptr,new_path_len)='{}'", new_path);

    let old_fe = wasi_ctx.get_fd_entry(old_dirfd)?;
    let new_fe = wasi_ctx.get_fd_entry(new_dirfd)?;
    let resolved_old = path_get(
        old_fe,
        wasi::__WASI_RIGHTS_PATH_LINK_SOURCE,
        0,
        0,
        old_path,
        false,
    )?;
    let resolved_new = path_get(
        new_fe,
        wasi::__WASI_RIGHTS_PATH_LINK_TARGET,
        0,
        0,
        new_path,
        false,
    )?;

    hostcalls_impl::path_link(resolved_old, resolved_new)
}

pub(crate) unsafe fn path_open(
    wasi_ctx: &mut WasiCtx,
    memory: &mut [u8],
    dirfd: wasi::__wasi_fd_t,
    dirflags: wasi::__wasi_lookupflags_t,
    path_ptr: wasi32::uintptr_t,
    path_len: wasi32::size_t,
    oflags: wasi::__wasi_oflags_t,
    fs_rights_base: wasi::__wasi_rights_t,
    fs_rights_inheriting: wasi::__wasi_rights_t,
    fs_flags: wasi::__wasi_fdflags_t,
    fd_out_ptr: wasi32::uintptr_t,
) -> WasiResult<()> {
    trace!(
        "path_open(dirfd={:?}, dirflags={:?}, path_ptr={:#x?}, path_len={:?}, oflags={:#x?}, fs_rights_base={:#x?}, fs_rights_inheriting={:#x?}, fs_flags={:#x?}, fd_out_ptr={:#x?})",
        dirfd,
        dirflags,
        path_ptr,
        path_len,
        oflags,
        fs_rights_base,
        fs_rights_inheriting,
        fs_flags,
        fd_out_ptr
    );

    // pre-encode fd_out_ptr to -1 in case of error in opening a path
    enc_fd_byref(memory, fd_out_ptr, wasi::__wasi_fd_t::max_value())?;

    let path = dec_slice_of_u8(memory, path_ptr, path_len).and_then(path_from_slice)?;

    trace!("     | (path_ptr,path_len)='{}'", path);

    let (needed_base, needed_inheriting) =
        path_open_rights(fs_rights_base, fs_rights_inheriting, oflags, fs_flags);
    trace!(
        "     | needed_base = {}, needed_inheriting = {}",
        needed_base,
        needed_inheriting
    );
    let fe = wasi_ctx.get_fd_entry(dirfd)?;
    let resolved = path_get(
        fe,
        needed_base,
        needed_inheriting,
        dirflags,
        path,
        oflags & wasi::__WASI_OFLAGS_CREAT != 0,
    )?;

    // which open mode do we need?
    let read = fs_rights_base & (wasi::__WASI_RIGHTS_FD_READ | wasi::__WASI_RIGHTS_FD_READDIR) != 0;
    let write = fs_rights_base
        & (wasi::__WASI_RIGHTS_FD_DATASYNC
            | wasi::__WASI_RIGHTS_FD_WRITE
            | wasi::__WASI_RIGHTS_FD_ALLOCATE
            | wasi::__WASI_RIGHTS_FD_FILESTAT_SET_SIZE)
        != 0;

    trace!(
        "     | calling path_open impl: read={}, write={}",
        read,
        write
    );
    let fd = resolved.open_with(read, write, oflags, fs_flags)?;

    let mut fe = FdEntry::from(fd)?;
    // We need to manually deny the rights which are not explicitly requested
    // because FdEntry::from will assign maximal consistent rights.
    fe.rights_base &= fs_rights_base;
    fe.rights_inheriting &= fs_rights_inheriting;
    let guest_fd = wasi_ctx.insert_fd_entry(fe)?;

    trace!("     | *fd={:?}", guest_fd);

    enc_fd_byref(memory, fd_out_ptr, guest_fd)
}

pub(crate) unsafe fn path_readlink(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasi::__wasi_fd_t,
    path_ptr: wasi32::uintptr_t,
    path_len: wasi32::size_t,
    buf_ptr: wasi32::uintptr_t,
    buf_len: wasi32::size_t,
    buf_used: wasi32::uintptr_t,
) -> WasiResult<()> {
    trace!(
        "path_readlink(dirfd={:?}, path_ptr={:#x?}, path_len={:?}, buf_ptr={:#x?}, buf_len={}, buf_used={:#x?})",
        dirfd,
        path_ptr,
        path_len,
        buf_ptr,
        buf_len,
        buf_used,
    );

    enc_usize_byref(memory, buf_used, 0)?;

    let path = dec_slice_of_u8(memory, path_ptr, path_len).and_then(helpers::path_from_slice)?;

    trace!("     | (path_ptr,path_len)='{}'", &path);

    let fe = wasi_ctx.get_fd_entry(dirfd)?;
    let resolved = path_get(fe, wasi::__WASI_RIGHTS_PATH_READLINK, 0, 0, &path, false)?;

    let mut buf = dec_slice_of_mut_u8(memory, buf_ptr, buf_len)?;

    let host_bufused = match resolved.dirfd() {
        Descriptor::VirtualFile(_virt) => {
            unimplemented!("virtual readlink");
        }
        _ => hostcalls_impl::path_readlink(resolved, &mut buf)?,
    };

    trace!("     | (buf_ptr,*buf_used)={:?}", buf);
    trace!("     | *buf_used={:?}", host_bufused);

    enc_usize_byref(memory, buf_used, host_bufused)
}

pub(crate) unsafe fn path_rename(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    old_dirfd: wasi::__wasi_fd_t,
    old_path_ptr: wasi32::uintptr_t,
    old_path_len: wasi32::size_t,
    new_dirfd: wasi::__wasi_fd_t,
    new_path_ptr: wasi32::uintptr_t,
    new_path_len: wasi32::size_t,
) -> WasiResult<()> {
    trace!(
        "path_rename(old_dirfd={:?}, old_path_ptr={:#x?}, old_path_len={:?}, new_dirfd={:?}, new_path_ptr={:#x?}, new_path_len={:?})",
        old_dirfd,
        old_path_ptr,
        old_path_len,
        new_dirfd,
        new_path_ptr,
        new_path_len,
    );

    let old_path = dec_slice_of_u8(memory, old_path_ptr, old_path_len).and_then(path_from_slice)?;
    let new_path = dec_slice_of_u8(memory, new_path_ptr, new_path_len).and_then(path_from_slice)?;

    trace!("     | (old_path_ptr,old_path_len)='{}'", old_path);
    trace!("     | (new_path_ptr,new_path_len)='{}'", new_path);

    let old_fe = wasi_ctx.get_fd_entry(old_dirfd)?;
    let new_fe = wasi_ctx.get_fd_entry(new_dirfd)?;
    let resolved_old = path_get(
        old_fe,
        wasi::__WASI_RIGHTS_PATH_RENAME_SOURCE,
        0,
        0,
        old_path,
        true,
    )?;
    let resolved_new = path_get(
        new_fe,
        wasi::__WASI_RIGHTS_PATH_RENAME_TARGET,
        0,
        0,
        new_path,
        true,
    )?;

    log::debug!("path_rename resolved_old={:?}", resolved_old);
    log::debug!("path_rename resolved_new={:?}", resolved_new);

    if let (Descriptor::OsHandle(_), Descriptor::OsHandle(_)) =
        (resolved_old.dirfd(), resolved_new.dirfd())
    {
        hostcalls_impl::path_rename(resolved_old, resolved_new)
    } else {
        // Virtual files do not support rename, at the moment, and streams don't have paths to
        // rename, so any combination of Descriptor that gets here is an error in the making.
        panic!("path_rename with one or more non-OS files");
    }
}

pub(crate) unsafe fn fd_filestat_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasi::__wasi_fd_t,
    filestat_ptr: wasi32::uintptr_t,
) -> WasiResult<()> {
    trace!(
        "fd_filestat_get(fd={:?}, filestat_ptr={:#x?})",
        fd,
        filestat_ptr
    );

    let fd = wasi_ctx
        .get_fd_entry(fd)?
        .as_descriptor(wasi::__WASI_RIGHTS_FD_FILESTAT_GET, 0)?
        .as_file()?;
    let host_filestat = match fd {
        Descriptor::OsHandle(fd) => hostcalls_impl::fd_filestat_get(&fd)?,
        Descriptor::VirtualFile(virt) => virt.filestat_get()?,
        _ => {
            unreachable!(
                "implementation error: fd should have been checked to not be a stream already"
            );
        }
    };

    trace!("     | *filestat_ptr={:?}", host_filestat);

    enc_filestat_byref(memory, filestat_ptr, host_filestat)
}

pub(crate) unsafe fn fd_filestat_set_times(
    wasi_ctx: &WasiCtx,
    _memory: &mut [u8],
    fd: wasi::__wasi_fd_t,
    st_atim: wasi::__wasi_timestamp_t,
    st_mtim: wasi::__wasi_timestamp_t,
    fst_flags: wasi::__wasi_fstflags_t,
) -> WasiResult<()> {
    trace!(
        "fd_filestat_set_times(fd={:?}, st_atim={}, st_mtim={}, fst_flags={:#x?})",
        fd,
        st_atim,
        st_mtim,
        fst_flags
    );

    let fd = wasi_ctx
        .get_fd_entry(fd)?
        .as_descriptor(wasi::__WASI_RIGHTS_FD_FILESTAT_SET_TIMES, 0)?
        .as_file()?;

    fd_filestat_set_times_impl(&fd, st_atim, st_mtim, fst_flags)
}

pub(crate) fn fd_filestat_set_times_impl(
    file: &Descriptor,
    st_atim: wasi::__wasi_timestamp_t,
    st_mtim: wasi::__wasi_timestamp_t,
    fst_flags: wasi::__wasi_fstflags_t,
) -> WasiResult<()> {
    let set_atim = fst_flags & wasi::__WASI_FSTFLAGS_ATIM != 0;
    let set_atim_now = fst_flags & wasi::__WASI_FSTFLAGS_ATIM_NOW != 0;
    let set_mtim = fst_flags & wasi::__WASI_FSTFLAGS_MTIM != 0;
    let set_mtim_now = fst_flags & wasi::__WASI_FSTFLAGS_MTIM_NOW != 0;

    if (set_atim && set_atim_now) || (set_mtim && set_mtim_now) {
        return Err(WasiError::EINVAL);
    }
    let atim = if set_atim {
        let time = UNIX_EPOCH + Duration::from_nanos(st_atim);
        Some(FileTime::from_system_time(time))
    } else if set_atim_now {
        let time = SystemTime::now();
        Some(FileTime::from_system_time(time))
    } else {
        None
    };

    let mtim = if set_mtim {
        let time = UNIX_EPOCH + Duration::from_nanos(st_mtim);
        Some(FileTime::from_system_time(time))
    } else if set_mtim_now {
        let time = SystemTime::now();
        Some(FileTime::from_system_time(time))
    } else {
        None
    };
    match file {
        Descriptor::OsHandle(fd) => set_file_handle_times(fd, atim, mtim).map_err(Into::into),
        Descriptor::VirtualFile(virt) => virt.filestat_set_times(atim, mtim),
        _ => {
            unreachable!(
                "implementation error: fd should have been checked to not be a stream already"
            );
        }
    }
}

pub(crate) unsafe fn fd_filestat_set_size(
    wasi_ctx: &WasiCtx,
    _memory: &mut [u8],
    fd: wasi::__wasi_fd_t,
    st_size: wasi::__wasi_filesize_t,
) -> WasiResult<()> {
    trace!("fd_filestat_set_size(fd={:?}, st_size={})", fd, st_size);

    let file = wasi_ctx
        .get_fd_entry(fd)?
        .as_descriptor(wasi::__WASI_RIGHTS_FD_FILESTAT_SET_SIZE, 0)?
        .as_file()?;

    // This check will be unnecessary when rust-lang/rust#63326 is fixed
    if st_size > i64::max_value() as u64 {
        return Err(WasiError::E2BIG);
    }
    match file {
        Descriptor::OsHandle(fd) => fd.set_len(st_size).map_err(Into::into),
        Descriptor::VirtualFile(virt) => virt.filestat_set_size(st_size),
        _ => {
            unreachable!(
                "implementation error: fd should have been checked to not be a stream already"
            );
        }
    }
}

pub(crate) unsafe fn path_filestat_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasi::__wasi_fd_t,
    dirflags: wasi::__wasi_lookupflags_t,
    path_ptr: wasi32::uintptr_t,
    path_len: wasi32::size_t,
    filestat_ptr: wasi32::uintptr_t,
) -> WasiResult<()> {
    trace!(
        "path_filestat_get(dirfd={:?}, dirflags={:?}, path_ptr={:#x?}, path_len={}, filestat_ptr={:#x?})",
        dirfd,
        dirflags,
        path_ptr,
        path_len,
        filestat_ptr
    );

    let path = dec_slice_of_u8(memory, path_ptr, path_len).and_then(path_from_slice)?;

    trace!("     | (path_ptr,path_len)='{}'", path);

    let fe = wasi_ctx.get_fd_entry(dirfd)?;
    let resolved = path_get(
        fe,
        wasi::__WASI_RIGHTS_PATH_FILESTAT_GET,
        0,
        dirflags,
        path,
        false,
    )?;
    let host_filestat = match resolved.dirfd() {
        Descriptor::VirtualFile(virt) => virt
            .openat(std::path::Path::new(resolved.path()), false, false, 0, 0)?
            .filestat_get()?,
        _ => hostcalls_impl::path_filestat_get(resolved, dirflags)?,
    };

    trace!("     | *filestat_ptr={:?}", host_filestat);

    enc_filestat_byref(memory, filestat_ptr, host_filestat)
}

pub(crate) unsafe fn path_filestat_set_times(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasi::__wasi_fd_t,
    dirflags: wasi::__wasi_lookupflags_t,
    path_ptr: wasi32::uintptr_t,
    path_len: wasi32::size_t,
    st_atim: wasi::__wasi_timestamp_t,
    st_mtim: wasi::__wasi_timestamp_t,
    fst_flags: wasi::__wasi_fstflags_t,
) -> WasiResult<()> {
    trace!(
        "path_filestat_set_times(dirfd={:?}, dirflags={:?}, path_ptr={:#x?}, path_len={}, st_atim={}, st_mtim={}, fst_flags={:#x?})",
        dirfd,
        dirflags,
        path_ptr,
        path_len,
        st_atim, st_mtim,
        fst_flags
    );

    let path = dec_slice_of_u8(memory, path_ptr, path_len).and_then(path_from_slice)?;

    trace!("     | (path_ptr,path_len)='{}'", path);

    let fe = wasi_ctx.get_fd_entry(dirfd)?;
    let resolved = path_get(
        fe,
        wasi::__WASI_RIGHTS_PATH_FILESTAT_SET_TIMES,
        0,
        dirflags,
        path,
        false,
    )?;

    match resolved.dirfd() {
        Descriptor::VirtualFile(_virt) => {
            unimplemented!("virtual filestat_set_times");
        }
        _ => {
            hostcalls_impl::path_filestat_set_times(resolved, dirflags, st_atim, st_mtim, fst_flags)
        }
    }
}

pub(crate) unsafe fn path_symlink(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    old_path_ptr: wasi32::uintptr_t,
    old_path_len: wasi32::size_t,
    dirfd: wasi::__wasi_fd_t,
    new_path_ptr: wasi32::uintptr_t,
    new_path_len: wasi32::size_t,
) -> WasiResult<()> {
    trace!(
        "path_symlink(old_path_ptr={:#x?}, old_path_len={}, dirfd={:?}, new_path_ptr={:#x?}, new_path_len={})",
        old_path_ptr,
        old_path_len,
        dirfd,
        new_path_ptr,
        new_path_len
    );

    let old_path = dec_slice_of_u8(memory, old_path_ptr, old_path_len).and_then(path_from_slice)?;
    let new_path = dec_slice_of_u8(memory, new_path_ptr, new_path_len).and_then(path_from_slice)?;

    trace!("     | (old_path_ptr,old_path_len)='{}'", old_path);
    trace!("     | (new_path_ptr,new_path_len)='{}'", new_path);

    let fe = wasi_ctx.get_fd_entry(dirfd)?;
    let resolved_new = path_get(fe, wasi::__WASI_RIGHTS_PATH_SYMLINK, 0, 0, new_path, true)?;

    match resolved_new.dirfd() {
        Descriptor::VirtualFile(_virt) => {
            unimplemented!("virtual path_symlink");
        }
        _non_virtual => hostcalls_impl::path_symlink(old_path, resolved_new),
    }
}

pub(crate) unsafe fn path_unlink_file(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasi::__wasi_fd_t,
    path_ptr: wasi32::uintptr_t,
    path_len: wasi32::size_t,
) -> WasiResult<()> {
    trace!(
        "path_unlink_file(dirfd={:?}, path_ptr={:#x?}, path_len={})",
        dirfd,
        path_ptr,
        path_len
    );

    let path = dec_slice_of_u8(memory, path_ptr, path_len).and_then(path_from_slice)?;

    trace!("     | (path_ptr,path_len)='{}'", path);

    let fe = wasi_ctx.get_fd_entry(dirfd)?;
    let resolved = path_get(fe, wasi::__WASI_RIGHTS_PATH_UNLINK_FILE, 0, 0, path, false)?;

    match resolved.dirfd() {
        Descriptor::VirtualFile(virt) => virt.unlink_file(resolved.path()),
        _ => hostcalls_impl::path_unlink_file(resolved),
    }
}

pub(crate) unsafe fn path_remove_directory(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasi::__wasi_fd_t,
    path_ptr: wasi32::uintptr_t,
    path_len: wasi32::size_t,
) -> WasiResult<()> {
    trace!(
        "path_remove_directory(dirfd={:?}, path_ptr={:#x?}, path_len={})",
        dirfd,
        path_ptr,
        path_len
    );

    let path = dec_slice_of_u8(memory, path_ptr, path_len).and_then(path_from_slice)?;

    trace!("     | (path_ptr,path_len)='{}'", path);

    let fe = wasi_ctx.get_fd_entry(dirfd)?;
    let resolved = path_get(
        fe,
        wasi::__WASI_RIGHTS_PATH_REMOVE_DIRECTORY,
        0,
        0,
        path,
        true,
    )?;

    log::debug!("path_remove_directory resolved={:?}", resolved);

    match resolved.dirfd() {
        Descriptor::VirtualFile(virt) => virt.remove_directory(resolved.path()),
        _ => hostcalls_impl::path_remove_directory(resolved),
    }
}

pub(crate) unsafe fn fd_prestat_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasi::__wasi_fd_t,
    prestat_ptr: wasi32::uintptr_t,
) -> WasiResult<()> {
    trace!(
        "fd_prestat_get(fd={:?}, prestat_ptr={:#x?})",
        fd,
        prestat_ptr
    );

    // TODO: should we validate any rights here?
    let fe = wasi_ctx.get_fd_entry(fd)?;
    let po_path = fe.preopen_path.as_ref().ok_or(WasiError::ENOTSUP)?;
    if fe.file_type != wasi::__WASI_FILETYPE_DIRECTORY {
        return Err(WasiError::ENOTDIR);
    }

    let path = host_impl::path_from_host(po_path.as_os_str())?;

    enc_prestat_byref(
        memory,
        prestat_ptr,
        host::__wasi_prestat_t {
            tag: wasi::__WASI_PREOPENTYPE_DIR,
            u: host::__wasi_prestat_u_t {
                dir: host::__wasi_prestat_dir_t {
                    pr_name_len: path.len(),
                },
            },
        },
    )
}

pub(crate) unsafe fn fd_prestat_dir_name(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasi::__wasi_fd_t,
    path_ptr: wasi32::uintptr_t,
    path_len: wasi32::size_t,
) -> WasiResult<()> {
    trace!(
        "fd_prestat_dir_name(fd={:?}, path_ptr={:#x?}, path_len={})",
        fd,
        path_ptr,
        path_len
    );

    // TODO: should we validate any rights here?
    let fe = wasi_ctx.get_fd_entry(fd)?;
    let po_path = fe.preopen_path.as_ref().ok_or(WasiError::ENOTSUP)?;
    if fe.file_type != wasi::__WASI_FILETYPE_DIRECTORY {
        return Err(WasiError::ENOTDIR);
    }

    let path = host_impl::path_from_host(po_path.as_os_str())?;

    if path.len() > dec_usize(path_len) {
        return Err(WasiError::ENAMETOOLONG);
    }

    trace!("     | (path_ptr,path_len)='{}'", path);

    enc_slice_of_u8(memory, path.as_bytes(), path_ptr)
}

pub(crate) unsafe fn fd_readdir(
    wasi_ctx: &mut WasiCtx,
    memory: &mut [u8],
    fd: wasi::__wasi_fd_t,
    buf: wasi32::uintptr_t,
    buf_len: wasi32::size_t,
    cookie: wasi::__wasi_dircookie_t,
    buf_used: wasi32::uintptr_t,
) -> WasiResult<()> {
    trace!(
        "fd_readdir(fd={:?}, buf={:#x?}, buf_len={}, cookie={:#x?}, buf_used={:#x?})",
        fd,
        buf,
        buf_len,
        cookie,
        buf_used,
    );

    enc_usize_byref(memory, buf_used, 0)?;

    let file = wasi_ctx
        .get_fd_entry_mut(fd)?
        .as_descriptor_mut(wasi::__WASI_RIGHTS_FD_READDIR, 0)?
        .as_file_mut()?;
    let host_buf = dec_slice_of_mut_u8(memory, buf, buf_len)?;

    trace!("     | (buf,buf_len)={:?}", host_buf);

    fn copy_entities<T: Iterator<Item = WasiResult<Dirent>>>(
        iter: T,
        mut host_buf: &mut [u8],
    ) -> WasiResult<usize> {
        let mut host_bufused = 0;
        for dirent in iter {
            let dirent_raw = dirent?.to_wasi_raw()?;
            let offset = dirent_raw.len();
            if host_buf.len() < offset {
                break;
            } else {
                host_buf[0..offset].copy_from_slice(&dirent_raw);
                host_bufused += offset;
                host_buf = &mut host_buf[offset..];
            }
        }
        Ok(host_bufused)
    }

    let host_bufused = match file {
        Descriptor::OsHandle(file) => {
            copy_entities(hostcalls_impl::fd_readdir(file, cookie)?, host_buf)?
        }
        Descriptor::VirtualFile(virt) => copy_entities(virt.readdir(cookie)?, host_buf)?,
        _ => {
            unreachable!(
                "implementation error: fd should have been checked to not be a stream already"
            );
        }
    };

    trace!("     | *buf_used={:?}", host_bufused);

    enc_usize_byref(memory, buf_used, host_bufused)
}
