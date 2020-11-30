use crate::wasi::{types as types_new, wasi_snapshot_preview1::WasiSnapshotPreview1};
use crate::{Error, WasiCtx};
use std::convert::{TryFrom, TryInto};
use types::*;

wiggle::from_witx!({
    witx: ["$WASI_ROOT/phases/old/snapshot_0/witx/wasi_unstable.witx"],
    ctx: WasiCtx,
    errors: { errno => Error },
});

impl wiggle::GuestErrorType for Errno {
    fn success() -> Self {
        Self::Success
    }
}

impl types::GuestErrorConversion for WasiCtx {
    fn into_errno(&self, e: wiggle::GuestError) -> Errno {
        tracing::debug!("Guest error: {:?}", e);
        e.into()
    }
}

impl types::UserErrorConversion for WasiCtx {
    fn errno_from_error(&self, e: Error) -> Result<Errno, String> {
        tracing::debug!("Error: {:?}", e);
        Ok(e.into())
    }
}

impl From<Error> for Errno {
    fn from(e: Error) -> Errno {
        types_new::Errno::from(e).into()
    }
}

impl From<wiggle::GuestError> for Errno {
    fn from(err: wiggle::GuestError) -> Self {
        types_new::Errno::from(err).into()
    }
}

impl wasi_unstable::WasiUnstable for WasiCtx {
    fn args_get<'a>(
        &self,
        argv: &wiggle::GuestPtr<'a, wiggle::GuestPtr<'a, u8>>,
        argv_buf: &wiggle::GuestPtr<'a, u8>,
    ) -> Result<(), Error> {
        WasiSnapshotPreview1::args_get(self, argv, argv_buf)
    }

    fn args_sizes_get(&self) -> Result<(Size, Size), Error> {
        WasiSnapshotPreview1::args_sizes_get(self)
    }

    fn environ_get<'a>(
        &self,
        environ: &wiggle::GuestPtr<'a, wiggle::GuestPtr<'a, u8>>,
        environ_buf: &wiggle::GuestPtr<'a, u8>,
    ) -> Result<(), Error> {
        WasiSnapshotPreview1::environ_get(self, environ, environ_buf)
    }

    fn environ_sizes_get(&self) -> Result<(Size, Size), Error> {
        WasiSnapshotPreview1::environ_sizes_get(self)
    }

    fn clock_res_get(&self, id: Clockid) -> Result<Timestamp, Error> {
        WasiSnapshotPreview1::clock_res_get(self, id.into())
    }

    fn clock_time_get(&self, id: Clockid, precision: Timestamp) -> Result<Timestamp, Error> {
        WasiSnapshotPreview1::clock_time_get(self, id.into(), precision)
    }

    fn fd_advise(
        &self,
        fd: Fd,
        offset: Filesize,
        len: Filesize,
        advice: Advice,
    ) -> Result<(), Error> {
        WasiSnapshotPreview1::fd_advise(self, fd.into(), offset, len, advice.into())
    }

    fn fd_allocate(&self, fd: Fd, offset: Filesize, len: Filesize) -> Result<(), Error> {
        WasiSnapshotPreview1::fd_allocate(self, fd.into(), offset, len)
    }

    fn fd_close(&self, fd: Fd) -> Result<(), Error> {
        WasiSnapshotPreview1::fd_close(self, fd.into())
    }

    fn fd_datasync(&self, fd: Fd) -> Result<(), Error> {
        WasiSnapshotPreview1::fd_datasync(self, fd.into())
    }

    fn fd_fdstat_get(&self, fd: Fd) -> Result<Fdstat, Error> {
        WasiSnapshotPreview1::fd_fdstat_get(self, fd.into()).map(|s| s.into())
    }

    fn fd_fdstat_set_flags(&self, fd: Fd, flags: Fdflags) -> Result<(), Error> {
        WasiSnapshotPreview1::fd_fdstat_set_flags(self, fd.into(), flags.into())
    }

    fn fd_fdstat_set_rights(
        &self,
        fd: Fd,
        fs_rights_base: Rights,
        fs_rights_inheriting: Rights,
    ) -> Result<(), Error> {
        WasiSnapshotPreview1::fd_fdstat_set_rights(
            self,
            fd.into(),
            fs_rights_base.into(),
            fs_rights_inheriting.into(),
        )
    }

    fn fd_filestat_get(&self, fd: Fd) -> Result<Filestat, Error> {
        WasiSnapshotPreview1::fd_filestat_get(self, fd.into()).and_then(|e| e.try_into())
    }

    fn fd_filestat_set_size(&self, fd: Fd, size: Filesize) -> Result<(), Error> {
        WasiSnapshotPreview1::fd_filestat_set_size(self, fd.into(), size)
    }

    fn fd_filestat_set_times(
        &self,
        fd: Fd,
        atim: Timestamp,
        mtim: Timestamp,
        fst_flags: Fstflags,
    ) -> Result<(), Error> {
        WasiSnapshotPreview1::fd_filestat_set_times(self, fd.into(), atim, mtim, fst_flags.into())
    }

    fn fd_pread<'a>(&self, fd: Fd, iovs: &IovecArray<'a>, offset: Filesize) -> Result<Size, Error> {
        WasiSnapshotPreview1::fd_pread(self, fd.into(), &cvt_iovec(iovs), offset)
    }

    fn fd_prestat_get(&self, fd: Fd) -> Result<Prestat, Error> {
        WasiSnapshotPreview1::fd_prestat_get(self, fd.into()).map(|e| e.into())
    }

    fn fd_prestat_dir_name<'a>(
        &self,
        fd: Fd,
        path: &wiggle::GuestPtr<'a, u8>,
        path_len: Size,
    ) -> Result<(), Error> {
        WasiSnapshotPreview1::fd_prestat_dir_name(self, fd.into(), path, path_len)
    }

    fn fd_pwrite<'a>(
        &self,
        fd: Fd,
        iovs: &CiovecArray<'a>,
        offset: Filesize,
    ) -> Result<Size, Error> {
        WasiSnapshotPreview1::fd_pwrite(self, fd.into(), &cvt_ciovec(iovs), offset)
    }

    fn fd_read<'a>(&self, fd: Fd, iovs: &IovecArray<'a>) -> Result<Size, Error> {
        WasiSnapshotPreview1::fd_read(self, fd.into(), &cvt_iovec(iovs))
    }

    fn fd_readdir<'a>(
        &self,
        fd: Fd,
        buf: &wiggle::GuestPtr<'a, u8>,
        buf_len: Size,
        cookie: Dircookie,
    ) -> Result<Size, Error> {
        WasiSnapshotPreview1::fd_readdir(self, fd.into(), buf, buf_len, cookie)
    }

    fn fd_renumber(&self, from: Fd, to: Fd) -> Result<(), Error> {
        WasiSnapshotPreview1::fd_renumber(self, from.into(), to.into())
    }

    fn fd_seek(&self, fd: Fd, offset: Filedelta, whence: Whence) -> Result<Filesize, Error> {
        WasiSnapshotPreview1::fd_seek(self, fd.into(), offset, whence.into())
    }

    fn fd_sync(&self, fd: Fd) -> Result<(), Error> {
        WasiSnapshotPreview1::fd_sync(self, fd.into())
    }

    fn fd_tell(&self, fd: Fd) -> Result<Filesize, Error> {
        WasiSnapshotPreview1::fd_tell(self, fd.into())
    }

    fn fd_write<'a>(&self, fd: Fd, iovs: &CiovecArray<'a>) -> Result<Size, Error> {
        WasiSnapshotPreview1::fd_write(self, fd.into(), &cvt_ciovec(iovs))
    }

    fn path_create_directory<'a>(
        &self,
        fd: Fd,
        path: &wiggle::GuestPtr<'a, str>,
    ) -> Result<(), Error> {
        WasiSnapshotPreview1::path_create_directory(self, fd.into(), path)
    }

    fn path_filestat_get<'a>(
        &self,
        fd: Fd,
        flags: Lookupflags,
        path: &wiggle::GuestPtr<'a, str>,
    ) -> Result<Filestat, Error> {
        WasiSnapshotPreview1::path_filestat_get(self, fd.into(), flags.into(), path)
            .and_then(|e| e.try_into())
    }

    fn path_filestat_set_times<'a>(
        &self,
        fd: Fd,
        flags: Lookupflags,
        path: &wiggle::GuestPtr<'a, str>,
        atim: Timestamp,
        mtim: Timestamp,
        fst_flags: Fstflags,
    ) -> Result<(), Error> {
        WasiSnapshotPreview1::path_filestat_set_times(
            self,
            fd.into(),
            flags.into(),
            path,
            atim,
            mtim,
            fst_flags.into(),
        )
    }

    fn path_link<'a>(
        &self,
        old_fd: Fd,
        old_flags: Lookupflags,
        old_path: &wiggle::GuestPtr<'a, str>,
        new_fd: Fd,
        new_path: &wiggle::GuestPtr<'a, str>,
    ) -> Result<(), Error> {
        WasiSnapshotPreview1::path_link(
            self,
            old_fd.into(),
            old_flags.into(),
            old_path,
            new_fd.into(),
            new_path,
        )
    }

    fn path_open<'a>(
        &self,
        fd: Fd,
        dirflags: Lookupflags,
        path: &wiggle::GuestPtr<'a, str>,
        oflags: Oflags,
        fs_rights_base: Rights,
        fs_rights_inheriting: Rights,
        fdflags: Fdflags,
    ) -> Result<Fd, Error> {
        WasiSnapshotPreview1::path_open(
            self,
            fd.into(),
            dirflags.into(),
            path,
            oflags.into(),
            fs_rights_base.into(),
            fs_rights_inheriting.into(),
            fdflags.into(),
        )
        .map(|e| e.into())
    }

    fn path_readlink<'a>(
        &self,
        fd: Fd,
        path: &wiggle::GuestPtr<'a, str>,
        buf: &wiggle::GuestPtr<'a, u8>,
        buf_len: Size,
    ) -> Result<Size, Error> {
        WasiSnapshotPreview1::path_readlink(self, fd.into(), path, buf, buf_len)
    }

    fn path_remove_directory<'a>(
        &self,
        fd: Fd,
        path: &wiggle::GuestPtr<'a, str>,
    ) -> Result<(), Error> {
        WasiSnapshotPreview1::path_remove_directory(self, fd.into(), path)
    }

    fn path_rename<'a>(
        &self,
        fd: Fd,
        old_path: &wiggle::GuestPtr<'a, str>,
        new_fd: Fd,
        new_path: &wiggle::GuestPtr<'a, str>,
    ) -> Result<(), Error> {
        WasiSnapshotPreview1::path_rename(self, fd.into(), old_path, new_fd.into(), new_path)
    }

    fn path_symlink<'a>(
        &self,
        old_path: &wiggle::GuestPtr<'a, str>,
        fd: Fd,
        new_path: &wiggle::GuestPtr<'a, str>,
    ) -> Result<(), Error> {
        WasiSnapshotPreview1::path_symlink(self, old_path, fd.into(), new_path)
    }

    fn path_unlink_file<'a>(&self, fd: Fd, path: &wiggle::GuestPtr<'a, str>) -> Result<(), Error> {
        WasiSnapshotPreview1::path_unlink_file(self, fd.into(), path)
    }

    fn poll_oneoff<'a>(
        &self,
        in_: &wiggle::GuestPtr<'a, Subscription>,
        out: &wiggle::GuestPtr<'a, Event>,
        nsubscriptions: Size,
    ) -> Result<Size, Error> {
        if u64::from(nsubscriptions) > types::Filesize::max_value() {
            return Err(Error::Inval);
        }

        let mut subscriptions = Vec::new();
        let subs = in_.as_array(nsubscriptions);
        for sub_ptr in subs.iter() {
            let sub_ptr = sub_ptr?;
            let sub: types::Subscription = sub_ptr.read()?;
            subscriptions.push(sub.into());
        }

        let events = self.poll_oneoff_impl(&subscriptions)?;
        let nevents = events.len().try_into()?;

        let out_events = out.as_array(nevents);
        for (event, event_ptr) in events.into_iter().zip(out_events.iter()) {
            let event_ptr = event_ptr?;
            event_ptr.write(event.into())?;
        }

        Ok(nevents)
    }

    fn proc_exit(&self, rval: Exitcode) -> Result<(), ()> {
        WasiSnapshotPreview1::proc_exit(self, rval)
    }

    fn proc_raise(&self, sig: Signal) -> Result<(), Error> {
        WasiSnapshotPreview1::proc_raise(self, sig.into())
    }

    fn sched_yield(&self) -> Result<(), Error> {
        WasiSnapshotPreview1::sched_yield(self)
    }

    fn random_get<'a>(&self, buf: &wiggle::GuestPtr<'a, u8>, buf_len: Size) -> Result<(), Error> {
        WasiSnapshotPreview1::random_get(self, buf, buf_len)
    }

    fn sock_recv<'a>(
        &self,
        fd: Fd,
        ri_data: &IovecArray<'a>,
        ri_flags: Riflags,
    ) -> Result<(Size, Roflags), Error> {
        WasiSnapshotPreview1::sock_recv(self, fd.into(), &cvt_iovec(ri_data), ri_flags.into())
            .map(|(s, f)| (s, f.into()))
    }

    fn sock_send<'a>(
        &self,
        fd: Fd,
        si_data: &CiovecArray<'a>,
        si_flags: Siflags,
    ) -> Result<Size, Error> {
        WasiSnapshotPreview1::sock_send(self, fd.into(), &cvt_ciovec(si_data), si_flags.into())
    }

    fn sock_shutdown(&self, fd: Fd, how: Sdflags) -> Result<(), Error> {
        WasiSnapshotPreview1::sock_shutdown(self, fd.into(), how.into())
    }
}

impl From<Clockid> for types_new::Clockid {
    fn from(id: Clockid) -> types_new::Clockid {
        match id {
            Clockid::Realtime => types_new::Clockid::Realtime,
            Clockid::Monotonic => types_new::Clockid::Monotonic,
            Clockid::ProcessCputimeId => types_new::Clockid::ProcessCputimeId,
            Clockid::ThreadCputimeId => types_new::Clockid::ThreadCputimeId,
        }
    }
}

impl From<Fd> for types_new::Fd {
    fn from(fd: Fd) -> types_new::Fd {
        types_new::Fd::from(u32::from(fd))
    }
}

impl From<types_new::Fd> for Fd {
    fn from(fd: types_new::Fd) -> Fd {
        Fd::from(u32::from(fd))
    }
}

impl From<Advice> for types_new::Advice {
    fn from(e: Advice) -> types_new::Advice {
        match e {
            Advice::Normal => types_new::Advice::Normal,
            Advice::Sequential => types_new::Advice::Sequential,
            Advice::Random => types_new::Advice::Random,
            Advice::Willneed => types_new::Advice::Willneed,
            Advice::Dontneed => types_new::Advice::Dontneed,
            Advice::Noreuse => types_new::Advice::Noreuse,
        }
    }
}

impl From<types_new::Fdstat> for Fdstat {
    fn from(e: types_new::Fdstat) -> Fdstat {
        Fdstat {
            fs_filetype: e.fs_filetype.into(),
            fs_flags: e.fs_flags.into(),
            fs_rights_base: e.fs_rights_base.into(),
            fs_rights_inheriting: e.fs_rights_inheriting.into(),
        }
    }
}

fn assert_rights_same() {
    macro_rules! assert_same {
        ($($id:ident)*) => ({$(
            assert_eq!(u64::from(Rights::$id), u64::from(types_new::Rights::$id));
        )*});
    }
    assert_same! {
        FD_DATASYNC
        FD_READ
        FD_SEEK
        FD_FDSTAT_SET_FLAGS
        FD_SYNC
        FD_TELL
        FD_WRITE
        FD_ADVISE
        FD_ALLOCATE
        PATH_CREATE_DIRECTORY
        PATH_CREATE_FILE
        PATH_LINK_SOURCE
        PATH_LINK_TARGET
        PATH_OPEN
        FD_READDIR
        PATH_READLINK
        PATH_RENAME_SOURCE
        PATH_RENAME_TARGET
        PATH_FILESTAT_GET
        PATH_FILESTAT_SET_TIMES
        PATH_FILESTAT_SET_SIZE
        FD_FILESTAT_GET
        FD_FILESTAT_SET_SIZE
        FD_FILESTAT_SET_TIMES
        PATH_SYMLINK
        PATH_REMOVE_DIRECTORY
        PATH_UNLINK_FILE
        POLL_FD_READWRITE
        SOCK_SHUTDOWN
    }
}

impl From<Rights> for types_new::Rights {
    fn from(e: Rights) -> types_new::Rights {
        assert_rights_same();
        u64::from(e).try_into().unwrap()
    }
}

impl From<types_new::Rights> for Rights {
    fn from(e: types_new::Rights) -> Rights {
        assert_rights_same();
        u64::from(e).try_into().unwrap()
    }
}

impl From<Filetype> for types_new::Filetype {
    fn from(e: Filetype) -> types_new::Filetype {
        match e {
            Filetype::Unknown => types_new::Filetype::Unknown,
            Filetype::BlockDevice => types_new::Filetype::BlockDevice,
            Filetype::CharacterDevice => types_new::Filetype::CharacterDevice,
            Filetype::Directory => types_new::Filetype::Directory,
            Filetype::RegularFile => types_new::Filetype::RegularFile,
            Filetype::SocketDgram => types_new::Filetype::SocketDgram,
            Filetype::SocketStream => types_new::Filetype::SocketStream,
            Filetype::SymbolicLink => types_new::Filetype::SymbolicLink,
        }
    }
}

impl From<types_new::Filetype> for Filetype {
    fn from(e: types_new::Filetype) -> Filetype {
        match e {
            types_new::Filetype::Unknown => Filetype::Unknown,
            types_new::Filetype::BlockDevice => Filetype::BlockDevice,
            types_new::Filetype::CharacterDevice => Filetype::CharacterDevice,
            types_new::Filetype::Directory => Filetype::Directory,
            types_new::Filetype::RegularFile => Filetype::RegularFile,
            types_new::Filetype::SocketDgram => Filetype::SocketDgram,
            types_new::Filetype::SocketStream => Filetype::SocketStream,
            types_new::Filetype::SymbolicLink => Filetype::SymbolicLink,
        }
    }
}

fn assert_fdflags_same() {
    macro_rules! assert_same {
        ($($id:ident)*) => ({$(
            assert_eq!(u16::from(Fdflags::$id), u16::from(types_new::Fdflags::$id));
        )*});
    }
    assert_same! {
        APPEND
        DSYNC
        NONBLOCK
        RSYNC
        SYNC
    }
}

impl From<Fdflags> for types_new::Fdflags {
    fn from(e: Fdflags) -> types_new::Fdflags {
        assert_fdflags_same();
        u16::from(e).try_into().unwrap()
    }
}

impl From<types_new::Fdflags> for Fdflags {
    fn from(e: types_new::Fdflags) -> Fdflags {
        assert_fdflags_same();
        u16::from(e).try_into().unwrap()
    }
}

impl TryFrom<types_new::Filestat> for Filestat {
    type Error = Error;

    fn try_from(e: types_new::Filestat) -> Result<Filestat, Error> {
        Ok(Filestat {
            dev: e.dev,
            ino: e.ino,
            filetype: e.filetype.into(),
            // wasi_snapshot_preview1 has a 64-bit nlink field but we have a
            // 32-bit field, so we need to perform a fallible conversion.
            nlink: e.nlink.try_into()?,
            size: e.size,
            atim: e.atim,
            mtim: e.mtim,
            ctim: e.ctim,
        })
    }
}

fn assert_fstflags_same() {
    macro_rules! assert_same {
        ($($id:ident)*) => ({$(
            assert_eq!(u16::from(Fstflags::$id), u16::from(types_new::Fstflags::$id));
        )*});
    }
    assert_same! {
        ATIM
        ATIM_NOW
        MTIM
        MTIM_NOW
    }
}

impl From<Fstflags> for types_new::Fstflags {
    fn from(e: Fstflags) -> types_new::Fstflags {
        assert_fstflags_same();
        u16::from(e).try_into().unwrap()
    }
}

impl From<types_new::Fstflags> for Fstflags {
    fn from(e: types_new::Fstflags) -> Fstflags {
        assert_fstflags_same();
        u16::from(e).try_into().unwrap()
    }
}

impl From<types_new::Prestat> for Prestat {
    fn from(e: types_new::Prestat) -> Prestat {
        match e {
            types_new::Prestat::Dir(d) => Prestat::Dir(d.into()),
        }
    }
}

impl From<types_new::PrestatDir> for PrestatDir {
    fn from(e: types_new::PrestatDir) -> PrestatDir {
        PrestatDir {
            pr_name_len: e.pr_name_len,
        }
    }
}

impl From<Whence> for types_new::Whence {
    fn from(e: Whence) -> types_new::Whence {
        match e {
            Whence::Set => types_new::Whence::Set,
            Whence::Cur => types_new::Whence::Cur,
            Whence::End => types_new::Whence::End,
        }
    }
}

fn assert_lookupflags_same() {
    macro_rules! assert_same {
        ($($id:ident)*) => ({$(
            assert_eq!(u32::from(Lookupflags::$id), u32::from(types_new::Lookupflags::$id));
        )*});
    }
    assert_same! {
        SYMLINK_FOLLOW
    }
}

impl From<Lookupflags> for types_new::Lookupflags {
    fn from(e: Lookupflags) -> types_new::Lookupflags {
        assert_lookupflags_same();
        u32::from(e).try_into().unwrap()
    }
}

fn assert_oflags_same() {
    macro_rules! assert_same {
        ($($id:ident)*) => ({$(
            assert_eq!(u16::from(Oflags::$id), u16::from(types_new::Oflags::$id));
        )*});
    }
    assert_same! {
        CREAT
        DIRECTORY
        EXCL
        TRUNC
    }
}

impl From<Oflags> for types_new::Oflags {
    fn from(e: Oflags) -> types_new::Oflags {
        assert_oflags_same();
        u16::from(e).try_into().unwrap()
    }
}

fn assert_sdflags_same() {
    macro_rules! assert_same {
        ($($id:ident)*) => ({$(
            assert_eq!(u8::from(Sdflags::$id), u8::from(types_new::Sdflags::$id));
        )*});
    }
    assert_same! {
        RD WR
    }
}

impl From<Sdflags> for types_new::Sdflags {
    fn from(e: Sdflags) -> types_new::Sdflags {
        assert_sdflags_same();
        u8::from(e).try_into().unwrap()
    }
}

impl From<Signal> for types_new::Signal {
    fn from(e: Signal) -> types_new::Signal {
        match e {
            Signal::None => types_new::Signal::None,
            Signal::Hup => types_new::Signal::Hup,
            Signal::Int => types_new::Signal::Int,
            Signal::Quit => types_new::Signal::Quit,
            Signal::Ill => types_new::Signal::Ill,
            Signal::Trap => types_new::Signal::Trap,
            Signal::Abrt => types_new::Signal::Abrt,
            Signal::Bus => types_new::Signal::Bus,
            Signal::Fpe => types_new::Signal::Fpe,
            Signal::Kill => types_new::Signal::Kill,
            Signal::Usr1 => types_new::Signal::Usr1,
            Signal::Segv => types_new::Signal::Segv,
            Signal::Usr2 => types_new::Signal::Usr2,
            Signal::Pipe => types_new::Signal::Pipe,
            Signal::Alrm => types_new::Signal::Alrm,
            Signal::Term => types_new::Signal::Term,
            Signal::Chld => types_new::Signal::Chld,
            Signal::Cont => types_new::Signal::Cont,
            Signal::Stop => types_new::Signal::Stop,
            Signal::Tstp => types_new::Signal::Tstp,
            Signal::Ttin => types_new::Signal::Ttin,
            Signal::Ttou => types_new::Signal::Ttou,
            Signal::Urg => types_new::Signal::Urg,
            Signal::Xcpu => types_new::Signal::Xcpu,
            Signal::Xfsz => types_new::Signal::Xfsz,
            Signal::Vtalrm => types_new::Signal::Vtalrm,
            Signal::Prof => types_new::Signal::Prof,
            Signal::Winch => types_new::Signal::Winch,
            Signal::Poll => types_new::Signal::Poll,
            Signal::Pwr => types_new::Signal::Pwr,
            Signal::Sys => types_new::Signal::Sys,
        }
    }
}

// For `wasi_unstable` and `wasi_snapshot_preview1` the memory layout of these
// two types was manually verified. It should be fine to effectively cast
// between the two types and get the same behavior.
fn cvt_iovec<'a>(e: &IovecArray<'a>) -> types_new::IovecArray<'a> {
    wiggle::GuestPtr::new(e.mem(), (e.offset_base(), e.len()))
}

fn cvt_ciovec<'a>(e: &CiovecArray<'a>) -> types_new::CiovecArray<'a> {
    wiggle::GuestPtr::new(e.mem(), (e.offset_base(), e.len()))
}

fn assert_riflags_same() {
    macro_rules! assert_same {
        ($($id:ident)*) => ({$(
            assert_eq!(u16::from(Riflags::$id), u16::from(types_new::Riflags::$id));
        )*});
    }
    assert_same! {
        RECV_PEEK
        RECV_WAITALL
    }
}

impl From<Riflags> for types_new::Riflags {
    fn from(e: Riflags) -> types_new::Riflags {
        assert_riflags_same();
        u16::from(e).try_into().unwrap()
    }
}

fn assert_roflags_same() {
    macro_rules! assert_same {
        ($($id:ident)*) => ({$(
            assert_eq!(u16::from(Roflags::$id), u16::from(types_new::Roflags::$id));
        )*});
    }
    assert_same! {
        RECV_DATA_TRUNCATED
    }
}

impl From<types_new::Roflags> for Roflags {
    fn from(e: types_new::Roflags) -> Roflags {
        assert_roflags_same();
        u16::from(e).try_into().unwrap()
    }
}

impl From<Subscription> for types_new::Subscription {
    fn from(e: Subscription) -> types_new::Subscription {
        types_new::Subscription {
            userdata: e.userdata,
            u: e.u.into(),
        }
    }
}

impl From<SubscriptionU> for types_new::SubscriptionU {
    fn from(e: SubscriptionU) -> types_new::SubscriptionU {
        match e {
            SubscriptionU::Clock(c) => {
                types_new::SubscriptionU::Clock(types_new::SubscriptionClock {
                    id: c.id.into(),
                    timeout: c.timeout,
                    precision: c.precision,
                    flags: c.flags.into(),
                })
            }
            SubscriptionU::FdRead(c) => {
                types_new::SubscriptionU::FdRead(types_new::SubscriptionFdReadwrite {
                    file_descriptor: c.file_descriptor.into(),
                })
            }
            SubscriptionU::FdWrite(c) => {
                types_new::SubscriptionU::FdWrite(types_new::SubscriptionFdReadwrite {
                    file_descriptor: c.file_descriptor.into(),
                })
            }
        }
    }
}

impl From<Subclockflags> for types_new::Subclockflags {
    fn from(e: Subclockflags) -> types_new::Subclockflags {
        macro_rules! assert_same {
            ($($id:ident)*) => ({$(
                assert_eq!(u16::from(Subclockflags::$id), u16::from(types_new::Subclockflags::$id));
            )*});
        }
        assert_same! {
            SUBSCRIPTION_CLOCK_ABSTIME
        }
        u16::from(e).try_into().unwrap()
    }
}

impl From<types_new::Event> for Event {
    fn from(e: types_new::Event) -> Event {
        Event {
            userdata: e.userdata,
            error: e.error.into(),
            type_: e.type_.into(),
            fd_readwrite: e.fd_readwrite.into(),
        }
    }
}

impl From<types_new::Errno> for Errno {
    fn from(e: types_new::Errno) -> Errno {
        match e {
            types_new::Errno::Success => Errno::Success,
            types_new::Errno::TooBig => Errno::TooBig,
            types_new::Errno::Acces => Errno::Acces,
            types_new::Errno::Addrinuse => Errno::Addrinuse,
            types_new::Errno::Addrnotavail => Errno::Addrnotavail,
            types_new::Errno::Afnosupport => Errno::Afnosupport,
            types_new::Errno::Again => Errno::Again,
            types_new::Errno::Already => Errno::Already,
            types_new::Errno::Badf => Errno::Badf,
            types_new::Errno::Badmsg => Errno::Badmsg,
            types_new::Errno::Busy => Errno::Busy,
            types_new::Errno::Canceled => Errno::Canceled,
            types_new::Errno::Child => Errno::Child,
            types_new::Errno::Connaborted => Errno::Connaborted,
            types_new::Errno::Connrefused => Errno::Connrefused,
            types_new::Errno::Connreset => Errno::Connreset,
            types_new::Errno::Deadlk => Errno::Deadlk,
            types_new::Errno::Destaddrreq => Errno::Destaddrreq,
            types_new::Errno::Dom => Errno::Dom,
            types_new::Errno::Dquot => Errno::Dquot,
            types_new::Errno::Exist => Errno::Exist,
            types_new::Errno::Fault => Errno::Fault,
            types_new::Errno::Fbig => Errno::Fbig,
            types_new::Errno::Hostunreach => Errno::Hostunreach,
            types_new::Errno::Idrm => Errno::Idrm,
            types_new::Errno::Ilseq => Errno::Ilseq,
            types_new::Errno::Inprogress => Errno::Inprogress,
            types_new::Errno::Intr => Errno::Intr,
            types_new::Errno::Inval => Errno::Inval,
            types_new::Errno::Io => Errno::Io,
            types_new::Errno::Isconn => Errno::Isconn,
            types_new::Errno::Isdir => Errno::Isdir,
            types_new::Errno::Loop => Errno::Loop,
            types_new::Errno::Mfile => Errno::Mfile,
            types_new::Errno::Mlink => Errno::Mlink,
            types_new::Errno::Msgsize => Errno::Msgsize,
            types_new::Errno::Multihop => Errno::Multihop,
            types_new::Errno::Nametoolong => Errno::Nametoolong,
            types_new::Errno::Netdown => Errno::Netdown,
            types_new::Errno::Netreset => Errno::Netreset,
            types_new::Errno::Netunreach => Errno::Netunreach,
            types_new::Errno::Nfile => Errno::Nfile,
            types_new::Errno::Nobufs => Errno::Nobufs,
            types_new::Errno::Nodev => Errno::Nodev,
            types_new::Errno::Noent => Errno::Noent,
            types_new::Errno::Noexec => Errno::Noexec,
            types_new::Errno::Nolck => Errno::Nolck,
            types_new::Errno::Nolink => Errno::Nolink,
            types_new::Errno::Nomem => Errno::Nomem,
            types_new::Errno::Nomsg => Errno::Nomsg,
            types_new::Errno::Noprotoopt => Errno::Noprotoopt,
            types_new::Errno::Nospc => Errno::Nospc,
            types_new::Errno::Nosys => Errno::Nosys,
            types_new::Errno::Notconn => Errno::Notconn,
            types_new::Errno::Notdir => Errno::Notdir,
            types_new::Errno::Notempty => Errno::Notempty,
            types_new::Errno::Notrecoverable => Errno::Notrecoverable,
            types_new::Errno::Notsock => Errno::Notsock,
            types_new::Errno::Notsup => Errno::Notsup,
            types_new::Errno::Notty => Errno::Notty,
            types_new::Errno::Nxio => Errno::Nxio,
            types_new::Errno::Overflow => Errno::Overflow,
            types_new::Errno::Ownerdead => Errno::Ownerdead,
            types_new::Errno::Perm => Errno::Perm,
            types_new::Errno::Pipe => Errno::Pipe,
            types_new::Errno::Proto => Errno::Proto,
            types_new::Errno::Protonosupport => Errno::Protonosupport,
            types_new::Errno::Prototype => Errno::Prototype,
            types_new::Errno::Range => Errno::Range,
            types_new::Errno::Rofs => Errno::Rofs,
            types_new::Errno::Spipe => Errno::Spipe,
            types_new::Errno::Srch => Errno::Srch,
            types_new::Errno::Stale => Errno::Stale,
            types_new::Errno::Timedout => Errno::Timedout,
            types_new::Errno::Txtbsy => Errno::Txtbsy,
            types_new::Errno::Xdev => Errno::Xdev,
            types_new::Errno::Notcapable => Errno::Notcapable,
        }
    }
}

impl From<types_new::Eventtype> for Eventtype {
    fn from(e: types_new::Eventtype) -> Eventtype {
        match e {
            types_new::Eventtype::Clock => Eventtype::Clock,
            types_new::Eventtype::FdRead => Eventtype::FdRead,
            types_new::Eventtype::FdWrite => Eventtype::FdWrite,
        }
    }
}

impl From<types_new::EventFdReadwrite> for EventFdReadwrite {
    fn from(e: types_new::EventFdReadwrite) -> EventFdReadwrite {
        EventFdReadwrite {
            nbytes: e.nbytes,
            flags: e.flags.into(),
        }
    }
}

impl From<types_new::Eventrwflags> for Eventrwflags {
    fn from(e: types_new::Eventrwflags) -> Eventrwflags {
        macro_rules! assert_same {
            ($($id:ident)*) => ({$(
                assert_eq!(u16::from(Eventrwflags::$id), u16::from(types_new::Eventrwflags::$id));
            )*});
        }
        assert_same! {
            FD_READWRITE_HANGUP
        }
        u16::from(e).try_into().unwrap()
    }
}
