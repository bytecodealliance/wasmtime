use crate::entry::{Descriptor, Entry};
use crate::sandboxed_tty_writer::SandboxedTTYWriter;
use crate::wasi::wasi_snapshot_preview1::WasiSnapshotPreview1;
use crate::wasi::{types, AsBytes, Errno, Result};
use crate::WasiCtx;
use crate::{clock, fd, path, poll};
use log::{debug, error, trace};
use std::cell::Ref;
use std::convert::TryInto;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::ops::DerefMut;
use wiggle_runtime::{GuestBorrows, GuestPtr};

impl<'a> WasiSnapshotPreview1 for WasiCtx {
    fn args_get<'b>(
        &self,
        argv: &GuestPtr<'b, GuestPtr<'b, u8>>,
        argv_buf: &GuestPtr<'b, u8>,
    ) -> Result<()> {
        trace!("args_get(argv_ptr={:?}, argv_buf={:?})", argv, argv_buf);

        let mut argv = argv.clone();
        let mut argv_buf = argv_buf.clone();

        for arg in &self.args {
            let arg_bytes = arg.as_bytes_with_nul();
            let elems = arg_bytes.len().try_into()?;
            argv_buf.as_array(elems).copy_from_slice(arg_bytes)?;
            argv.write(argv_buf)?;
            argv = argv.add(1)?;
            argv_buf = argv_buf.add(elems)?;
        }

        Ok(())
    }

    fn args_sizes_get(&self) -> Result<(types::Size, types::Size)> {
        trace!("args_sizes_get");

        let argc = self.args.len().try_into()?;
        let mut argv_size: types::Size = 0;
        for arg in &self.args {
            let arg_len = arg.as_bytes_with_nul().len().try_into()?;
            argv_size = argv_size.checked_add(arg_len).ok_or(Errno::Overflow)?;
        }

        trace!("     | *argc_ptr={:?}", argc);
        trace!("     | *argv_buf_size_ptr={:?}", argv_size);

        Ok((argc, argv_size))
    }

    fn environ_get<'b>(
        &self,
        environ: &GuestPtr<'b, GuestPtr<'b, u8>>,
        environ_buf: &GuestPtr<'b, u8>,
    ) -> Result<()> {
        trace!(
            "environ_get(environ={:?}, environ_buf={:?})",
            environ,
            environ_buf
        );

        let mut environ = environ.clone();
        let mut environ_buf = environ_buf.clone();

        for e in &self.env {
            let environ_bytes = e.as_bytes_with_nul();
            let elems = environ_bytes.len().try_into()?;
            environ_buf.as_array(elems).copy_from_slice(environ_bytes)?;
            environ.write(environ_buf)?;
            environ = environ.add(1)?;
            environ_buf = environ_buf.add(elems)?;
        }

        Ok(())
    }

    fn environ_sizes_get(&self) -> Result<(types::Size, types::Size)> {
        trace!("environ_sizes_get");

        let environ_count = self.env.len().try_into()?;
        let mut environ_size: types::Size = 0;
        for environ in &self.env {
            let env_len = environ.as_bytes_with_nul().len().try_into()?;
            environ_size = environ_size.checked_add(env_len).ok_or(Errno::Overflow)?;
        }

        trace!("     | *environ_count_ptr={:?}", environ_count);
        trace!("     | *environ_size_ptr={:?}", environ_size);

        Ok((environ_count, environ_size))
    }

    fn clock_res_get(&self, id: types::Clockid) -> Result<types::Timestamp> {
        trace!("clock_res_get(id={:?})", id);
        let resolution = clock::res_get(id)?;
        trace!("     | *resolution_ptr={:?}", resolution);
        Ok(resolution)
    }

    fn clock_time_get(
        &self,
        id: types::Clockid,
        precision: types::Timestamp,
    ) -> Result<types::Timestamp> {
        trace!("clock_time_get(id={:?}, precision={:?})", id, precision);
        let time = clock::time_get(id)?;
        trace!("     | *time_ptr={:?}", time);
        Ok(time)
    }

    fn fd_advise(
        &self,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
        advice: types::Advice,
    ) -> Result<()> {
        trace!(
            "fd_advise(fd={:?}, offset={}, len={}, advice={})",
            fd,
            offset,
            len,
            advice
        );
        let mut entry = self.get_entry_mut(fd)?;
        let file = entry
            .as_descriptor_mut(types::Rights::FD_ADVISE, types::Rights::empty())?
            .as_file_mut()?;
        match file {
            Descriptor::OsHandle(fd) => fd::advise(&fd, advice, offset, len),
            Descriptor::VirtualFile(virt) => virt.advise(advice, offset, len),
            _ => {
                unreachable!(
                    "implementation error: fd should have been checked to not be a stream already"
                );
            }
        }
    }

    fn fd_allocate(
        &self,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
    ) -> Result<()> {
        trace!("fd_allocate(fd={:?}, offset={}, len={})", fd, offset, len);

        let entry = self.get_entry(fd)?;
        let file = entry
            .as_descriptor(types::Rights::FD_ALLOCATE, types::Rights::empty())?
            .as_file()?;
        match file {
            Descriptor::OsHandle(fd) => {
                let metadata = fd.metadata()?;
                let current_size = metadata.len();
                let wanted_size = offset.checked_add(len).ok_or(Errno::TooBig)?;
                // This check will be unnecessary when rust-lang/rust#63326 is fixed
                if wanted_size > i64::max_value() as u64 {
                    return Err(Errno::TooBig);
                }
                if wanted_size > current_size {
                    fd.set_len(wanted_size)?;
                }
                Ok(())
            }
            Descriptor::VirtualFile(virt) => virt.allocate(offset, len),
            _ => {
                unreachable!(
                    "implementation error: fd should have been checked to not be a stream already"
                );
            }
        }
    }

    fn fd_close(&self, fd: types::Fd) -> Result<()> {
        trace!("fd_close(fd={:?})", fd);

        if let Ok(fe) = self.get_entry(fd) {
            // can't close preopened files
            if fe.preopen_path.is_some() {
                return Err(Errno::Notsup);
            }
        }

        self.remove_entry(fd)?;
        Ok(())
    }

    fn fd_datasync(&self, fd: types::Fd) -> Result<()> {
        trace!("fd_datasync(fd={:?})", fd);

        let entry = self.get_entry(fd)?;
        let file = entry.as_descriptor(types::Rights::FD_DATASYNC, types::Rights::empty())?;
        match file {
            Descriptor::OsHandle(fd) => fd.sync_data()?,
            Descriptor::VirtualFile(virt) => virt.datasync()?,
            other => other.as_os_handle().sync_data()?,
        };
        Ok(())
    }

    fn fd_fdstat_get(&self, fd: types::Fd) -> Result<types::Fdstat> {
        trace!("fd_fdstat_get(fd={:?})", fd);

        let fe = self.get_entry(fd)?;
        let wasi_file = fe.as_descriptor(types::Rights::empty(), types::Rights::empty())?;
        let fs_flags = match wasi_file {
            Descriptor::OsHandle(wasi_fd) => fd::fdstat_get(&wasi_fd)?,
            Descriptor::VirtualFile(virt) => virt.fdstat_get(),
            other => fd::fdstat_get(&other.as_os_handle())?,
        };
        let fdstat = types::Fdstat {
            fs_filetype: fe.file_type,
            fs_rights_base: fe.rights_base,
            fs_rights_inheriting: fe.rights_inheriting,
            fs_flags,
        };

        trace!("     | *buf={:?}", fdstat);

        Ok(fdstat)
    }

    fn fd_fdstat_set_flags(&self, fd: types::Fd, flags: types::Fdflags) -> Result<()> {
        trace!("fd_fdstat_set_flags(fd={:?}, fdflags={})", fd, flags);

        let mut entry = self.get_entry_mut(fd)?;
        let descriptor =
            entry.as_descriptor_mut(types::Rights::FD_FDSTAT_SET_FLAGS, types::Rights::empty())?;
        match descriptor {
            Descriptor::OsHandle(handle) => {
                let set_result = fd::fdstat_set_flags(&handle, flags)?.map(Descriptor::OsHandle);
                if let Some(new_descriptor) = set_result {
                    *descriptor = new_descriptor;
                }
            }
            Descriptor::VirtualFile(handle) => {
                handle.fdstat_set_flags(flags)?;
            }
            _ => {
                let set_result = fd::fdstat_set_flags(&descriptor.as_os_handle(), flags)?
                    .map(Descriptor::OsHandle);
                if let Some(new_descriptor) = set_result {
                    *descriptor = new_descriptor;
                }
            }
        };
        Ok(())
    }

    fn fd_fdstat_set_rights(
        &self,
        fd: types::Fd,
        fs_rights_base: types::Rights,
        fs_rights_inheriting: types::Rights,
    ) -> Result<()> {
        trace!(
            "fd_fdstat_set_rights(fd={:?}, fs_rights_base={}, fs_rights_inheriting={})",
            fd,
            fs_rights_base,
            fs_rights_inheriting
        );
        let mut entry = self.get_entry_mut(fd)?;
        if entry.rights_base & fs_rights_base != fs_rights_base
            || entry.rights_inheriting & fs_rights_inheriting != fs_rights_inheriting
        {
            return Err(Errno::Notcapable);
        }
        entry.rights_base = fs_rights_base;
        entry.rights_inheriting = fs_rights_inheriting;
        Ok(())
    }

    fn fd_filestat_get(&self, fd: types::Fd) -> Result<types::Filestat> {
        trace!("fd_filestat_get(fd={:?})", fd);

        let entry = self.get_entry(fd)?;
        let fd = entry
            .as_descriptor(types::Rights::FD_FILESTAT_GET, types::Rights::empty())?
            .as_file()?;
        let host_filestat = match fd {
            Descriptor::OsHandle(fd) => fd::filestat_get(&fd)?,
            Descriptor::VirtualFile(virt) => virt.filestat_get()?,
            _ => {
                unreachable!(
                    "implementation error: fd should have been checked to not be a stream already"
                );
            }
        };

        trace!("     | *filestat_ptr={:?}", host_filestat);

        Ok(host_filestat)
    }

    fn fd_filestat_set_size(&self, fd: types::Fd, size: types::Filesize) -> Result<()> {
        trace!("fd_filestat_set_size(fd={:?}, size={})", fd, size);

        let entry = self.get_entry(fd)?;
        let file = entry
            .as_descriptor(types::Rights::FD_FILESTAT_SET_SIZE, types::Rights::empty())?
            .as_file()?;
        // This check will be unnecessary when rust-lang/rust#63326 is fixed
        if size > i64::max_value() as u64 {
            return Err(Errno::TooBig);
        }
        match file {
            Descriptor::OsHandle(fd) => fd.set_len(size)?,
            Descriptor::VirtualFile(virt) => virt.filestat_set_size(size)?,
            _ => {
                unreachable!(
                    "implementation error: fd should have been checked to not be a stream already"
                );
            }
        };
        Ok(())
    }

    fn fd_filestat_set_times(
        &self,
        fd: types::Fd,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<()> {
        trace!(
            "fd_filestat_set_times(fd={:?}, atim={}, mtim={}, fst_flags={})",
            fd,
            atim,
            mtim,
            fst_flags
        );
        let entry = self.get_entry(fd)?;
        let fd = entry
            .as_descriptor(types::Rights::FD_FILESTAT_SET_TIMES, types::Rights::empty())?
            .as_file()?;
        fd::filestat_set_times_impl(&fd, atim, mtim, fst_flags)
    }

    fn fd_pread(
        &self,
        fd: types::Fd,
        iovs: &types::IovecArray<'_>,
        offset: types::Filesize,
    ) -> Result<types::Size> {
        trace!("fd_pread(fd={:?}, iovs={:?}, offset={})", fd, iovs, offset,);

        let mut buf = Vec::new();
        let mut bc = GuestBorrows::new();
        bc.borrow_slice(iovs)?;
        for iov_ptr in iovs.iter() {
            let iov_ptr = iov_ptr?;
            let iov: types::Iovec = iov_ptr.read()?;
            let slice = unsafe {
                let buf = iov.buf.as_array(iov.buf_len);
                let raw = buf.as_raw(&mut bc)?;
                &mut *raw
            };
            buf.push(io::IoSliceMut::new(slice));
        }

        let mut entry = self.get_entry_mut(fd)?;
        let file = entry
            .as_descriptor_mut(
                types::Rights::FD_READ | types::Rights::FD_SEEK,
                types::Rights::empty(),
            )?
            .as_file_mut()?;

        if offset > i64::max_value() as u64 {
            return Err(Errno::Io);
        }

        let host_nread = match file {
            Descriptor::OsHandle(fd) => {
                let cur_pos = fd.seek(SeekFrom::Current(0))?;
                fd.seek(SeekFrom::Start(offset))?;
                let nread = fd.read_vectored(&mut buf)?;
                fd.seek(SeekFrom::Start(cur_pos))?;
                nread
            }
            Descriptor::VirtualFile(virt) => virt.preadv(&mut buf, offset)?,
            _ => {
                unreachable!(
                    "implementation error: fd should have been checked to not be a stream already"
                );
            }
        };
        let host_nread = host_nread.try_into()?;

        trace!("     | *nread={:?}", host_nread);

        Ok(host_nread)
    }

    fn fd_prestat_get(&self, fd: types::Fd) -> Result<types::Prestat> {
        trace!("fd_prestat_get(fd={:?})", fd);

        // TODO: should we validate any rights here?
        let fe = self.get_entry(fd)?;
        let po_path = fe.preopen_path.as_ref().ok_or(Errno::Notsup)?;
        if fe.file_type != types::Filetype::Directory {
            return Err(Errno::Notdir);
        }

        let path = path::from_host(po_path.as_os_str())?;
        let prestat = types::PrestatDir {
            pr_name_len: path.len().try_into()?,
        };
        Ok(types::Prestat::Dir(prestat))
    }

    fn fd_prestat_dir_name(
        &self,
        fd: types::Fd,
        path: &GuestPtr<u8>,
        path_len: types::Size,
    ) -> Result<()> {
        trace!(
            "fd_prestat_dir_name(fd={:?}, path={:?}, path_len={})",
            fd,
            path,
            path_len
        );

        // TODO: should we validate any rights here?
        let fe = self.get_entry(fd)?;
        let po_path = fe.preopen_path.as_ref().ok_or(Errno::Notsup)?;
        if fe.file_type != types::Filetype::Directory {
            return Err(Errno::Notdir);
        }

        let host_path = path::from_host(po_path.as_os_str())?;
        let host_path_len = host_path.len().try_into()?;

        if host_path_len > path_len {
            return Err(Errno::Nametoolong);
        }

        trace!("     | (path_ptr,path_len)='{}'", host_path);

        path.as_array(host_path_len)
            .copy_from_slice(host_path.as_bytes())?;

        Ok(())
    }

    fn fd_pwrite(
        &self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'_>,
        offset: types::Filesize,
    ) -> Result<types::Size> {
        trace!(
            "fd_pwrite(fd={:?}, ciovs={:?}, offset={})",
            fd,
            ciovs,
            offset,
        );

        let mut buf = Vec::new();
        let mut bc = GuestBorrows::new();
        bc.borrow_slice(ciovs)?;
        for ciov_ptr in ciovs.iter() {
            let ciov_ptr = ciov_ptr?;
            let ciov: types::Ciovec = ciov_ptr.read()?;
            let slice = unsafe {
                let buf = ciov.buf.as_array(ciov.buf_len);
                let raw = buf.as_raw(&mut bc)?;
                &*raw
            };
            buf.push(io::IoSlice::new(slice));
        }

        let mut entry = self.get_entry_mut(fd)?;
        let file = entry
            .as_descriptor_mut(
                types::Rights::FD_WRITE | types::Rights::FD_SEEK,
                types::Rights::empty(),
            )?
            .as_file_mut()?;

        if offset > i64::max_value() as u64 {
            return Err(Errno::Io);
        }

        let host_nwritten = match file {
            Descriptor::OsHandle(fd) => {
                let cur_pos = fd.seek(SeekFrom::Current(0))?;
                fd.seek(SeekFrom::Start(offset))?;
                let nwritten = fd.write_vectored(&buf)?;
                fd.seek(SeekFrom::Start(cur_pos))?;
                nwritten
            }
            Descriptor::VirtualFile(virt) => virt.pwritev(&buf, offset)?,
            _ => {
                unreachable!(
                    "implementation error: fd should have been checked to not be a stream already"
                );
            }
        };
        trace!("     | *nwritten={:?}", host_nwritten);
        let host_nwritten = host_nwritten.try_into()?;

        Ok(host_nwritten)
    }

    fn fd_read(&self, fd: types::Fd, iovs: &types::IovecArray<'_>) -> Result<types::Size> {
        trace!("fd_read(fd={:?}, iovs={:?})", fd, iovs);

        let mut bc = GuestBorrows::new();
        let mut slices = Vec::new();
        bc.borrow_slice(&iovs)?;
        for iov_ptr in iovs.iter() {
            let iov_ptr = iov_ptr?;
            let iov: types::Iovec = iov_ptr.read()?;
            let slice = unsafe {
                let buf = iov.buf.as_array(iov.buf_len);
                let raw = buf.as_raw(&mut bc)?;
                &mut *raw
            };
            slices.push(io::IoSliceMut::new(slice));
        }

        let mut entry = self.get_entry_mut(fd)?;
        let host_nread =
            match entry.as_descriptor_mut(types::Rights::FD_READ, types::Rights::empty())? {
                Descriptor::OsHandle(file) => file.read_vectored(&mut slices)?,
                Descriptor::VirtualFile(virt) => virt.read_vectored(&mut slices)?,
                Descriptor::Stdin => io::stdin().read_vectored(&mut slices)?,
                _ => return Err(Errno::Badf),
            };
        let host_nread = host_nread.try_into()?;

        trace!("     | *nread={:?}", host_nread);

        Ok(host_nread)
    }

    fn fd_readdir(
        &self,
        fd: types::Fd,
        buf: &GuestPtr<u8>,
        buf_len: types::Size,
        cookie: types::Dircookie,
    ) -> Result<types::Size> {
        trace!(
            "fd_readdir(fd={:?}, buf={:?}, buf_len={}, cookie={:?})",
            fd,
            buf,
            buf_len,
            cookie,
        );

        let mut entry = self.get_entry_mut(fd)?;
        let file = entry
            .as_descriptor_mut(types::Rights::FD_READDIR, types::Rights::empty())?
            .as_file_mut()?;

        fn copy_entities<T: Iterator<Item = Result<(types::Dirent, String)>>>(
            iter: T,
            buf: &GuestPtr<u8>,
            buf_len: types::Size,
        ) -> Result<types::Size> {
            let mut bufused = 0;
            let mut buf = buf.clone();
            for pair in iter {
                let (dirent, name) = pair?;
                let dirent_raw = dirent.as_bytes()?;
                let dirent_len: types::Size = dirent_raw.len().try_into()?;
                let name_raw = name.as_bytes();
                let name_len = name_raw.len().try_into()?;
                let offset = dirent_len.checked_add(name_len).ok_or(Errno::Overflow)?;
                if (buf_len - bufused) < offset {
                    break;
                } else {
                    buf.as_array(dirent_len).copy_from_slice(&dirent_raw)?;
                    buf = buf.add(dirent_len)?;
                    buf.as_array(name_len).copy_from_slice(name_raw)?;
                    buf = buf.add(name_len)?;
                    bufused += offset;
                }
            }
            Ok(bufused)
        }
        let bufused = match file {
            Descriptor::OsHandle(file) => copy_entities(fd::readdir(file, cookie)?, buf, buf_len)?,
            Descriptor::VirtualFile(virt) => copy_entities(virt.readdir(cookie)?, buf, buf_len)?,
            _ => {
                unreachable!(
                    "implementation error: fd should have been checked to not be a stream already"
                );
            }
        };

        trace!("     | *buf_used={:?}", bufused);

        Ok(bufused)
    }

    fn fd_renumber(&self, from: types::Fd, to: types::Fd) -> Result<()> {
        trace!("fd_renumber(from={:?}, to={:?})", from, to);

        if !self.contains_entry(from) {
            return Err(Errno::Badf);
        }

        // Don't allow renumbering over a pre-opened resource.
        // TODO: Eventually, we do want to permit this, once libpreopen in
        // userspace is capable of removing entries from its tables as well.
        if let Ok(from_fe) = self.get_entry(from) {
            if from_fe.preopen_path.is_some() {
                return Err(Errno::Notsup);
            }
        }
        if let Ok(to_fe) = self.get_entry(to) {
            if to_fe.preopen_path.is_some() {
                return Err(Errno::Notsup);
            }
        }
        let fe = self.remove_entry(from)?;
        self.insert_entry_at(to, fe);
        Ok(())
    }

    fn fd_seek(
        &self,
        fd: types::Fd,
        offset: types::Filedelta,
        whence: types::Whence,
    ) -> Result<types::Filesize> {
        trace!(
            "fd_seek(fd={:?}, offset={:?}, whence={:?})",
            fd,
            offset,
            whence,
        );

        let rights = if offset == 0 && whence == types::Whence::Cur {
            types::Rights::FD_TELL
        } else {
            types::Rights::FD_SEEK | types::Rights::FD_TELL
        };
        let mut entry = self.get_entry_mut(fd)?;
        let file = entry
            .as_descriptor_mut(rights, types::Rights::empty())?
            .as_file_mut()?;
        let pos = match whence {
            types::Whence::Cur => SeekFrom::Current(offset),
            types::Whence::End => SeekFrom::End(offset),
            types::Whence::Set => SeekFrom::Start(offset as u64),
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

        Ok(host_newoffset)
    }

    fn fd_sync(&self, fd: types::Fd) -> Result<()> {
        trace!("fd_sync(fd={:?})", fd);

        let entry = self.get_entry(fd)?;
        let file = entry
            .as_descriptor(types::Rights::FD_SYNC, types::Rights::empty())?
            .as_file()?;
        match file {
            Descriptor::OsHandle(fd) => fd.sync_all()?,
            Descriptor::VirtualFile(virt) => virt.sync()?,
            _ => {
                unreachable!(
                    "implementation error: fd should have been checked to not be a stream already"
                );
            }
        };
        Ok(())
    }

    fn fd_tell(&self, fd: types::Fd) -> Result<types::Filesize> {
        trace!("fd_tell(fd={:?})", fd);

        let mut entry = self.get_entry_mut(fd)?;
        let file = entry
            .as_descriptor_mut(types::Rights::FD_TELL, types::Rights::empty())?
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

        Ok(host_offset)
    }

    fn fd_write(&self, fd: types::Fd, ciovs: &types::CiovecArray<'_>) -> Result<types::Size> {
        trace!("fd_write(fd={:?}, ciovs={:#x?})", fd, ciovs);

        let mut bc = GuestBorrows::new();
        let mut slices = Vec::new();
        bc.borrow_slice(&ciovs)?;
        for ciov_ptr in ciovs.iter() {
            let ciov_ptr = ciov_ptr?;
            let ciov: types::Ciovec = ciov_ptr.read()?;
            let slice = unsafe {
                let buf = ciov.buf.as_array(ciov.buf_len);
                let raw = buf.as_raw(&mut bc)?;
                &*raw
            };
            slices.push(io::IoSlice::new(slice));
        }

        // perform unbuffered writes
        let mut entry = self.get_entry_mut(fd)?;
        let isatty = entry.isatty();
        let desc = entry.as_descriptor_mut(types::Rights::FD_WRITE, types::Rights::empty())?;
        let host_nwritten = match desc {
            Descriptor::OsHandle(file) => {
                if isatty {
                    SandboxedTTYWriter::new(file.deref_mut()).write_vectored(&slices)?
                } else {
                    file.write_vectored(&slices)?
                }
            }
            Descriptor::VirtualFile(virt) => {
                if isatty {
                    unimplemented!("writes to virtual tty");
                } else {
                    virt.write_vectored(&slices)?
                }
            }
            Descriptor::Stdin => return Err(Errno::Badf),
            Descriptor::Stdout => {
                // lock for the duration of the scope
                let stdout = io::stdout();
                let mut stdout = stdout.lock();
                let nwritten = if isatty {
                    SandboxedTTYWriter::new(&mut stdout).write_vectored(&slices)?
                } else {
                    stdout.write_vectored(&slices)?
                };
                stdout.flush()?;
                nwritten
            }
            // Always sanitize stderr, even if it's not directly connected to a tty,
            // because stderr is meant for diagnostics rather than binary output,
            // and may be redirected to a file which could end up being displayed
            // on a tty later.
            Descriptor::Stderr => {
                SandboxedTTYWriter::new(&mut io::stderr()).write_vectored(&slices)?
            }
        };
        trace!("     | *nwritten={:?}", host_nwritten);
        Ok(host_nwritten.try_into()?)
    }

    fn path_create_directory(&self, dirfd: types::Fd, path: &GuestPtr<'_, str>) -> Result<()> {
        trace!("path_create_directory(dirfd={:?}, path={:?})", dirfd, path);

        let rights = types::Rights::PATH_OPEN | types::Rights::PATH_CREATE_DIRECTORY;
        let entry = self.get_entry(dirfd)?;
        let resolved = path::get(
            &entry,
            rights,
            types::Rights::empty(),
            types::Lookupflags::empty(),
            path,
            false,
        )?;
        resolved.create_directory()
    }

    fn path_filestat_get(
        &self,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestPtr<'_, str>,
    ) -> Result<types::Filestat> {
        trace!(
            "path_filestat_get(dirfd={:?}, flags={:?}, path={:?})",
            dirfd,
            flags,
            path,
        );

        let entry = self.get_entry(dirfd)?;
        let resolved = path::get(
            &entry,
            types::Rights::PATH_FILESTAT_GET,
            types::Rights::empty(),
            flags,
            path,
            false,
        )?;
        let host_filestat = match resolved.dirfd() {
            Descriptor::VirtualFile(virt) => virt
                .openat(
                    std::path::Path::new(resolved.path()),
                    false,
                    false,
                    types::Oflags::empty(),
                    types::Fdflags::empty(),
                )?
                .filestat_get()?,
            _ => path::filestat_get(resolved, flags)?,
        };

        trace!("     | *filestat_ptr={:?}", host_filestat);

        Ok(host_filestat)
    }

    fn path_filestat_set_times(
        &self,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestPtr<'_, str>,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<()> {
        trace!(
            "path_filestat_set_times(dirfd={:?}, flags={:?}, path={:?}, atim={}, mtim={}, fst_flags={})",
            dirfd,
            flags,
            path,
            atim,
            mtim,
            fst_flags,
        );

        let entry = self.get_entry(dirfd)?;
        let resolved = path::get(
            &entry,
            types::Rights::PATH_FILESTAT_SET_TIMES,
            types::Rights::empty(),
            flags,
            path,
            false,
        )?;
        match resolved.dirfd() {
            Descriptor::VirtualFile(_virt) => {
                unimplemented!("virtual filestat_set_times");
            }
            _ => path::filestat_set_times(resolved, flags, atim, mtim, fst_flags),
        }
    }

    fn path_link(
        &self,
        old_fd: types::Fd,
        old_flags: types::Lookupflags,
        old_path: &GuestPtr<'_, str>,
        new_fd: types::Fd,
        new_path: &GuestPtr<'_, str>,
    ) -> Result<()> {
        trace!(
            "path_link(old_fd={:?}, old_flags={:?}, old_path={:?}, new_fd={:?}, new_path={:?})",
            old_fd,
            old_flags,
            old_path,
            new_fd,
            new_path,
        );

        let old_entry = self.get_entry(old_fd)?;
        let resolved_old = path::get(
            &old_entry,
            types::Rights::PATH_LINK_SOURCE,
            types::Rights::empty(),
            types::Lookupflags::empty(),
            old_path,
            false,
        )?;
        let new_entry = self.get_entry(new_fd)?;
        let resolved_new = path::get(
            &new_entry,
            types::Rights::PATH_LINK_TARGET,
            types::Rights::empty(),
            types::Lookupflags::empty(),
            new_path,
            false,
        )?;
        path::link(
            resolved_old,
            resolved_new,
            old_flags.contains(&types::Lookupflags::SYMLINK_FOLLOW),
        )
    }

    fn path_open(
        &self,
        dirfd: types::Fd,
        dirflags: types::Lookupflags,
        path: &GuestPtr<'_, str>,
        oflags: types::Oflags,
        fs_rights_base: types::Rights,
        fs_rights_inheriting: types::Rights,
        fdflags: types::Fdflags,
    ) -> Result<types::Fd> {
        trace!(
            "path_open(dirfd={:?}, dirflags={}, path={:?}, oflags={}, fs_rights_base={}, fs_rights_inheriting={}, fdflags={}",
            dirfd,
            dirflags,
            path,
            oflags,
            fs_rights_base,
            fs_rights_inheriting,
            fdflags,
        );

        let (needed_base, needed_inheriting) =
            path::open_rights(fs_rights_base, fs_rights_inheriting, oflags, fdflags);

        trace!(
            "     | needed_base = {}, needed_inheriting = {}",
            needed_base,
            needed_inheriting
        );

        let resolved = {
            let entry = self.get_entry(dirfd)?;
            path::get(
                &entry,
                needed_base,
                needed_inheriting,
                dirflags,
                path,
                oflags & types::Oflags::CREAT != types::Oflags::empty(),
            )?
        };

        // which open mode do we need?
        let read = fs_rights_base & (types::Rights::FD_READ | types::Rights::FD_READDIR)
            != types::Rights::empty();
        let write = fs_rights_base
            & (types::Rights::FD_DATASYNC
                | types::Rights::FD_WRITE
                | types::Rights::FD_ALLOCATE
                | types::Rights::FD_FILESTAT_SET_SIZE)
            != types::Rights::empty();

        trace!(
            "     | calling path_open impl: read={}, write={}",
            read,
            write
        );

        let fd = resolved.open_with(read, write, oflags, fdflags)?;
        let mut fe = Entry::from(fd)?;
        // We need to manually deny the rights which are not explicitly requested
        // because Entry::from will assign maximal consistent rights.
        fe.rights_base &= fs_rights_base;
        fe.rights_inheriting &= fs_rights_inheriting;
        let guest_fd = self.insert_entry(fe)?;

        trace!("     | *fd={:?}", guest_fd);

        Ok(guest_fd)
    }

    fn path_readlink(
        &self,
        dirfd: types::Fd,
        path: &GuestPtr<'_, str>,
        buf: &GuestPtr<u8>,
        buf_len: types::Size,
    ) -> Result<types::Size> {
        trace!(
            "path_readlink(dirfd={:?}, path={:?}, buf={:?}, buf_len={})",
            dirfd,
            path,
            buf,
            buf_len,
        );

        let entry = self.get_entry(dirfd)?;
        let resolved = path::get(
            &entry,
            types::Rights::PATH_READLINK,
            types::Rights::empty(),
            types::Lookupflags::empty(),
            path,
            false,
        )?;

        let slice = unsafe {
            let mut bc = GuestBorrows::new();
            let buf = buf.as_array(buf_len);
            let raw = buf.as_raw(&mut bc)?;
            &mut *raw
        };
        let host_bufused = match resolved.dirfd() {
            Descriptor::VirtualFile(_virt) => {
                unimplemented!("virtual readlink");
            }
            _ => path::readlink(resolved, slice)?,
        };
        let host_bufused = host_bufused.try_into()?;

        trace!("     | (buf_ptr,*buf_used)={:?}", slice);
        trace!("     | *buf_used={:?}", host_bufused);

        Ok(host_bufused)
    }

    fn path_remove_directory(&self, dirfd: types::Fd, path: &GuestPtr<'_, str>) -> Result<()> {
        trace!("path_remove_directory(dirfd={:?}, path={:?})", dirfd, path);

        let entry = self.get_entry(dirfd)?;
        let resolved = path::get(
            &entry,
            types::Rights::PATH_REMOVE_DIRECTORY,
            types::Rights::empty(),
            types::Lookupflags::empty(),
            path,
            true,
        )?;

        debug!("path_remove_directory resolved={:?}", resolved);

        match resolved.dirfd() {
            Descriptor::VirtualFile(virt) => virt.remove_directory(resolved.path()),
            _ => path::remove_directory(resolved),
        }
    }

    fn path_rename(
        &self,
        old_fd: types::Fd,
        old_path: &GuestPtr<'_, str>,
        new_fd: types::Fd,
        new_path: &GuestPtr<'_, str>,
    ) -> Result<()> {
        trace!(
            "path_rename(old_fd={:?}, old_path={:?}, new_fd={:?}, new_path={:?})",
            old_fd,
            old_path,
            new_fd,
            new_path,
        );

        let entry = self.get_entry(old_fd)?;
        let resolved_old = path::get(
            &entry,
            types::Rights::PATH_RENAME_SOURCE,
            types::Rights::empty(),
            types::Lookupflags::empty(),
            old_path,
            true,
        )?;
        let entry = self.get_entry(new_fd)?;
        let resolved_new = path::get(
            &entry,
            types::Rights::PATH_RENAME_TARGET,
            types::Rights::empty(),
            types::Lookupflags::empty(),
            new_path,
            true,
        )?;

        log::debug!("path_rename resolved_old={:?}", resolved_old);
        log::debug!("path_rename resolved_new={:?}", resolved_new);

        if let (Descriptor::OsHandle(_), Descriptor::OsHandle(_)) =
            (resolved_old.dirfd(), resolved_new.dirfd())
        {
            path::rename(resolved_old, resolved_new)
        } else {
            // Virtual files do not support rename, at the moment, and streams don't have paths to
            // rename, so any combination of Descriptor that gets here is an error in the making.
            panic!("path_rename with one or more non-OS files");
        }
    }

    fn path_symlink(
        &self,
        old_path: &GuestPtr<'_, str>,
        dirfd: types::Fd,
        new_path: &GuestPtr<'_, str>,
    ) -> Result<()> {
        trace!(
            "path_symlink(old_path={:?}, dirfd={:?}, new_path={:?})",
            old_path,
            dirfd,
            new_path,
        );

        let entry = self.get_entry(dirfd)?;
        let resolved_new = path::get(
            &entry,
            types::Rights::PATH_SYMLINK,
            types::Rights::empty(),
            types::Lookupflags::empty(),
            new_path,
            true,
        )?;

        let old_path = unsafe {
            let mut bc = GuestBorrows::new();
            let raw = old_path.as_raw(&mut bc)?;
            &*raw
        };

        trace!("     | (old_path_ptr,old_path_len)='{}'", old_path);

        match resolved_new.dirfd() {
            Descriptor::VirtualFile(_virt) => {
                unimplemented!("virtual path_symlink");
            }
            _non_virtual => path::symlink(old_path, resolved_new),
        }
    }

    fn path_unlink_file(&self, dirfd: types::Fd, path: &GuestPtr<'_, str>) -> Result<()> {
        trace!("path_unlink_file(dirfd={:?}, path={:?})", dirfd, path);

        let entry = self.get_entry(dirfd)?;
        let resolved = path::get(
            &entry,
            types::Rights::PATH_UNLINK_FILE,
            types::Rights::empty(),
            types::Lookupflags::empty(),
            path,
            false,
        )?;
        match resolved.dirfd() {
            Descriptor::VirtualFile(virt) => virt.unlink_file(resolved.path()),
            _ => path::unlink_file(resolved),
        }
    }

    fn poll_oneoff(
        &self,
        in_: &GuestPtr<types::Subscription>,
        out: &GuestPtr<types::Event>,
        nsubscriptions: types::Size,
    ) -> Result<types::Size> {
        trace!(
            "poll_oneoff(in_={:?}, out={:?}, nsubscriptions={})",
            in_,
            out,
            nsubscriptions,
        );

        if u64::from(nsubscriptions) > types::Filesize::max_value() {
            return Err(Errno::Inval);
        }

        let mut subscriptions = Vec::new();
        let mut bc = GuestBorrows::new();
        let subs = in_.as_array(nsubscriptions);
        bc.borrow_slice(&subs)?;
        for sub_ptr in subs.iter() {
            let sub_ptr = sub_ptr?;
            let sub: types::Subscription = sub_ptr.read()?;
            subscriptions.push(sub);
        }

        let mut events = Vec::new();
        let mut timeout: Option<poll::ClockEventData> = None;
        let mut fd_events = Vec::new();

        // As mandated by the WASI spec:
        // > If `nsubscriptions` is 0, returns `errno::inval`.
        if subscriptions.is_empty() {
            return Err(Errno::Inval);
        }

        for subscription in subscriptions {
            match subscription.u {
                types::SubscriptionU::Clock(clock) => {
                    let delay = clock::to_relative_ns_delay(clock)?;
                    log::debug!("poll_oneoff event.u.clock = {:?}", clock);
                    log::debug!("poll_oneoff delay = {:?}ns", delay);
                    let current = poll::ClockEventData {
                        delay,
                        userdata: subscription.userdata,
                    };
                    let timeout = timeout.get_or_insert(current);
                    if current.delay < timeout.delay {
                        *timeout = current;
                    }
                }
                types::SubscriptionU::FdRead(fd_read) => {
                    let fd = fd_read.file_descriptor;
                    let rights = types::Rights::FD_READ | types::Rights::POLL_FD_READWRITE;
                    let entry = match self.get_entry(fd) {
                        Ok(entry) => entry,
                        Err(error) => {
                            events.push(types::Event {
                                userdata: subscription.userdata,
                                error,
                                type_: types::Eventtype::FdRead,
                                fd_readwrite: types::EventFdReadwrite {
                                    nbytes: 0,
                                    flags: types::Eventrwflags::empty(),
                                },
                            });
                            continue;
                        }
                    };
                    // TODO Can this be simplified?
                    // Validate rights on the entry before converting into host descriptor.
                    entry.validate_rights(rights, types::Rights::empty())?;
                    let descriptor = Ref::map(entry, |entry| {
                        entry.as_descriptor(rights, types::Rights::empty()).unwrap()
                    });
                    fd_events.push(poll::FdEventData {
                        descriptor,
                        r#type: types::Eventtype::FdRead,
                        userdata: subscription.userdata,
                    });
                }
                types::SubscriptionU::FdWrite(fd_write) => {
                    let fd = fd_write.file_descriptor;
                    let rights = types::Rights::FD_WRITE | types::Rights::POLL_FD_READWRITE;
                    let entry = match self.get_entry(fd) {
                        Ok(entry) => entry,
                        Err(error) => {
                            events.push(types::Event {
                                userdata: subscription.userdata,
                                error,
                                type_: types::Eventtype::FdWrite,
                                fd_readwrite: types::EventFdReadwrite {
                                    nbytes: 0,
                                    flags: types::Eventrwflags::empty(),
                                },
                            });
                            continue;
                        }
                    };
                    // TODO Can this be simplified?
                    // Validate rights on the entry before converting into host descriptor.
                    entry.validate_rights(rights, types::Rights::empty())?;
                    let descriptor = Ref::map(entry, |entry| {
                        entry.as_descriptor(rights, types::Rights::empty()).unwrap()
                    });
                    fd_events.push(poll::FdEventData {
                        descriptor,
                        r#type: types::Eventtype::FdWrite,
                        userdata: subscription.userdata,
                    });
                }
            }
        }
        log::debug!("poll_oneoff events = {:?}", events);
        log::debug!("poll_oneoff timeout = {:?}", timeout);
        log::debug!("poll_oneoff fd_events = {:?}", fd_events);
        // The underlying implementation should successfully and immediately return
        // if no events have been passed. Such situation may occur if all provided
        // events have been filtered out as errors in the code above.
        poll::oneoff(timeout, fd_events, &mut events)?;
        let nevents = events.len().try_into()?;

        let out_events = out.as_array(nevents);
        bc.borrow_slice(&out_events)?;
        for (event, event_ptr) in events.into_iter().zip(out_events.iter()) {
            let event_ptr = event_ptr?;
            event_ptr.write(event)?;
        }

        trace!("     | *nevents={:?}", nevents);

        Ok(nevents)
    }

    // This is just a temporary to ignore the warning which becomes a hard error
    // in the CI. Once we figure out non-returns in `wiggle`, this should be gone.
    #[allow(unreachable_code)]
    fn proc_exit(&self, rval: types::Exitcode) -> std::result::Result<(), ()> {
        trace!("proc_exit(rval={:?})", rval);
        // TODO: Rather than call std::process::exit here, we should trigger a
        // stack unwind similar to a trap.
        std::process::exit(rval as i32);
        Ok(())
    }

    fn proc_raise(&self, _sig: types::Signal) -> Result<()> {
        unimplemented!("proc_raise")
    }

    fn sched_yield(&self) -> Result<()> {
        trace!("sched_yield()");
        std::thread::yield_now();
        Ok(())
    }

    fn random_get(&self, buf: &GuestPtr<u8>, buf_len: types::Size) -> Result<()> {
        trace!("random_get(buf={:?}, buf_len={:?})", buf, buf_len);

        let slice = unsafe {
            let mut bc = GuestBorrows::new();
            let buf = buf.as_array(buf_len);
            let raw = buf.as_raw(&mut bc)?;
            &mut *raw
        };
        getrandom::getrandom(slice).map_err(|err| {
            error!("getrandom failure: {:?}", err);
            Errno::Io
        })
    }

    fn sock_recv(
        &self,
        _fd: types::Fd,
        _ri_data: &types::IovecArray<'_>,
        _ri_flags: types::Riflags,
    ) -> Result<(types::Size, types::Roflags)> {
        unimplemented!("sock_recv")
    }

    fn sock_send(
        &self,
        _fd: types::Fd,
        _si_data: &types::CiovecArray<'_>,
        _si_flags: types::Siflags,
    ) -> Result<types::Size> {
        unimplemented!("sock_send")
    }

    fn sock_shutdown(&self, _fd: types::Fd, _how: types::Sdflags) -> Result<()> {
        unimplemented!("sock_shutdown")
    }
}
