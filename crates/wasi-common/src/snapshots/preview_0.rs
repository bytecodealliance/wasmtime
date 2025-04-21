use crate::file::TableFileExt;
use crate::sched::{
    subscription::{RwEventFlags, SubscriptionResult},
    Poll, Userdata,
};
use crate::snapshots::preview_1::types as snapshot1_types;
use crate::snapshots::preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1 as Snapshot1;
use crate::{ErrorExt, WasiCtx};
use cap_std::time::Duration;
use std::collections::HashSet;
use wiggle::{GuestMemory, GuestPtr};

wiggle::from_witx!({
    witx: ["witx/preview0/wasi_unstable.witx"],
    errors: { errno => trappable Error },
    async: *,
    wasmtime: false,
});

use types::Error;

impl ErrorExt for Error {
    fn not_found() -> Self {
        types::Errno::Noent.into()
    }
    fn too_big() -> Self {
        types::Errno::TooBig.into()
    }
    fn badf() -> Self {
        types::Errno::Badf.into()
    }
    fn exist() -> Self {
        types::Errno::Exist.into()
    }
    fn illegal_byte_sequence() -> Self {
        types::Errno::Ilseq.into()
    }
    fn invalid_argument() -> Self {
        types::Errno::Inval.into()
    }
    fn io() -> Self {
        types::Errno::Io.into()
    }
    fn name_too_long() -> Self {
        types::Errno::Nametoolong.into()
    }
    fn not_dir() -> Self {
        types::Errno::Notdir.into()
    }
    fn not_supported() -> Self {
        types::Errno::Notsup.into()
    }
    fn overflow() -> Self {
        types::Errno::Overflow.into()
    }
    fn range() -> Self {
        types::Errno::Range.into()
    }
    fn seek_pipe() -> Self {
        types::Errno::Spipe.into()
    }
    fn perm() -> Self {
        types::Errno::Perm.into()
    }
}

impl wiggle::GuestErrorType for types::Errno {
    fn success() -> Self {
        Self::Success
    }
}

impl From<wiggle::GuestError> for Error {
    fn from(err: wiggle::GuestError) -> Error {
        snapshot1_types::Error::from(err).into()
    }
}

impl From<snapshot1_types::Error> for Error {
    fn from(error: snapshot1_types::Error) -> Error {
        match error.downcast() {
            Ok(errno) => Error::from(types::Errno::from(errno)),
            Err(trap) => Error::trap(trap),
        }
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(_err: std::num::TryFromIntError) -> Error {
        types::Errno::Overflow.into()
    }
}

// Type conversions
// The vast majority of the types defined in `types` and `snapshot1_types` are identical. However,
// since they are defined in separate places for mechanical (wiggle) reasons, we need to manually
// define conversion functions between them.
// Below we have defined these functions as they are needed.

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

// This implementation, wherever possible, delegates directly to the Snapshot1 implementation,
// performing the no-op type conversions along the way.
#[wiggle::async_trait]
impl wasi_unstable::WasiUnstable for WasiCtx {
    async fn args_get(
        &mut self,
        memory: &mut GuestMemory<'_>,
        argv: GuestPtr<GuestPtr<u8>>,
        argv_buf: GuestPtr<u8>,
    ) -> Result<(), Error> {
        Snapshot1::args_get(self, memory, argv, argv_buf).await?;
        Ok(())
    }

    async fn args_sizes_get(
        &mut self,
        memory: &mut GuestMemory<'_>,
    ) -> Result<(types::Size, types::Size), Error> {
        let s = Snapshot1::args_sizes_get(self, memory).await?;
        Ok(s)
    }

    async fn environ_get(
        &mut self,
        memory: &mut GuestMemory<'_>,
        environ: GuestPtr<GuestPtr<u8>>,
        environ_buf: GuestPtr<u8>,
    ) -> Result<(), Error> {
        Snapshot1::environ_get(self, memory, environ, environ_buf).await?;
        Ok(())
    }

    async fn environ_sizes_get(
        &mut self,
        memory: &mut GuestMemory<'_>,
    ) -> Result<(types::Size, types::Size), Error> {
        let s = Snapshot1::environ_sizes_get(self, memory).await?;
        Ok(s)
    }

    async fn clock_res_get(
        &mut self,
        memory: &mut GuestMemory<'_>,
        id: types::Clockid,
    ) -> Result<types::Timestamp, Error> {
        let t = Snapshot1::clock_res_get(self, memory, id.into()).await?;
        Ok(t)
    }

    async fn clock_time_get(
        &mut self,
        memory: &mut GuestMemory<'_>,
        id: types::Clockid,
        precision: types::Timestamp,
    ) -> Result<types::Timestamp, Error> {
        let t = Snapshot1::clock_time_get(self, memory, id.into(), precision).await?;
        Ok(t)
    }

    async fn fd_advise(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
        advice: types::Advice,
    ) -> Result<(), Error> {
        Snapshot1::fd_advise(self, memory, fd.into(), offset, len, advice.into()).await?;
        Ok(())
    }

    async fn fd_allocate(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
    ) -> Result<(), Error> {
        Snapshot1::fd_allocate(self, memory, fd.into(), offset, len).await?;
        Ok(())
    }

    async fn fd_close(&mut self, memory: &mut GuestMemory<'_>, fd: types::Fd) -> Result<(), Error> {
        Snapshot1::fd_close(self, memory, fd.into()).await?;
        Ok(())
    }

    async fn fd_datasync(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
    ) -> Result<(), Error> {
        Snapshot1::fd_datasync(self, memory, fd.into()).await?;
        Ok(())
    }

    async fn fd_fdstat_get(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
    ) -> Result<types::Fdstat, Error> {
        Ok(Snapshot1::fd_fdstat_get(self, memory, fd.into())
            .await?
            .into())
    }

    async fn fd_fdstat_set_flags(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        flags: types::Fdflags,
    ) -> Result<(), Error> {
        Snapshot1::fd_fdstat_set_flags(self, memory, fd.into(), flags.into()).await?;
        Ok(())
    }

    async fn fd_fdstat_set_rights(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        fs_rights_base: types::Rights,
        fs_rights_inheriting: types::Rights,
    ) -> Result<(), Error> {
        Snapshot1::fd_fdstat_set_rights(
            self,
            memory,
            fd.into(),
            fs_rights_base.into(),
            fs_rights_inheriting.into(),
        )
        .await?;
        Ok(())
    }

    async fn fd_filestat_get(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
    ) -> Result<types::Filestat, Error> {
        Ok(Snapshot1::fd_filestat_get(self, memory, fd.into())
            .await?
            .into())
    }

    async fn fd_filestat_set_size(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        size: types::Filesize,
    ) -> Result<(), Error> {
        Snapshot1::fd_filestat_set_size(self, memory, fd.into(), size).await?;
        Ok(())
    }

    async fn fd_filestat_set_times(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<(), Error> {
        Snapshot1::fd_filestat_set_times(self, memory, fd.into(), atim, mtim, fst_flags.into())
            .await?;
        Ok(())
    }

    // NOTE on fd_read, fd_pread, fd_write, fd_pwrite implementations:
    // these cast their pointers from preview0 vectors to preview1 vectors and
    // this only works because the representation didn't change between preview0
    // and preview1.

    async fn fd_read(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        iovs: types::IovecArray,
    ) -> Result<types::Size, Error> {
        Ok(Snapshot1::fd_read(self, memory, fd.into(), iovs.cast()).await?)
    }

    async fn fd_pread(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        iovs: types::IovecArray,
        offset: types::Filesize,
    ) -> Result<types::Size, Error> {
        Ok(Snapshot1::fd_pread(self, memory, fd.into(), iovs.cast(), offset).await?)
    }

    async fn fd_write(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        ciovs: types::CiovecArray,
    ) -> Result<types::Size, Error> {
        Ok(Snapshot1::fd_write(self, memory, fd.into(), ciovs.cast()).await?)
    }

    async fn fd_pwrite(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        ciovs: types::CiovecArray,
        offset: types::Filesize,
    ) -> Result<types::Size, Error> {
        Ok(Snapshot1::fd_pwrite(self, memory, fd.into(), ciovs.cast(), offset).await?)
    }

    async fn fd_prestat_get(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
    ) -> Result<types::Prestat, Error> {
        Ok(Snapshot1::fd_prestat_get(self, memory, fd.into())
            .await?
            .into())
    }

    async fn fd_prestat_dir_name(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        path: GuestPtr<u8>,
        path_max_len: types::Size,
    ) -> Result<(), Error> {
        Snapshot1::fd_prestat_dir_name(self, memory, fd.into(), path, path_max_len).await?;
        Ok(())
    }

    async fn fd_renumber(
        &mut self,
        memory: &mut GuestMemory<'_>,
        from: types::Fd,
        to: types::Fd,
    ) -> Result<(), Error> {
        Snapshot1::fd_renumber(self, memory, from.into(), to.into()).await?;
        Ok(())
    }

    async fn fd_seek(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        offset: types::Filedelta,
        whence: types::Whence,
    ) -> Result<types::Filesize, Error> {
        Ok(Snapshot1::fd_seek(self, memory, fd.into(), offset, whence.into()).await?)
    }

    async fn fd_sync(&mut self, memory: &mut GuestMemory<'_>, fd: types::Fd) -> Result<(), Error> {
        Snapshot1::fd_sync(self, memory, fd.into()).await?;
        Ok(())
    }

    async fn fd_tell(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
    ) -> Result<types::Filesize, Error> {
        Ok(Snapshot1::fd_tell(self, memory, fd.into()).await?)
    }

    async fn fd_readdir(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        buf: GuestPtr<u8>,
        buf_len: types::Size,
        cookie: types::Dircookie,
    ) -> Result<types::Size, Error> {
        Ok(Snapshot1::fd_readdir(self, memory, fd.into(), buf, buf_len, cookie).await?)
    }

    async fn path_create_directory(
        &mut self,
        memory: &mut GuestMemory<'_>,
        dirfd: types::Fd,
        path: GuestPtr<str>,
    ) -> Result<(), Error> {
        Snapshot1::path_create_directory(self, memory, dirfd.into(), path).await?;
        Ok(())
    }

    async fn path_filestat_get(
        &mut self,
        memory: &mut GuestMemory<'_>,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: GuestPtr<str>,
    ) -> Result<types::Filestat, Error> {
        Ok(
            Snapshot1::path_filestat_get(self, memory, dirfd.into(), flags.into(), path)
                .await?
                .into(),
        )
    }

    async fn path_filestat_set_times(
        &mut self,
        memory: &mut GuestMemory<'_>,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: GuestPtr<str>,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<(), Error> {
        Snapshot1::path_filestat_set_times(
            self,
            memory,
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

    async fn path_link(
        &mut self,
        memory: &mut GuestMemory<'_>,
        src_fd: types::Fd,
        src_flags: types::Lookupflags,
        src_path: GuestPtr<str>,
        target_fd: types::Fd,
        target_path: GuestPtr<str>,
    ) -> Result<(), Error> {
        Snapshot1::path_link(
            self,
            memory,
            src_fd.into(),
            src_flags.into(),
            src_path,
            target_fd.into(),
            target_path,
        )
        .await?;
        Ok(())
    }

    async fn path_open(
        &mut self,
        memory: &mut GuestMemory<'_>,
        dirfd: types::Fd,
        dirflags: types::Lookupflags,
        path: GuestPtr<str>,
        oflags: types::Oflags,
        fs_rights_base: types::Rights,
        fs_rights_inheriting: types::Rights,
        fdflags: types::Fdflags,
    ) -> Result<types::Fd, Error> {
        Ok(Snapshot1::path_open(
            self,
            memory,
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

    async fn path_readlink(
        &mut self,
        memory: &mut GuestMemory<'_>,
        dirfd: types::Fd,
        path: GuestPtr<str>,
        buf: GuestPtr<u8>,
        buf_len: types::Size,
    ) -> Result<types::Size, Error> {
        Ok(Snapshot1::path_readlink(self, memory, dirfd.into(), path, buf, buf_len).await?)
    }

    async fn path_remove_directory(
        &mut self,
        memory: &mut GuestMemory<'_>,
        dirfd: types::Fd,
        path: GuestPtr<str>,
    ) -> Result<(), Error> {
        Snapshot1::path_remove_directory(self, memory, dirfd.into(), path).await?;
        Ok(())
    }

    async fn path_rename(
        &mut self,
        memory: &mut GuestMemory<'_>,
        src_fd: types::Fd,
        src_path: GuestPtr<str>,
        dest_fd: types::Fd,
        dest_path: GuestPtr<str>,
    ) -> Result<(), Error> {
        Snapshot1::path_rename(
            self,
            memory,
            src_fd.into(),
            src_path,
            dest_fd.into(),
            dest_path,
        )
        .await?;
        Ok(())
    }

    async fn path_symlink(
        &mut self,
        memory: &mut GuestMemory<'_>,
        src_path: GuestPtr<str>,
        dirfd: types::Fd,
        dest_path: GuestPtr<str>,
    ) -> Result<(), Error> {
        Snapshot1::path_symlink(self, memory, src_path, dirfd.into(), dest_path).await?;
        Ok(())
    }

    async fn path_unlink_file(
        &mut self,
        memory: &mut GuestMemory<'_>,
        dirfd: types::Fd,
        path: GuestPtr<str>,
    ) -> Result<(), Error> {
        Snapshot1::path_unlink_file(self, memory, dirfd.into(), path).await?;
        Ok(())
    }

    // NOTE on poll_oneoff implementation:
    // Like fd_write and friends, the arguments and return values are behind GuestPtrs,
    // so they are not values we can convert and pass to the poll_oneoff in Snapshot1.
    // Instead, we have copied the implementation of these functions from the Snapshot1 code.
    // The implementations are identical, but the `types::` in scope locally is different.
    // The bodies of these functions is mostly about converting the GuestPtr and types::-based
    // representation to use the Poll abstraction.
    async fn poll_oneoff(
        &mut self,
        memory: &mut GuestMemory<'_>,
        subs: GuestPtr<types::Subscription>,
        events: GuestPtr<types::Event>,
        nsubscriptions: types::Size,
    ) -> Result<types::Size, Error> {
        if nsubscriptions == 0 {
            return Err(Error::invalid_argument().context("nsubscriptions must be nonzero"));
        }

        // Special-case a `poll_oneoff` which is just sleeping on a single
        // relative timer event, such as what WASI libc uses to implement sleep
        // functions. This supports all clock IDs, because POSIX says that
        // `clock_settime` doesn't effect relative sleeps.
        if nsubscriptions == 1 {
            let sub = memory.read(subs)?;
            if let types::SubscriptionU::Clock(clocksub) = sub.u {
                if !clocksub
                    .flags
                    .contains(types::Subclockflags::SUBSCRIPTION_CLOCK_ABSTIME)
                {
                    self.sched
                        .sleep(Duration::from_nanos(clocksub.timeout))
                        .await?;
                    memory.write(
                        events,
                        types::Event {
                            userdata: sub.userdata,
                            error: types::Errno::Success,
                            type_: types::Eventtype::Clock,
                            fd_readwrite: fd_readwrite_empty(),
                        },
                    )?;
                    return Ok(1);
                }
            }
        }

        let table = &self.table;
        let mut sub_fds: HashSet<types::Fd> = HashSet::new();
        // We need these refmuts to outlive Poll, which will hold the &mut dyn WasiFile inside
        let mut reads: Vec<(u32, Userdata)> = Vec::new();
        let mut writes: Vec<(u32, Userdata)> = Vec::new();
        let mut poll = Poll::new();

        let subs = subs.as_array(nsubscriptions);
        for sub_elem in subs.iter() {
            let sub_ptr = sub_elem?;
            let sub = memory.read(sub_ptr)?;
            match sub.u {
                types::SubscriptionU::Clock(clocksub) => match clocksub.id {
                    types::Clockid::Monotonic => {
                        let clock = self.clocks.monotonic()?;
                        let precision = Duration::from_nanos(clocksub.precision);
                        let duration = Duration::from_nanos(clocksub.timeout);
                        let start = if clocksub
                            .flags
                            .contains(types::Subclockflags::SUBSCRIPTION_CLOCK_ABSTIME)
                        {
                            clock.creation_time
                        } else {
                            clock.abs_clock.now(precision)
                        };
                        let deadline = start
                            .checked_add(duration)
                            .ok_or_else(|| Error::overflow().context("deadline"))?;
                        poll.subscribe_monotonic_clock(
                            &*clock.abs_clock,
                            deadline,
                            precision,
                            sub.userdata.into(),
                        )
                    }
                    _ => Err(Error::invalid_argument()
                        .context("timer subscriptions only support monotonic timer"))?,
                },
                types::SubscriptionU::FdRead(readsub) => {
                    let fd = readsub.file_descriptor;
                    if sub_fds.contains(&fd) {
                        return Err(Error::invalid_argument()
                            .context("Fd can be subscribed to at most once per poll"));
                    } else {
                        sub_fds.insert(fd);
                    }
                    table.get_file(u32::from(fd))?;
                    reads.push((u32::from(fd), sub.userdata.into()));
                }
                types::SubscriptionU::FdWrite(writesub) => {
                    let fd = writesub.file_descriptor;
                    if sub_fds.contains(&fd) {
                        return Err(Error::invalid_argument()
                            .context("Fd can be subscribed to at most once per poll"));
                    } else {
                        sub_fds.insert(fd);
                    }
                    table.get_file(u32::from(fd))?;
                    writes.push((u32::from(fd), sub.userdata.into()));
                }
            }
        }

        self.sched.poll_oneoff(&mut poll).await?;

        let results = poll.results();
        let num_results = results.len();
        assert!(
            num_results <= nsubscriptions as usize,
            "results exceeds subscriptions"
        );
        let events = events.as_array(
            num_results
                .try_into()
                .expect("not greater than nsubscriptions"),
        );
        for ((result, userdata), event_elem) in results.into_iter().zip(events.iter()) {
            let event_ptr = event_elem?;
            let userdata: types::Userdata = userdata.into();
            memory.write(
                event_ptr,
                match result {
                    SubscriptionResult::Read(r) => {
                        let type_ = types::Eventtype::FdRead;
                        match r {
                            Ok((nbytes, flags)) => types::Event {
                                userdata,
                                error: types::Errno::Success,
                                type_,
                                fd_readwrite: types::EventFdReadwrite {
                                    nbytes,
                                    flags: types::Eventrwflags::from(&flags),
                                },
                            },
                            Err(e) => types::Event {
                                userdata,
                                error: types::Errno::from(e.downcast().map_err(Error::trap)?),
                                type_,
                                fd_readwrite: fd_readwrite_empty(),
                            },
                        }
                    }
                    SubscriptionResult::Write(r) => {
                        let type_ = types::Eventtype::FdWrite;
                        match r {
                            Ok((nbytes, flags)) => types::Event {
                                userdata,
                                error: types::Errno::Success,
                                type_,
                                fd_readwrite: types::EventFdReadwrite {
                                    nbytes,
                                    flags: types::Eventrwflags::from(&flags),
                                },
                            },
                            Err(e) => types::Event {
                                userdata,
                                error: types::Errno::from(e.downcast().map_err(Error::trap)?),
                                type_,
                                fd_readwrite: fd_readwrite_empty(),
                            },
                        }
                    }
                    SubscriptionResult::MonotonicClock(r) => {
                        let type_ = types::Eventtype::Clock;
                        types::Event {
                            userdata,
                            error: match r {
                                Ok(()) => types::Errno::Success,
                                Err(e) => types::Errno::from(e.downcast().map_err(Error::trap)?),
                            },
                            type_,
                            fd_readwrite: fd_readwrite_empty(),
                        }
                    }
                },
            )?;
        }

        Ok(num_results.try_into().expect("results fit into memory"))
    }

    async fn proc_exit(
        &mut self,
        memory: &mut GuestMemory<'_>,
        status: types::Exitcode,
    ) -> anyhow::Error {
        Snapshot1::proc_exit(self, memory, status).await
    }

    async fn proc_raise(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        _sig: types::Signal,
    ) -> Result<(), Error> {
        Err(Error::trap(anyhow::Error::msg("proc_raise unsupported")))
    }

    async fn sched_yield(&mut self, memory: &mut GuestMemory<'_>) -> Result<(), Error> {
        Snapshot1::sched_yield(self, memory).await?;
        Ok(())
    }

    async fn random_get(
        &mut self,
        memory: &mut GuestMemory<'_>,
        buf: GuestPtr<u8>,
        buf_len: types::Size,
    ) -> Result<(), Error> {
        Snapshot1::random_get(self, memory, buf, buf_len).await?;
        Ok(())
    }

    async fn sock_recv(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        _fd: types::Fd,
        _ri_data: types::IovecArray,
        _ri_flags: types::Riflags,
    ) -> Result<(types::Size, types::Roflags), Error> {
        Err(Error::trap(anyhow::Error::msg("sock_recv unsupported")))
    }

    async fn sock_send(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        _fd: types::Fd,
        _si_data: types::CiovecArray,
        _si_flags: types::Siflags,
    ) -> Result<types::Size, Error> {
        Err(Error::trap(anyhow::Error::msg("sock_send unsupported")))
    }

    async fn sock_shutdown(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        _fd: types::Fd,
        _how: types::Sdflags,
    ) -> Result<(), Error> {
        Err(Error::trap(anyhow::Error::msg("sock_shutdown unsupported")))
    }
}

impl From<&RwEventFlags> for types::Eventrwflags {
    fn from(flags: &RwEventFlags) -> types::Eventrwflags {
        let mut out = types::Eventrwflags::empty();
        if flags.contains(RwEventFlags::HANGUP) {
            out = out | types::Eventrwflags::FD_READWRITE_HANGUP;
        }
        out
    }
}

fn fd_readwrite_empty() -> types::EventFdReadwrite {
    types::EventFdReadwrite {
        nbytes: 0,
        flags: types::Eventrwflags::empty(),
    }
}
