//! Bindings for WASIp0 aka Preview 0 aka `wasi_unstable`.
//!
//! This module is purely here for backwards compatibility in the Wasmtime CLI.
//! You probably want to use [`preview1`](crate::preview1) instead.

use crate::preview0::types::Error;
use crate::preview1::types as snapshot1_types;
use crate::preview1::wasi_snapshot_preview1::WasiSnapshotPreview1 as Snapshot1;
use crate::preview1::WasiP1Ctx;
use wiggle::{GuestError, GuestPtr};

pub fn add_to_linker_async<T: Send>(
    linker: &mut wasmtime::Linker<T>,
    f: impl Fn(&mut T) -> &mut WasiP1Ctx + Copy + Send + Sync + 'static,
) -> anyhow::Result<()> {
    wasi_unstable::add_to_linker(linker, f)
}

pub fn add_to_linker_sync<T: Send>(
    linker: &mut wasmtime::Linker<T>,
    f: impl Fn(&mut T) -> &mut WasiP1Ctx + Copy + Send + Sync + 'static,
) -> anyhow::Result<()> {
    sync::add_wasi_unstable_to_linker(linker, f)
}

wiggle::from_witx!({
    witx: ["$CARGO_MANIFEST_DIR/witx/preview0/wasi_unstable.witx"],
    async: {
        wasi_unstable::{
            fd_advise, fd_close, fd_datasync, fd_fdstat_get, fd_filestat_get, fd_filestat_set_size,
            fd_filestat_set_times, fd_read, fd_pread, fd_seek, fd_sync, fd_readdir, fd_write,
            fd_pwrite, poll_oneoff, path_create_directory, path_filestat_get,
            path_filestat_set_times, path_link, path_open, path_readlink, path_remove_directory,
            path_rename, path_symlink, path_unlink_file
        }
    },
    errors: { errno => trappable Error },
});

mod sync {
    use anyhow::Result;
    use std::future::Future;

    wiggle::wasmtime_integration!({
        witx: ["$CARGO_MANIFEST_DIR/witx/preview0/wasi_unstable.witx"],
        target: super,
        block_on[in_tokio]: {
            wasi_unstable::{
                fd_advise, fd_close, fd_datasync, fd_fdstat_get, fd_filestat_get, fd_filestat_set_size,
                fd_filestat_set_times, fd_read, fd_pread, fd_seek, fd_sync, fd_readdir, fd_write,
                fd_pwrite, poll_oneoff, path_create_directory, path_filestat_get,
                path_filestat_set_times, path_link, path_open, path_readlink, path_remove_directory,
                path_rename, path_symlink, path_unlink_file
            }
        },
        errors: { errno => trappable Error },
    });

    // Small wrapper around `in_tokio` to add a `Result` layer which is always
    // `Ok`
    fn in_tokio<F: Future>(future: F) -> Result<F::Output> {
        Ok(crate::runtime::in_tokio(future))
    }
}

impl wiggle::GuestErrorType for types::Errno {
    fn success() -> Self {
        Self::Success
    }
}

#[wiggle::async_trait]
impl<T: Snapshot1 + Send> wasi_unstable::WasiUnstable for T {
    fn args_get<'a>(
        &mut self,
        argv: &GuestPtr<'a, GuestPtr<'a, u8>>,
        argv_buf: &GuestPtr<'a, u8>,
    ) -> Result<(), Error> {
        Snapshot1::args_get(self, argv, argv_buf)?;
        Ok(())
    }

    fn args_sizes_get(&mut self) -> Result<(types::Size, types::Size), Error> {
        let s = Snapshot1::args_sizes_get(self)?;
        Ok(s)
    }

    fn environ_get<'a>(
        &mut self,
        environ: &GuestPtr<'a, GuestPtr<'a, u8>>,
        environ_buf: &GuestPtr<'a, u8>,
    ) -> Result<(), Error> {
        Snapshot1::environ_get(self, environ, environ_buf)?;
        Ok(())
    }

    fn environ_sizes_get(&mut self) -> Result<(types::Size, types::Size), Error> {
        let s = Snapshot1::environ_sizes_get(self)?;
        Ok(s)
    }

    fn clock_res_get(&mut self, id: types::Clockid) -> Result<types::Timestamp, Error> {
        let t = Snapshot1::clock_res_get(self, id.into())?;
        Ok(t)
    }

    fn clock_time_get(
        &mut self,
        id: types::Clockid,
        precision: types::Timestamp,
    ) -> Result<types::Timestamp, Error> {
        let t = Snapshot1::clock_time_get(self, id.into(), precision)?;
        Ok(t)
    }

    async fn fd_advise(
        &mut self,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
        advice: types::Advice,
    ) -> Result<(), Error> {
        Snapshot1::fd_advise(self, fd.into(), offset, len, advice.into()).await?;
        Ok(())
    }

    fn fd_allocate(
        &mut self,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
    ) -> Result<(), Error> {
        Snapshot1::fd_allocate(self, fd.into(), offset, len)?;
        Ok(())
    }

    async fn fd_close(&mut self, fd: types::Fd) -> Result<(), Error> {
        Snapshot1::fd_close(self, fd.into()).await?;
        Ok(())
    }

    async fn fd_datasync(&mut self, fd: types::Fd) -> Result<(), Error> {
        Snapshot1::fd_datasync(self, fd.into()).await?;
        Ok(())
    }

    async fn fd_fdstat_get(&mut self, fd: types::Fd) -> Result<types::Fdstat, Error> {
        Ok(Snapshot1::fd_fdstat_get(self, fd.into()).await?.into())
    }

    fn fd_fdstat_set_flags(&mut self, fd: types::Fd, flags: types::Fdflags) -> Result<(), Error> {
        Snapshot1::fd_fdstat_set_flags(self, fd.into(), flags.into())?;
        Ok(())
    }

    fn fd_fdstat_set_rights(
        &mut self,
        fd: types::Fd,
        fs_rights_base: types::Rights,
        fs_rights_inheriting: types::Rights,
    ) -> Result<(), Error> {
        Snapshot1::fd_fdstat_set_rights(
            self,
            fd.into(),
            fs_rights_base.into(),
            fs_rights_inheriting.into(),
        )?;
        Ok(())
    }

    async fn fd_filestat_get(&mut self, fd: types::Fd) -> Result<types::Filestat, Error> {
        Ok(Snapshot1::fd_filestat_get(self, fd.into()).await?.into())
    }

    async fn fd_filestat_set_size(
        &mut self,
        fd: types::Fd,
        size: types::Filesize,
    ) -> Result<(), Error> {
        Snapshot1::fd_filestat_set_size(self, fd.into(), size).await?;
        Ok(())
    }

    async fn fd_filestat_set_times(
        &mut self,
        fd: types::Fd,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<(), Error> {
        Snapshot1::fd_filestat_set_times(self, fd.into(), atim, mtim, fst_flags.into()).await?;
        Ok(())
    }

    async fn fd_read<'a>(
        &mut self,
        fd: types::Fd,
        iovs: &types::IovecArray<'a>,
    ) -> Result<types::Size, Error> {
        assert_iovec_array_same();
        let result = Snapshot1::fd_read(self, fd.into(), &iovs.cast()).await?;
        Ok(result)
    }

    async fn fd_pread<'a>(
        &mut self,
        fd: types::Fd,
        iovs: &types::IovecArray<'a>,
        offset: types::Filesize,
    ) -> Result<types::Size, Error> {
        assert_iovec_array_same();
        let result = Snapshot1::fd_pread(self, fd.into(), &iovs.cast(), offset).await?;
        Ok(result)
    }

    async fn fd_write<'a>(
        &mut self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'a>,
    ) -> Result<types::Size, Error> {
        assert_ciovec_array_same();
        let result = Snapshot1::fd_write(self, fd.into(), &ciovs.cast()).await?;
        Ok(result)
    }

    async fn fd_pwrite<'a>(
        &mut self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'a>,
        offset: types::Filesize,
    ) -> Result<types::Size, Error> {
        assert_ciovec_array_same();
        let result = Snapshot1::fd_pwrite(self, fd.into(), &ciovs.cast(), offset).await?;
        Ok(result)
    }

    fn fd_prestat_get(&mut self, fd: types::Fd) -> Result<types::Prestat, Error> {
        Ok(Snapshot1::fd_prestat_get(self, fd.into())?.into())
    }

    fn fd_prestat_dir_name(
        &mut self,
        fd: types::Fd,
        path: &GuestPtr<'_, u8>,
        path_max_len: types::Size,
    ) -> Result<(), Error> {
        Snapshot1::fd_prestat_dir_name(self, fd.into(), path, path_max_len)?;
        Ok(())
    }

    fn fd_renumber(&mut self, from: types::Fd, to: types::Fd) -> Result<(), Error> {
        Snapshot1::fd_renumber(self, from.into(), to.into())?;
        Ok(())
    }

    async fn fd_seek(
        &mut self,
        fd: types::Fd,
        offset: types::Filedelta,
        whence: types::Whence,
    ) -> Result<types::Filesize, Error> {
        Ok(Snapshot1::fd_seek(self, fd.into(), offset, whence.into()).await?)
    }

    async fn fd_sync(&mut self, fd: types::Fd) -> Result<(), Error> {
        Snapshot1::fd_sync(self, fd.into()).await?;
        Ok(())
    }

    fn fd_tell(&mut self, fd: types::Fd) -> Result<types::Filesize, Error> {
        Ok(Snapshot1::fd_tell(self, fd.into())?)
    }

    async fn fd_readdir<'a>(
        &mut self,
        fd: types::Fd,
        buf: &GuestPtr<'a, u8>,
        buf_len: types::Size,
        cookie: types::Dircookie,
    ) -> Result<types::Size, Error> {
        Ok(Snapshot1::fd_readdir(self, fd.into(), buf, buf_len, cookie).await?)
    }

    async fn path_create_directory<'a>(
        &mut self,
        dirfd: types::Fd,
        path: &GuestPtr<'a, str>,
    ) -> Result<(), Error> {
        Snapshot1::path_create_directory(self, dirfd.into(), path).await?;
        Ok(())
    }

    async fn path_filestat_get<'a>(
        &mut self,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestPtr<'a, str>,
    ) -> Result<types::Filestat, Error> {
        Ok(
            Snapshot1::path_filestat_get(self, dirfd.into(), flags.into(), path)
                .await?
                .into(),
        )
    }

    async fn path_filestat_set_times<'a>(
        &mut self,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestPtr<'a, str>,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<(), Error> {
        Snapshot1::path_filestat_set_times(
            self,
            dirfd.into(),
            flags.into(),
            path,
            atim,
            mtim,
            fst_flags.into(),
        )
        .await?;
        Ok(())
    }

    async fn path_link<'a>(
        &mut self,
        src_fd: types::Fd,
        src_flags: types::Lookupflags,
        src_path: &GuestPtr<'a, str>,
        target_fd: types::Fd,
        target_path: &GuestPtr<'a, str>,
    ) -> Result<(), Error> {
        Snapshot1::path_link(
            self,
            src_fd.into(),
            src_flags.into(),
            src_path,
            target_fd.into(),
            target_path,
        )
        .await?;
        Ok(())
    }

    async fn path_open<'a>(
        &mut self,
        dirfd: types::Fd,
        dirflags: types::Lookupflags,
        path: &GuestPtr<'a, str>,
        oflags: types::Oflags,
        fs_rights_base: types::Rights,
        fs_rights_inheriting: types::Rights,
        fdflags: types::Fdflags,
    ) -> Result<types::Fd, Error> {
        Ok(Snapshot1::path_open(
            self,
            dirfd.into(),
            dirflags.into(),
            path,
            oflags.into(),
            fs_rights_base.into(),
            fs_rights_inheriting.into(),
            fdflags.into(),
        )
        .await?
        .into())
    }

    async fn path_readlink<'a>(
        &mut self,
        dirfd: types::Fd,
        path: &GuestPtr<'a, str>,
        buf: &GuestPtr<'a, u8>,
        buf_len: types::Size,
    ) -> Result<types::Size, Error> {
        Ok(Snapshot1::path_readlink(self, dirfd.into(), path, buf, buf_len).await?)
    }

    async fn path_remove_directory<'a>(
        &mut self,
        dirfd: types::Fd,
        path: &GuestPtr<'a, str>,
    ) -> Result<(), Error> {
        Snapshot1::path_remove_directory(self, dirfd.into(), path).await?;
        Ok(())
    }

    async fn path_rename<'a>(
        &mut self,
        src_fd: types::Fd,
        src_path: &GuestPtr<'a, str>,
        dest_fd: types::Fd,
        dest_path: &GuestPtr<'a, str>,
    ) -> Result<(), Error> {
        Snapshot1::path_rename(self, src_fd.into(), src_path, dest_fd.into(), dest_path).await?;
        Ok(())
    }

    async fn path_symlink<'a>(
        &mut self,
        src_path: &GuestPtr<'a, str>,
        dirfd: types::Fd,
        dest_path: &GuestPtr<'a, str>,
    ) -> Result<(), Error> {
        Snapshot1::path_symlink(self, src_path, dirfd.into(), dest_path).await?;
        Ok(())
    }

    async fn path_unlink_file<'a>(
        &mut self,
        dirfd: types::Fd,
        path: &GuestPtr<'a, str>,
    ) -> Result<(), Error> {
        Snapshot1::path_unlink_file(self, dirfd.into(), path).await?;
        Ok(())
    }

    // The representation of `SubscriptionClock` is different in preview0 and
    // preview1 so a bit of a hack is employed here. The change was to remove a
    // field from `SubscriptionClock` so to implement this without copying too
    // much the `subs` field is overwritten with preview1-compatible structures
    // and then the preview1 implementation is used. Before returning though
    // the old values are restored to pretend like we didn't overwrite them.
    //
    // Surely no one would pass overlapping pointers to this API right?
    async fn poll_oneoff<'a>(
        &mut self,
        subs: &GuestPtr<'a, types::Subscription>,
        events: &GuestPtr<'a, types::Event>,
        nsubscriptions: types::Size,
    ) -> Result<types::Size, Error> {
        let subs_array = subs.as_array(nsubscriptions);
        let mut old_subs = Vec::new();
        for slot in subs_array.iter() {
            let slot = slot?;
            let sub = slot.read()?;
            old_subs.push(sub.clone());
            slot.cast().write(snapshot1_types::Subscription {
                userdata: sub.userdata,
                u: match sub.u {
                    types::SubscriptionU::Clock(c) => {
                        snapshot1_types::SubscriptionU::Clock(c.into())
                    }
                    types::SubscriptionU::FdRead(c) => {
                        snapshot1_types::SubscriptionU::FdRead(c.into())
                    }
                    types::SubscriptionU::FdWrite(c) => {
                        snapshot1_types::SubscriptionU::FdWrite(c.into())
                    }
                },
            })?;
        }
        let ret =
            Snapshot1::poll_oneoff(self, &subs.cast(), &events.cast(), nsubscriptions).await?;
        for (sub, slot) in old_subs.into_iter().zip(subs_array.iter()) {
            slot?.write(sub)?;
        }
        Ok(ret)
    }

    fn proc_exit(&mut self, status: types::Exitcode) -> anyhow::Error {
        Snapshot1::proc_exit(self, status)
    }

    fn proc_raise(&mut self, sig: types::Signal) -> Result<(), Error> {
        Snapshot1::proc_raise(self, sig.into())?;
        Ok(())
    }

    fn sched_yield(&mut self) -> Result<(), Error> {
        Snapshot1::sched_yield(self)?;
        Ok(())
    }

    fn random_get(&mut self, buf: &GuestPtr<'_, u8>, buf_len: types::Size) -> Result<(), Error> {
        Snapshot1::random_get(self, buf, buf_len)?;
        Ok(())
    }

    fn sock_recv(
        &mut self,
        _fd: types::Fd,
        _ri_data: &types::IovecArray<'_>,
        _ri_flags: types::Riflags,
    ) -> Result<(types::Size, types::Roflags), Error> {
        Err(Error::trap(anyhow::Error::msg("sock_recv unsupported")))
    }

    fn sock_send(
        &mut self,
        _fd: types::Fd,
        _si_data: &types::CiovecArray<'_>,
        _si_flags: types::Siflags,
    ) -> Result<types::Size, Error> {
        Err(Error::trap(anyhow::Error::msg("sock_send unsupported")))
    }

    fn sock_shutdown(&mut self, _fd: types::Fd, _how: types::Sdflags) -> Result<(), Error> {
        Err(Error::trap(anyhow::Error::msg("sock_shutdown unsupported")))
    }
}

fn assert_iovec_array_same() {
    // NB: this isn't enough to assert the types are the same, but it's
    // something. Additionally preview1 and preview0 aren't changing any more
    // and it's been manually verified that these two types are the same, so
    // it's ok to cast between them.
    assert_eq!(
        std::mem::size_of::<types::IovecArray<'_>>(),
        std::mem::size_of::<snapshot1_types::IovecArray<'_>>()
    );
}

fn assert_ciovec_array_same() {
    // NB: see above too
    assert_eq!(
        std::mem::size_of::<types::CiovecArray<'_>>(),
        std::mem::size_of::<snapshot1_types::CiovecArray<'_>>()
    );
}

impl From<snapshot1_types::Error> for Error {
    fn from(error: snapshot1_types::Error) -> Error {
        match error.downcast() {
            Ok(errno) => Error::from(types::Errno::from(errno)),
            Err(trap) => Error::trap(trap),
        }
    }
}

/// Fd is a newtype wrapper around u32. Unwrap and wrap it.
impl From<types::Fd> for snapshot1_types::Fd {
    fn from(fd: types::Fd) -> snapshot1_types::Fd {
        u32::from(fd).into()
    }
}

/// Fd is a newtype wrapper around u32. Unwrap and wrap it.
impl From<snapshot1_types::Fd> for types::Fd {
    fn from(fd: snapshot1_types::Fd) -> types::Fd {
        u32::from(fd).into()
    }
}

/// Trivial conversion between two c-style enums that have the exact same set of variants.
/// Could we do something unsafe and not list all these variants out? Probably, but doing
/// it this way doesn't bother me much. I copy-pasted the list of variants out of the
/// rendered rustdocs.
/// LLVM ought to compile these From impls into no-ops, inshallah
macro_rules! convert_enum {
    ($from:ty, $to:ty, $($var:ident),+) => {
        impl From<$from> for $to {
            fn from(e: $from) -> $to {
                match e {
                    $( <$from>::$var => <$to>::$var, )+
                }
            }
        }
    }
}
convert_enum!(
    snapshot1_types::Errno,
    types::Errno,
    Success,
    TooBig,
    Acces,
    Addrinuse,
    Addrnotavail,
    Afnosupport,
    Again,
    Already,
    Badf,
    Badmsg,
    Busy,
    Canceled,
    Child,
    Connaborted,
    Connrefused,
    Connreset,
    Deadlk,
    Destaddrreq,
    Dom,
    Dquot,
    Exist,
    Fault,
    Fbig,
    Hostunreach,
    Idrm,
    Ilseq,
    Inprogress,
    Intr,
    Inval,
    Io,
    Isconn,
    Isdir,
    Loop,
    Mfile,
    Mlink,
    Msgsize,
    Multihop,
    Nametoolong,
    Netdown,
    Netreset,
    Netunreach,
    Nfile,
    Nobufs,
    Nodev,
    Noent,
    Noexec,
    Nolck,
    Nolink,
    Nomem,
    Nomsg,
    Noprotoopt,
    Nospc,
    Nosys,
    Notconn,
    Notdir,
    Notempty,
    Notrecoverable,
    Notsock,
    Notsup,
    Notty,
    Nxio,
    Overflow,
    Ownerdead,
    Perm,
    Pipe,
    Proto,
    Protonosupport,
    Prototype,
    Range,
    Rofs,
    Spipe,
    Srch,
    Stale,
    Timedout,
    Txtbsy,
    Xdev,
    Notcapable
);
convert_enum!(
    types::Clockid,
    snapshot1_types::Clockid,
    Realtime,
    Monotonic,
    ProcessCputimeId,
    ThreadCputimeId
);

convert_enum!(
    types::Advice,
    snapshot1_types::Advice,
    Normal,
    Sequential,
    Random,
    Willneed,
    Dontneed,
    Noreuse
);
convert_enum!(
    snapshot1_types::Filetype,
    types::Filetype,
    Directory,
    BlockDevice,
    CharacterDevice,
    RegularFile,
    SocketDgram,
    SocketStream,
    SymbolicLink,
    Unknown
);
convert_enum!(types::Whence, snapshot1_types::Whence, Cur, End, Set);

convert_enum!(
    types::Signal,
    snapshot1_types::Signal,
    None,
    Hup,
    Int,
    Quit,
    Ill,
    Trap,
    Abrt,
    Bus,
    Fpe,
    Kill,
    Usr1,
    Segv,
    Usr2,
    Pipe,
    Alrm,
    Term,
    Chld,
    Cont,
    Stop,
    Tstp,
    Ttin,
    Ttou,
    Urg,
    Xcpu,
    Xfsz,
    Vtalrm,
    Prof,
    Winch,
    Poll,
    Pwr,
    Sys
);

/// Prestat isn't a c-style enum, its a union where the variant has a payload. Its the only one of
/// those we need to convert, so write it by hand.
impl From<snapshot1_types::Prestat> for types::Prestat {
    fn from(p: snapshot1_types::Prestat) -> types::Prestat {
        match p {
            snapshot1_types::Prestat::Dir(d) => types::Prestat::Dir(d.into()),
        }
    }
}

/// Trivial conversion between two structs that have the exact same set of fields,
/// with recursive descent into the field types.
macro_rules! convert_struct {
    ($from:ty, $to:path, $($field:ident),+) => {
        impl From<$from> for $to {
            fn from(e: $from) -> $to {
                $to {
                    $( $field: e.$field.into(), )+
                }
            }
        }
    }
}

convert_struct!(snapshot1_types::PrestatDir, types::PrestatDir, pr_name_len);
convert_struct!(
    snapshot1_types::Fdstat,
    types::Fdstat,
    fs_filetype,
    fs_rights_base,
    fs_rights_inheriting,
    fs_flags
);
convert_struct!(
    types::SubscriptionClock,
    snapshot1_types::SubscriptionClock,
    id,
    timeout,
    precision,
    flags
);
convert_struct!(
    types::SubscriptionFdReadwrite,
    snapshot1_types::SubscriptionFdReadwrite,
    file_descriptor
);

/// Snapshot1 Filestat is incompatible with Snapshot0 Filestat - the nlink
/// field is u32 on this Filestat, and u64 on theirs. If you've got more than
/// 2^32 links I don't know what to tell you
impl From<snapshot1_types::Filestat> for types::Filestat {
    fn from(f: snapshot1_types::Filestat) -> types::Filestat {
        types::Filestat {
            dev: f.dev.into(),
            ino: f.ino.into(),
            filetype: f.filetype.into(),
            nlink: f.nlink.try_into().unwrap_or(u32::MAX),
            size: f.size.into(),
            atim: f.atim.into(),
            mtim: f.mtim.into(),
            ctim: f.ctim.into(),
        }
    }
}

/// Trivial conversion between two bitflags that have the exact same set of flags.
macro_rules! convert_flags {
    ($from:ty, $to:ty, $($flag:ident),+) => {
        impl From<$from> for $to {
            fn from(f: $from) -> $to {
                let mut out = <$to>::empty();
                $(
                    if f.contains(<$from>::$flag) {
                        out |= <$to>::$flag;
                    }
                )+
                out
            }
        }
    }
}

/// Need to convert in both directions? This saves listing out the flags twice
macro_rules! convert_flags_bidirectional {
    ($from:ty, $to:ty, $($flag:tt)*) => {
        convert_flags!($from, $to, $($flag)*);
        convert_flags!($to, $from, $($flag)*);
    }
}

convert_flags_bidirectional!(
    snapshot1_types::Fdflags,
    types::Fdflags,
    APPEND,
    DSYNC,
    NONBLOCK,
    RSYNC,
    SYNC
);
convert_flags!(
    types::Lookupflags,
    snapshot1_types::Lookupflags,
    SYMLINK_FOLLOW
);
convert_flags!(
    types::Fstflags,
    snapshot1_types::Fstflags,
    ATIM,
    ATIM_NOW,
    MTIM,
    MTIM_NOW
);
convert_flags!(
    types::Oflags,
    snapshot1_types::Oflags,
    CREAT,
    DIRECTORY,
    EXCL,
    TRUNC
);
convert_flags_bidirectional!(
    types::Rights,
    snapshot1_types::Rights,
    FD_DATASYNC,
    FD_READ,
    FD_SEEK,
    FD_FDSTAT_SET_FLAGS,
    FD_SYNC,
    FD_TELL,
    FD_WRITE,
    FD_ADVISE,
    FD_ALLOCATE,
    PATH_CREATE_DIRECTORY,
    PATH_CREATE_FILE,
    PATH_LINK_SOURCE,
    PATH_LINK_TARGET,
    PATH_OPEN,
    FD_READDIR,
    PATH_READLINK,
    PATH_RENAME_SOURCE,
    PATH_RENAME_TARGET,
    PATH_FILESTAT_GET,
    PATH_FILESTAT_SET_SIZE,
    PATH_FILESTAT_SET_TIMES,
    FD_FILESTAT_GET,
    FD_FILESTAT_SET_SIZE,
    FD_FILESTAT_SET_TIMES,
    PATH_SYMLINK,
    PATH_REMOVE_DIRECTORY,
    PATH_UNLINK_FILE,
    POLL_FD_READWRITE,
    SOCK_SHUTDOWN
);
convert_flags!(
    types::Subclockflags,
    snapshot1_types::Subclockflags,
    SUBSCRIPTION_CLOCK_ABSTIME
);

impl From<GuestError> for types::Error {
    fn from(err: GuestError) -> Self {
        snapshot1_types::Error::from(err).into()
    }
}
