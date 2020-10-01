use crate::sched::{Poll, SchedResult, SubscriptionResult, Userdata};
use crate::sys::clock;
use crate::wasi::types;
use crate::wasi::wasi_snapshot_preview1::WasiSnapshotPreview1;
use crate::{Error, WasiCtx};
use std::ops::{Deref, DerefMut};
use wiggle::{GuestPtr, GuestSlice};

impl<'a> WasiSnapshotPreview1 for WasiCtx {
    fn args_get<'b>(
        &self,
        argv: &GuestPtr<'b, GuestPtr<'b, u8>>,
        argv_buf: &GuestPtr<'b, u8>,
    ) -> Result<(), Error> {
        self.args.write_to_guest(argv_buf, argv)
    }

    fn args_sizes_get(&self) -> Result<(types::Size, types::Size), Error> {
        Ok((self.args.number_elements, self.args.cumulative_size))
    }

    fn environ_get<'b>(
        &self,
        environ: &GuestPtr<'b, GuestPtr<'b, u8>>,
        environ_buf: &GuestPtr<'b, u8>,
    ) -> Result<(), Error> {
        self.env.write_to_guest(environ_buf, environ)
    }

    fn environ_sizes_get(&self) -> Result<(types::Size, types::Size), Error> {
        Ok((self.env.number_elements, self.env.cumulative_size))
    }

    fn clock_res_get(&self, id: types::Clockid) -> Result<types::Timestamp, Error> {
        let resolution = clock::res_get(id)?;
        Ok(resolution)
    }

    fn clock_time_get(
        &self,
        id: types::Clockid,
        _precision: types::Timestamp,
    ) -> Result<types::Timestamp, Error> {
        let time = clock::time_get(id)?;
        Ok(time)
    }

    fn fd_advise(
        &self,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
        advice: types::Advice,
    ) -> Result<(), Error> {
        self.get_entry(fd)?.fd_advise(offset, len, advice)
    }

    fn fd_allocate(
        &self,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
    ) -> Result<(), Error> {
        self.get_entry(fd)?.fd_allocate(offset, len)
    }

    fn fd_close(&self, fd: types::Fd) -> Result<(), Error> {
        self.get_entry(fd)?.fd_close()?;
        self.remove_entry(fd)?;
        Ok(())
    }

    fn fd_datasync(&self, fd: types::Fd) -> Result<(), Error> {
        self.get_entry(fd)?.fd_datasync()
    }

    fn fd_fdstat_get(&self, fd: types::Fd) -> Result<types::Fdstat, Error> {
        self.get_entry(fd)?.fd_fdstat_get()
    }

    fn fd_fdstat_set_flags(&self, fd: types::Fd, flags: types::Fdflags) -> Result<(), Error> {
        self.get_entry(fd)?.fd_fdstat_set_flags(flags)
    }

    fn fd_fdstat_set_rights(
        &self,
        fd: types::Fd,
        fs_rights_base: types::Rights,
        fs_rights_inheriting: types::Rights,
    ) -> Result<(), Error> {
        self.get_entry(fd)?
            .fd_fdstat_set_rights(fs_rights_base, fs_rights_inheriting)
    }

    fn fd_filestat_get(&self, fd: types::Fd) -> Result<types::Filestat, Error> {
        self.get_entry(fd)?.fd_filestat_get()
    }

    fn fd_filestat_set_size(&self, fd: types::Fd, size: types::Filesize) -> Result<(), Error> {
        self.get_entry(fd)?.fd_filestat_set_size(size)
    }

    fn fd_filestat_set_times(
        &self,
        fd: types::Fd,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<(), Error> {
        self.get_entry(fd)?
            .fd_filestat_set_times(atim, mtim, fst_flags)
    }

    fn fd_read(&self, fd: types::Fd, iovs: &types::IovecArray<'_>) -> Result<types::Size, Error> {
        let entry = self.get_entry(fd)?;

        let mut guest_slices = Vec::new();
        for iov_ptr in iovs.iter() {
            let iov_ptr = iov_ptr?;
            let iov: types::Iovec = iov_ptr.read()?;
            guest_slices.push(iov.buf.as_array(iov.buf_len).as_slice()?);
        }

        entry.fd_read(guest_slices)
    }

    fn fd_pread(
        &self,
        fd: types::Fd,
        iovs: &types::IovecArray<'_>,
        offset: types::Filesize,
    ) -> Result<types::Size, Error> {
        let entry = self.get_entry(fd)?;

        // Rather than expose the details of our IovecArray to the Entry, it accepts a
        // Vec<GuestSlice<u8>>
        let mut guest_slices: Vec<GuestSlice<'_, u8>> = Vec::new();
        for iov_ptr in iovs.iter() {
            let iov_ptr = iov_ptr?;
            let iov: types::Iovec = iov_ptr.read()?;
            guest_slices.push(iov.buf.as_array(iov.buf_len).as_slice()?);
        }

        entry.fd_pread(guest_slices, offset)
    }

    fn fd_write(
        &self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'_>,
    ) -> Result<types::Size, Error> {
        let entry = self.get_entry(fd)?;

        let mut guest_slices = Vec::new();
        for ciov_ptr in ciovs.iter() {
            let ciov_ptr = ciov_ptr?;
            let ciov: types::Ciovec = ciov_ptr.read()?;
            guest_slices.push(ciov.buf.as_array(ciov.buf_len).as_slice()?);
        }
        entry.fd_write(guest_slices)
    }
    fn fd_pwrite(
        &self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'_>,
        offset: types::Filesize,
    ) -> Result<types::Size, Error> {
        let entry = self.get_entry(fd)?;

        let mut guest_slices = Vec::new();
        for ciov_ptr in ciovs.iter() {
            let ciov_ptr = ciov_ptr?;
            let ciov: types::Ciovec = ciov_ptr.read()?;
            guest_slices.push(ciov.buf.as_array(ciov.buf_len).as_slice()?);
        }

        entry.fd_pwrite(guest_slices, offset)
    }

    fn fd_prestat_get(&self, fd: types::Fd) -> Result<types::Prestat, Error> {
        self.get_entry(fd)?.fd_prestat_get()
    }

    fn fd_prestat_dir_name(
        &self,
        fd: types::Fd,
        path: &GuestPtr<u8>,
        path_len: types::Size,
    ) -> Result<(), Error> {
        let entry = self.get_entry(fd)?;
        let mut path = path.as_array(path_len).as_slice()?;
        entry.fd_prestat_dir_name(path.deref_mut())
    }
    fn fd_readdir(
        &self,
        fd: types::Fd,
        buf: &GuestPtr<u8>,
        buf_len: types::Size,
        cookie: types::Dircookie,
    ) -> Result<types::Size, Error> {
        let entry = self.get_entry(fd)?;
        let mut buf = buf.as_array(buf_len).as_slice()?;
        entry.fd_readdir(buf.deref_mut(), cookie)
    }

    fn fd_renumber(&self, from: types::Fd, to: types::Fd) -> Result<(), Error> {
        if !self.contains_entry(from) {
            return Err(Error::Badf);
        }

        // Don't allow renumbering over a pre-opened resource.
        // TODO: Eventually, we do want to permit this, once libpreopen in
        // userspace is capable of removing entries from its tables as well.
        if let Ok(from_fe) = self.get_entry(from) {
            if from_fe.preopen_path.is_some() {
                return Err(Error::Notsup);
            }
        }
        if let Ok(to_fe) = self.get_entry(to) {
            if to_fe.preopen_path.is_some() {
                return Err(Error::Notsup);
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
    ) -> Result<types::Filesize, Error> {
        self.get_entry(fd)?.fd_seek(offset, whence)
    }

    fn fd_sync(&self, fd: types::Fd) -> Result<(), Error> {
        self.get_entry(fd)?.fd_sync()
    }

    fn fd_tell(&self, fd: types::Fd) -> Result<types::Filesize, Error> {
        self.get_entry(fd)?.fd_tell()
    }

    fn path_create_directory(
        &self,
        dirfd: types::Fd,
        path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        let entry = self.get_entry(dirfd)?;
        let path = path.as_str()?;
        entry.path_create_directory(path.deref())
    }

    fn path_filestat_get(
        &self,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestPtr<'_, str>,
    ) -> Result<types::Filestat, Error> {
        let entry = self.get_entry(dirfd)?;
        let path = path.as_str()?;
        entry.path_filestat_get(flags, path.deref())
    }

    fn path_filestat_set_times(
        &self,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestPtr<'_, str>,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<(), Error> {
        let entry = self.get_entry(dirfd)?;
        let path = path.as_str()?;
        entry.path_filestat_set_times(flags, path.deref(), atim, mtim, fst_flags)
    }

    fn path_link(
        &self,
        old_fd: types::Fd,
        old_flags: types::Lookupflags,
        old_path: &GuestPtr<'_, str>,
        new_fd: types::Fd,
        new_path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        let old_entry = self.get_entry(old_fd)?;
        let new_entry = self.get_entry(new_fd)?;
        old_entry.path_link(old_flags, old_path, &new_entry, new_path)
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
    ) -> Result<types::Fd, Error> {
        let entry = self.get_entry(dirfd)?;
        let path = path.as_str()?;
        let opened_entry = entry.path_open(
            dirflags,
            path.deref(),
            oflags,
            fs_rights_base,
            fs_rights_inheriting,
            fdflags,
        )?;
        let guest_fd = self.insert_entry(opened_entry)?;
        Ok(guest_fd)
    }

    fn path_readlink(
        &self,
        dirfd: types::Fd,
        path: &GuestPtr<'_, str>,
        buf: &GuestPtr<u8>,
        buf_len: types::Size,
    ) -> Result<types::Size, Error> {
        let entry = self.get_entry(dirfd)?;
        let buf = buf.as_array(buf_len);
        entry.path_readlink(path, &buf)
    }

    fn path_remove_directory(
        &self,
        dirfd: types::Fd,
        path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        let entry = self.get_entry(dirfd)?;
        let path = path.as_str()?;
        entry.path_remove_directory(path.deref())
    }

    fn path_rename(
        &self,
        old_fd: types::Fd,
        old_path: &GuestPtr<'_, str>,
        new_fd: types::Fd,
        new_path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        let old_entry = self.get_entry(old_fd)?;
        let new_entry = self.get_entry(new_fd)?;
        old_entry.path_rename(old_path, &new_entry, new_path)
    }

    fn path_symlink(
        &self,
        old_path: &GuestPtr<'_, str>,
        dirfd: types::Fd,
        new_path: &GuestPtr<'_, str>,
    ) -> Result<(), Error> {
        let entry = self.get_entry(dirfd)?;
        entry.path_symlink(old_path, new_path)
    }

    fn path_unlink_file(&self, dirfd: types::Fd, path: &GuestPtr<'_, str>) -> Result<(), Error> {
        let entry = self.get_entry(dirfd)?;
        let path = path.as_str()?;
        entry.path_unlink_file(path.deref())
    }

    fn poll_oneoff(
        &self,
        subs: &GuestPtr<types::Subscription>,
        events: &GuestPtr<types::Event>,
        nsubscriptions: types::Size,
    ) -> Result<types::Size, Error> {
        if nsubscriptions == 0 {
            // As mandated by the WASI spec:
            // > If `nsubscriptions` is 0, returns `errno::inval`.
            return Err(Error::Inval);
        }
        let subs = subs.as_array(nsubscriptions);
        let events = events.as_array(nsubscriptions);

        let poll = {
            let mut error_events = Vec::new();
            let poll = poll_builder(&self, subs, &mut error_events)?;
            if !error_events.is_empty() {
                return poll_writer(error_events, events);
            }
            poll
        };

        // TODO actually run the poll here

        poll_writer(poll.results(), events)
    }

    fn proc_exit(&self, _rval: types::Exitcode) -> Result<(), ()> {
        // proc_exit is special in that it's expected to unwind the stack, which
        // typically requires runtime-specific logic.
        unimplemented!("runtimes are expected to override this implementation")
    }

    fn proc_raise(&self, _sig: types::Signal) -> Result<(), Error> {
        unimplemented!("proc_raise")
    }

    fn sched_yield(&self) -> Result<(), Error> {
        std::thread::yield_now();
        Ok(())
    }

    fn random_get(&self, buf: &GuestPtr<u8>, buf_len: types::Size) -> Result<(), Error> {
        let mut slice = buf.as_array(buf_len).as_slice()?;
        getrandom::getrandom(&mut *slice)?;
        Ok(())
    }

    fn sock_recv(
        &self,
        _fd: types::Fd,
        _ri_data: &types::IovecArray<'_>,
        _ri_flags: types::Riflags,
    ) -> Result<(types::Size, types::Roflags), Error> {
        unimplemented!("sock_recv")
    }

    fn sock_send(
        &self,
        _fd: types::Fd,
        _si_data: &types::CiovecArray<'_>,
        _si_flags: types::Siflags,
    ) -> Result<types::Size, Error> {
        unimplemented!("sock_send")
    }

    fn sock_shutdown(&self, _fd: types::Fd, _how: types::Sdflags) -> Result<(), Error> {
        unimplemented!("sock_shutdown")
    }
}

pub fn poll_builder<'a>(
    ctx: &'a WasiCtx,
    subs: GuestPtr<[types::Subscription]>,
    // Errors get written to the events if subscriptions are invalid:
    errors: &mut Vec<(SubscriptionResult, Userdata)>,
) -> Result<Poll<'a>, Error> {
    let mut poll = Poll::new(ctx);

    for sub_ptr in subs.iter() {
        let sub_ptr = sub_ptr?;
        let sub: types::Subscription = sub_ptr.read()?;
        match sub.u {
            types::SubscriptionU::Clock(clock) => {
                let r = if clock.flags == types::Subclockflags::SUBSCRIPTION_CLOCK_ABSTIME {
                    poll.subscribe_absolute_time(clock.timeout, sub.userdata)
                } else {
                    poll.subscribe_relative_time(clock.timeout, sub.userdata)
                };
                if let Err(e) = r {
                    errors.push((SubscriptionResult::Timer(SchedResult::Err(e)), sub.userdata));
                }
            }
            types::SubscriptionU::FdRead(fd_read) => {
                if let Err(e) = poll.subscribe_fd_read(fd_read.file_descriptor, sub.userdata) {
                    errors.push((SubscriptionResult::Read(SchedResult::Err(e)), sub.userdata));
                }
            }
            types::SubscriptionU::FdWrite(fd_write) => {
                if let Err(e) = poll.subscribe_fd_write(fd_write.file_descriptor, sub.userdata) {
                    errors.push((SubscriptionResult::Write(SchedResult::Err(e)), sub.userdata));
                }
            }
        }
    }
    Ok(poll)
}

fn poll_writer(
    results: Vec<(SubscriptionResult, Userdata)>,
    events: GuestPtr<[types::Event]>,
) -> Result<types::Size, Error> {
}
