// Temporary for scaffolding this module out:
#![allow(unused_variables)]

use crate::wasi;

use core::borrow::Borrow;

use anyhow::Context;
use wiggle::GuestPtr;

pub struct WasiPreview1Adapter {/* all members private and only used inside this module. also, this struct should be Send. */}
impl WasiPreview1Adapter {
    // This should be the only public interface of this struct. It should take
    // no parameters: anything it needs from the preview 2 implementation
    // should be retrieved lazily.
    pub fn new() -> Self {
        todo!()
    }
}

// Any context that needs to support preview 1 will impl this trait. They can
// construct the needed member with WasiPreview1Adapter::new().
pub trait WasiPreview1View: Send {
    fn adapter(&self) -> &WasiPreview1Adapter;
    fn adapter_mut(&mut self) -> &mut WasiPreview1Adapter;
}

// This becomes the only way to add preview 1 support to a wasmtime (module)
// Linker:
pub fn add_to_linker<
    T: WasiPreview1View
        + wasi::environment::Host
        + wasi::exit::Host
        + wasi::filesystem::Host
        + wasi::monotonic_clock::Host
        + wasi::poll::Host
        + wasi::preopens::Host
        + wasi::random::Host
        + wasi::streams::Host
        + wasi::wall_clock::Host,
>(
    linker: &mut wasmtime::Linker<T>,
) -> anyhow::Result<()> {
    wasi_snapshot_preview1::add_to_linker(linker, |t| t)
}

// Generate the wasi_snapshot_preview1::WasiSnapshotPreview1 trait,
// and the module types.
// None of the generated modules, traits, or types should be used externally
// to this module.
wiggle::from_witx!({
    witx: ["$CARGO_MANIFEST_DIR/witx/wasi_snapshot_preview1.witx"],
    errors: { errno => trappable Error },
    async: *,
});

impl wiggle::GuestErrorType for types::Errno {
    fn success() -> Self {
        Self::Success
    }
}

impl From<anyhow::Error> for types::Error {
    fn from(e: anyhow::Error) -> Self {
        types::Error::trap(e)
    }
}

type ErrnoResult<T> = Result<T, types::Errno>;

fn write_bytes<'a>(
    ptr: impl Borrow<GuestPtr<'a, u8>>,
    buf: impl AsRef<[u8]>,
) -> ErrnoResult<GuestPtr<'a, u8>> {
    // NOTE: legacy implementation always returns Inval errno

    let buf = buf.as_ref();
    let len = buf.len().try_into().or(Err(types::Errno::Inval))?;

    let ptr = ptr.borrow();
    ptr.as_array(len)
        .copy_from_slice(buf)
        .or(Err(types::Errno::Inval))?;
    ptr.add(len).or(Err(types::Errno::Inval))
}

fn write_byte<'a>(ptr: impl Borrow<GuestPtr<'a, u8>>, byte: u8) -> ErrnoResult<GuestPtr<'a, u8>> {
    let ptr = ptr.borrow();
    ptr.write(byte).or(Err(types::Errno::Inval))?;
    ptr.add(1).or(Err(types::Errno::Inval))
}

// Implement the WasiSnapshotPreview1 trait using only the traits that are
// required for T, i.e., in terms of the preview 2 wit interface, and state
// stored in the WasiPreview1Adapter struct.
#[wiggle::async_trait]
impl<
        T: WasiPreview1View
            + wasi::environment::Host
            + wasi::exit::Host
            + wasi::filesystem::Host
            + wasi::monotonic_clock::Host
            + wasi::poll::Host
            + wasi::preopens::Host
            + wasi::random::Host
            + wasi::streams::Host
            + wasi::wall_clock::Host,
    > wasi_snapshot_preview1::WasiSnapshotPreview1 for T
{
    async fn args_get<'b>(
        &mut self,
        argv: &GuestPtr<'b, GuestPtr<'b, u8>>,
        argv_buf: &GuestPtr<'b, u8>,
    ) -> Result<(), types::Error> {
        self.get_arguments()
            .await
            .context("failed to call `get-arguments`")?
            .into_iter()
            .try_fold(
                (*argv, *argv_buf),
                |(argv, argv_buf), arg| -> ErrnoResult<_> {
                    // NOTE: legacy implementation always returns Inval errno

                    argv.write(argv_buf).map_err(|_| types::Errno::Inval)?;
                    let argv = argv.add(1).map_err(|_| types::Errno::Inval)?;

                    let argv_buf = write_bytes(argv_buf, arg)?;
                    let argv_buf = write_byte(argv_buf, 0)?;

                    Ok((argv, argv_buf))
                },
            )?;
        Ok(())
    }

    async fn args_sizes_get(&mut self) -> Result<(types::Size, types::Size), types::Error> {
        todo!()
    }

    async fn environ_get<'b>(
        &mut self,
        environ: &GuestPtr<'b, GuestPtr<'b, u8>>,
        environ_buf: &GuestPtr<'b, u8>,
    ) -> Result<(), types::Error> {
        todo!()
    }

    async fn environ_sizes_get(&mut self) -> Result<(types::Size, types::Size), types::Error> {
        todo!()
    }

    async fn clock_res_get(
        &mut self,
        id: types::Clockid,
    ) -> Result<types::Timestamp, types::Error> {
        todo!()
    }

    async fn clock_time_get(
        &mut self,
        id: types::Clockid,
        precision: types::Timestamp,
    ) -> Result<types::Timestamp, types::Error> {
        todo!()
    }

    async fn fd_advise(
        &mut self,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
        advice: types::Advice,
    ) -> Result<(), types::Error> {
        todo!()
    }

    async fn fd_allocate(
        &mut self,
        fd: types::Fd,
        _offset: types::Filesize,
        _len: types::Filesize,
    ) -> Result<(), types::Error> {
        todo!()
    }

    async fn fd_close(&mut self, fd: types::Fd) -> Result<(), types::Error> {
        todo!()
    }

    async fn fd_datasync(&mut self, fd: types::Fd) -> Result<(), types::Error> {
        todo!()
    }

    async fn fd_fdstat_get(&mut self, fd: types::Fd) -> Result<types::Fdstat, types::Error> {
        todo!()
    }

    async fn fd_fdstat_set_flags(
        &mut self,
        fd: types::Fd,
        flags: types::Fdflags,
    ) -> Result<(), types::Error> {
        todo!()
    }

    async fn fd_fdstat_set_rights(
        &mut self,
        fd: types::Fd,
        fs_rights_base: types::Rights,
        fs_rights_inheriting: types::Rights,
    ) -> Result<(), types::Error> {
        todo!()
    }

    async fn fd_filestat_get(&mut self, fd: types::Fd) -> Result<types::Filestat, types::Error> {
        todo!()
    }

    async fn fd_filestat_set_size(
        &mut self,
        fd: types::Fd,
        size: types::Filesize,
    ) -> Result<(), types::Error> {
        todo!()
    }

    async fn fd_filestat_set_times(
        &mut self,
        fd: types::Fd,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<(), types::Error> {
        todo!()
    }

    async fn fd_read<'a>(
        &mut self,
        fd: types::Fd,
        iovs: &types::IovecArray<'a>,
    ) -> Result<types::Size, types::Error> {
        todo!()
    }

    async fn fd_pread<'a>(
        &mut self,
        fd: types::Fd,
        iovs: &types::IovecArray<'a>,
        offset: types::Filesize,
    ) -> Result<types::Size, types::Error> {
        todo!()
    }

    async fn fd_write<'a>(
        &mut self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'a>,
    ) -> Result<types::Size, types::Error> {
        todo!()
    }

    async fn fd_pwrite<'a>(
        &mut self,
        fd: types::Fd,
        ciovs: &types::CiovecArray<'a>,
        offset: types::Filesize,
    ) -> Result<types::Size, types::Error> {
        todo!()
    }

    async fn fd_prestat_get(&mut self, fd: types::Fd) -> Result<types::Prestat, types::Error> {
        todo!()
    }

    async fn fd_prestat_dir_name<'a>(
        &mut self,
        fd: types::Fd,
        path: &GuestPtr<'a, u8>,
        path_max_len: types::Size,
    ) -> Result<(), types::Error> {
        todo!()
    }
    async fn fd_renumber(&mut self, from: types::Fd, to: types::Fd) -> Result<(), types::Error> {
        todo!()
    }

    async fn fd_seek(
        &mut self,
        fd: types::Fd,
        offset: types::Filedelta,
        whence: types::Whence,
    ) -> Result<types::Filesize, types::Error> {
        todo!()
    }

    async fn fd_sync(&mut self, fd: types::Fd) -> Result<(), types::Error> {
        todo!()
    }

    async fn fd_tell(&mut self, fd: types::Fd) -> Result<types::Filesize, types::Error> {
        todo!()
    }

    async fn fd_readdir<'a>(
        &mut self,
        fd: types::Fd,
        buf: &GuestPtr<'a, u8>,
        buf_len: types::Size,
        cookie: types::Dircookie,
    ) -> Result<types::Size, types::Error> {
        todo!()
    }

    async fn path_create_directory<'a>(
        &mut self,
        dirfd: types::Fd,
        path: &GuestPtr<'a, str>,
    ) -> Result<(), types::Error> {
        todo!()
    }

    async fn path_filestat_get<'a>(
        &mut self,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestPtr<'a, str>,
    ) -> Result<types::Filestat, types::Error> {
        todo!()
    }

    async fn path_filestat_set_times<'a>(
        &mut self,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: &GuestPtr<'a, str>,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<(), types::Error> {
        todo!()
    }

    async fn path_link<'a>(
        &mut self,
        src_fd: types::Fd,
        src_flags: types::Lookupflags,
        src_path: &GuestPtr<'a, str>,
        target_fd: types::Fd,
        target_path: &GuestPtr<'a, str>,
    ) -> Result<(), types::Error> {
        todo!()
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
    ) -> Result<types::Fd, types::Error> {
        todo!()
    }

    async fn path_readlink<'a>(
        &mut self,
        dirfd: types::Fd,
        path: &GuestPtr<'a, str>,
        buf: &GuestPtr<'a, u8>,
        buf_len: types::Size,
    ) -> Result<types::Size, types::Error> {
        todo!()
    }

    async fn path_remove_directory<'a>(
        &mut self,
        dirfd: types::Fd,
        path: &GuestPtr<'a, str>,
    ) -> Result<(), types::Error> {
        todo!()
    }

    async fn path_rename<'a>(
        &mut self,
        src_fd: types::Fd,
        src_path: &GuestPtr<'a, str>,
        dest_fd: types::Fd,
        dest_path: &GuestPtr<'a, str>,
    ) -> Result<(), types::Error> {
        todo!()
    }

    async fn path_symlink<'a>(
        &mut self,
        src_path: &GuestPtr<'a, str>,
        dirfd: types::Fd,
        dest_path: &GuestPtr<'a, str>,
    ) -> Result<(), types::Error> {
        todo!()
    }

    async fn path_unlink_file<'a>(
        &mut self,
        dirfd: types::Fd,
        path: &GuestPtr<'a, str>,
    ) -> Result<(), types::Error> {
        todo!()
    }

    async fn poll_oneoff<'a>(
        &mut self,
        subs: &GuestPtr<'a, types::Subscription>,
        events: &GuestPtr<'a, types::Event>,
        nsubscriptions: types::Size,
    ) -> Result<types::Size, types::Error> {
        todo!()
    }

    async fn proc_exit(&mut self, status: types::Exitcode) -> anyhow::Error {
        todo!()
    }

    async fn proc_raise(&mut self, _sig: types::Signal) -> Result<(), types::Error> {
        todo!()
    }

    async fn sched_yield(&mut self) -> Result<(), types::Error> {
        todo!()
    }

    async fn random_get<'a>(
        &mut self,
        buf: &GuestPtr<'a, u8>,
        buf_len: types::Size,
    ) -> Result<(), types::Error> {
        todo!()
    }

    async fn sock_accept(
        &mut self,
        fd: types::Fd,
        flags: types::Fdflags,
    ) -> Result<types::Fd, types::Error> {
        todo!()
    }

    async fn sock_recv<'a>(
        &mut self,
        fd: types::Fd,
        ri_data: &types::IovecArray<'a>,
        ri_flags: types::Riflags,
    ) -> Result<(types::Size, types::Roflags), types::Error> {
        todo!()
    }

    async fn sock_send<'a>(
        &mut self,
        fd: types::Fd,
        si_data: &types::CiovecArray<'a>,
        _si_flags: types::Siflags,
    ) -> Result<types::Size, types::Error> {
        todo!()
    }

    async fn sock_shutdown(
        &mut self,
        fd: types::Fd,
        how: types::Sdflags,
    ) -> Result<(), types::Error> {
        todo!()
    }
}
