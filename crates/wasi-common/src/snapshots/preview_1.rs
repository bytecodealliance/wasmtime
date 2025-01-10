use crate::{
    dir::{DirEntry, OpenResult, ReaddirCursor, ReaddirEntity, TableDirExt},
    file::{
        Advice, FdFlags, FdStat, FileAccessMode, FileEntry, FileType, Filestat, OFlags, RiFlags,
        RoFlags, SdFlags, SiFlags, TableFileExt, WasiFile,
    },
    sched::{
        subscription::{RwEventFlags, SubscriptionResult},
        Poll, Userdata,
    },
    I32Exit, SystemTimeSpec, WasiCtx,
};
use cap_std::time::{Duration, SystemClock};
use std::borrow::Cow;
use std::io::{IoSlice, IoSliceMut};
use std::ops::Deref;
use std::sync::Arc;
use wiggle::GuestMemory;
use wiggle::GuestPtr;

pub mod error;
use error::{Error, ErrorExt};

// Limit the size of intermediate buffers when copying to WebAssembly shared
// memory.
pub(crate) const MAX_SHARED_BUFFER_SIZE: usize = 1 << 16;

wiggle::from_witx!({
    witx: ["$CARGO_MANIFEST_DIR/witx/preview1/wasi_snapshot_preview1.witx"],
    errors: { errno => trappable Error },
    // Note: not every function actually needs to be async, however, nearly all of them do, and
    // keeping that set the same in this macro and the wasmtime_wiggle / lucet_wiggle macros is
    // tedious, and there is no cost to having a sync function be async in this case.
    async: *,
    wasmtime: false,
});

impl wiggle::GuestErrorType for types::Errno {
    fn success() -> Self {
        Self::Success
    }
}

#[wiggle::async_trait]
impl wasi_snapshot_preview1::WasiSnapshotPreview1 for WasiCtx {
    async fn args_get(
        &mut self,
        memory: &mut GuestMemory<'_>,
        argv: GuestPtr<GuestPtr<u8>>,
        argv_buf: GuestPtr<u8>,
    ) -> Result<(), Error> {
        self.args.write_to_guest(memory, argv_buf, argv)
    }

    async fn args_sizes_get(
        &mut self,
        _memory: &mut GuestMemory<'_>,
    ) -> Result<(types::Size, types::Size), Error> {
        Ok((self.args.number_elements(), self.args.cumulative_size()))
    }

    async fn environ_get(
        &mut self,
        memory: &mut GuestMemory<'_>,
        environ: GuestPtr<GuestPtr<u8>>,
        environ_buf: GuestPtr<u8>,
    ) -> Result<(), Error> {
        self.env.write_to_guest(memory, environ_buf, environ)
    }

    async fn environ_sizes_get(
        &mut self,
        _memory: &mut GuestMemory<'_>,
    ) -> Result<(types::Size, types::Size), Error> {
        Ok((self.env.number_elements(), self.env.cumulative_size()))
    }

    async fn clock_res_get(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        id: types::Clockid,
    ) -> Result<types::Timestamp, Error> {
        let resolution = match id {
            types::Clockid::Realtime => Ok(self.clocks.system()?.resolution()),
            types::Clockid::Monotonic => Ok(self.clocks.monotonic()?.abs_clock.resolution()),
            types::Clockid::ProcessCputimeId | types::Clockid::ThreadCputimeId => {
                Err(Error::badf().context("process and thread clocks are not supported"))
            }
        }?;
        Ok(resolution.as_nanos().try_into()?)
    }

    async fn clock_time_get(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        id: types::Clockid,
        precision: types::Timestamp,
    ) -> Result<types::Timestamp, Error> {
        let precision = Duration::from_nanos(precision);
        match id {
            types::Clockid::Realtime => {
                let now = self.clocks.system()?.now(precision).into_std();
                let d = now
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .map_err(|_| {
                        Error::trap(anyhow::Error::msg("current time before unix epoch"))
                    })?;
                Ok(d.as_nanos().try_into()?)
            }
            types::Clockid::Monotonic => {
                let clock = self.clocks.monotonic()?;
                let now = clock.abs_clock.now(precision);
                let d = now.duration_since(clock.creation_time);
                Ok(d.as_nanos().try_into()?)
            }
            types::Clockid::ProcessCputimeId | types::Clockid::ThreadCputimeId => {
                Err(Error::badf().context("process and thread clocks are not supported"))
            }
        }
    }

    async fn fd_advise(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        offset: types::Filesize,
        len: types::Filesize,
        advice: types::Advice,
    ) -> Result<(), Error> {
        self.table()
            .get_file(u32::from(fd))?
            .file
            .advise(offset, len, advice.into())
            .await?;
        Ok(())
    }

    async fn fd_allocate(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        _offset: types::Filesize,
        _len: types::Filesize,
    ) -> Result<(), Error> {
        // Check if fd is a file, and has rights, just to reject those cases
        // with the errors expected:
        let _ = self.table().get_file(u32::from(fd))?;
        // This operation from cloudabi is linux-specific, isn't even
        // supported across all linux filesystems, and has no support on macos
        // or windows. Rather than ship spotty support, it has been removed
        // from preview 2, and we are no longer supporting it in preview 1 as
        // well.
        Err(Error::not_supported())
    }

    async fn fd_close(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: types::Fd,
    ) -> Result<(), Error> {
        let table = self.table();
        let fd = u32::from(fd);

        // Fail fast: If not present in table, Badf
        if !table.contains_key(fd) {
            return Err(Error::badf().context("key not in table"));
        }
        // fd_close must close either a File or a Dir handle
        if table.is::<FileEntry>(fd) {
            let _ = table.delete::<FileEntry>(fd);
        } else if table.is::<DirEntry>(fd) {
            let _ = table.delete::<DirEntry>(fd);
        } else {
            return Err(Error::badf().context("key does not refer to file or directory"));
        }

        Ok(())
    }

    async fn fd_datasync(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: types::Fd,
    ) -> Result<(), Error> {
        self.table()
            .get_file(u32::from(fd))?
            .file
            .datasync()
            .await?;
        Ok(())
    }

    async fn fd_fdstat_get(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: types::Fd,
    ) -> Result<types::Fdstat, Error> {
        let table = self.table();
        let fd = u32::from(fd);
        if table.is::<FileEntry>(fd) {
            let file_entry: Arc<FileEntry> = table.get(fd)?;
            let fdstat = file_entry.get_fdstat().await?;
            Ok(types::Fdstat::from(&fdstat))
        } else if table.is::<DirEntry>(fd) {
            let _dir_entry: Arc<DirEntry> = table.get(fd)?;
            let dir_fdstat = types::Fdstat {
                fs_filetype: types::Filetype::Directory,
                fs_rights_base: directory_base_rights(),
                fs_rights_inheriting: directory_inheriting_rights(),
                fs_flags: types::Fdflags::empty(),
            };
            Ok(dir_fdstat)
        } else {
            Err(Error::badf())
        }
    }

    async fn fd_fdstat_set_flags(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        flags: types::Fdflags,
    ) -> Result<(), Error> {
        if let Some(table) = self.table_mut() {
            table
                .get_file_mut(u32::from(fd))?
                .file
                .set_fdflags(FdFlags::from(flags))
                .await
        } else {
            log::warn!("`fd_fdstat_set_flags` does not work with wasi-threads enabled; see https://github.com/bytecodealliance/wasmtime/issues/5643");
            Err(Error::not_supported())
        }
    }

    async fn fd_fdstat_set_rights(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        _fs_rights_base: types::Rights,
        _fs_rights_inheriting: types::Rights,
    ) -> Result<(), Error> {
        let table = self.table();
        let fd = u32::from(fd);
        if table.is::<FileEntry>(fd) {
            let _file_entry: Arc<FileEntry> = table.get(fd)?;
            Ok(())
        } else if table.is::<DirEntry>(fd) {
            let _dir_entry: Arc<DirEntry> = table.get(fd)?;
            Ok(())
        } else {
            Err(Error::badf())
        }
    }

    async fn fd_filestat_get(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: types::Fd,
    ) -> Result<types::Filestat, Error> {
        let table = self.table();
        let fd = u32::from(fd);
        if table.is::<FileEntry>(fd) {
            let filestat = table.get_file(fd)?.file.get_filestat().await?;
            Ok(filestat.into())
        } else if table.is::<DirEntry>(fd) {
            let filestat = table.get_dir(fd)?.dir.get_filestat().await?;
            Ok(filestat.into())
        } else {
            Err(Error::badf())
        }
    }

    async fn fd_filestat_set_size(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        size: types::Filesize,
    ) -> Result<(), Error> {
        self.table()
            .get_file(u32::from(fd))?
            .file
            .set_filestat_size(size)
            .await?;
        Ok(())
    }

    async fn fd_filestat_set_times(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        atim: types::Timestamp,
        mtim: types::Timestamp,
        fst_flags: types::Fstflags,
    ) -> Result<(), Error> {
        let fd = u32::from(fd);
        let table = self.table();
        // Validate flags
        let set_atim = fst_flags.contains(types::Fstflags::ATIM);
        let set_atim_now = fst_flags.contains(types::Fstflags::ATIM_NOW);
        let set_mtim = fst_flags.contains(types::Fstflags::MTIM);
        let set_mtim_now = fst_flags.contains(types::Fstflags::MTIM_NOW);

        let atim = systimespec(set_atim, atim, set_atim_now).map_err(|e| e.context("atim"))?;
        let mtim = systimespec(set_mtim, mtim, set_mtim_now).map_err(|e| e.context("mtim"))?;

        if table.is::<FileEntry>(fd) {
            table
                .get_file(fd)
                .expect("checked that entry is file")
                .file
                .set_times(atim, mtim)
                .await
        } else if table.is::<DirEntry>(fd) {
            table
                .get_dir(fd)
                .expect("checked that entry is dir")
                .dir
                .set_times(".", atim, mtim, false)
                .await
        } else {
            Err(Error::badf())
        }
    }

    async fn fd_read(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        iovs: types::IovecArray,
    ) -> Result<types::Size, Error> {
        let f = self.table().get_file(u32::from(fd))?;
        // Access mode check normalizes error returned (windows would prefer ACCES here)
        if !f.access_mode.contains(FileAccessMode::READ) {
            Err(types::Errno::Badf)?
        }
        let f = &f.file;

        let iovs: Vec<wiggle::GuestPtr<[u8]>> = iovs
            .iter()
            .map(|iov_ptr| {
                let iov_ptr = iov_ptr?;
                let iov: types::Iovec = memory.read(iov_ptr)?;
                Ok(iov.buf.as_array(iov.buf_len))
            })
            .collect::<Result<_, Error>>()?;

        // If the first iov structure is from shared memory we can safely assume
        // all the rest will be. We then read into memory based on the memory's
        // shared-ness:
        // - if not shared, we copy directly into the Wasm memory
        // - if shared, we use an intermediate buffer; this avoids Rust unsafety
        //   due to holding on to a `&mut [u8]` of Wasm memory when we cannot
        //   guarantee the `&mut` exclusivity--other threads could be modifying
        //   the data as this functions writes to it. Though likely there is no
        //   issue with OS writing to io structs in multi-threaded scenarios,
        //   since we do not know here if `&dyn WasiFile` does anything else
        //   (e.g., read), we cautiously incur some performance overhead by
        //   copying twice.
        let is_shared_memory = memory.is_shared_memory();
        let bytes_read: u64 = if is_shared_memory {
            // For shared memory, read into an intermediate buffer. Only the
            // first iov will be filled and even then the read is capped by the
            // `MAX_SHARED_BUFFER_SIZE`, so users are expected to re-call.
            let iov = iovs.into_iter().next();
            if let Some(iov) = iov {
                let mut buffer = vec![0; (iov.len() as usize).min(MAX_SHARED_BUFFER_SIZE)];
                let bytes_read = f.read_vectored(&mut [IoSliceMut::new(&mut buffer)]).await?;
                let iov = iov
                    .get_range(0..bytes_read.try_into()?)
                    .expect("it should always be possible to slice the iov smaller");
                memory.copy_from_slice(&buffer[0..bytes_read.try_into()?], iov)?;
                bytes_read
            } else {
                return Ok(0);
            }
        } else {
            // Convert the first unsafe guest slice into a safe one--Wiggle
            // can only track mutable borrows for an entire region, and converting
            // all guest pointers to slices would cause a runtime borrow-checking
            // error. As read is allowed to return less than the requested amount,
            // it's valid (though not as efficient) for us to only perform the
            // read of the first buffer.
            let guest_slice: &mut [u8] = match iovs.into_iter().filter(|iov| iov.len() > 0).next() {
                Some(iov) => memory.as_slice_mut(iov)?.unwrap(),
                None => return Ok(0),
            };

            // Read directly into the Wasm memory.
            f.read_vectored(&mut [IoSliceMut::new(guest_slice)]).await?
        };

        Ok(types::Size::try_from(bytes_read)?)
    }

    async fn fd_pread(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        iovs: types::IovecArray,
        offset: types::Filesize,
    ) -> Result<types::Size, Error> {
        let f = self.table().get_file(u32::from(fd))?;
        // Access mode check normalizes error returned (windows would prefer ACCES here)
        if !f.access_mode.contains(FileAccessMode::READ) {
            Err(types::Errno::Badf)?
        }
        let f = &f.file;

        let iovs: Vec<wiggle::GuestPtr<[u8]>> = iovs
            .iter()
            .map(|iov_ptr| {
                let iov_ptr = iov_ptr?;
                let iov: types::Iovec = memory.read(iov_ptr)?;
                Ok(iov.buf.as_array(iov.buf_len))
            })
            .collect::<Result<_, Error>>()?;

        // If the first iov structure is from shared memory we can safely assume
        // all the rest will be. We then read into memory based on the memory's
        // shared-ness:
        // - if not shared, we copy directly into the Wasm memory
        // - if shared, we use an intermediate buffer; this avoids Rust unsafety
        //   due to holding on to a `&mut [u8]` of Wasm memory when we cannot
        //   guarantee the `&mut` exclusivity--other threads could be modifying
        //   the data as this functions writes to it. Though likely there is no
        //   issue with OS writing to io structs in multi-threaded scenarios,
        //   since we do not know here if `&dyn WasiFile` does anything else
        //   (e.g., read), we cautiously incur some performance overhead by
        //   copying twice.
        let is_shared_memory = memory.is_shared_memory();
        let bytes_read: u64 = if is_shared_memory {
            // For shared memory, read into an intermediate buffer. Only the
            // first iov will be filled and even then the read is capped by the
            // `MAX_SHARED_BUFFER_SIZE`, so users are expected to re-call.
            let iov = iovs.into_iter().next();
            if let Some(iov) = iov {
                let mut buffer = vec![0; (iov.len() as usize).min(MAX_SHARED_BUFFER_SIZE)];
                let bytes_read = f
                    .read_vectored_at(&mut [IoSliceMut::new(&mut buffer)], offset)
                    .await?;
                let iov = iov
                    .get_range(0..bytes_read.try_into()?)
                    .expect("it should always be possible to slice the iov smaller");
                memory.copy_from_slice(&buffer[0..bytes_read.try_into()?], iov)?;
                bytes_read
            } else {
                return Ok(0);
            }
        } else {
            // Convert unsafe guest slices to safe ones.
            let guest_slice: &mut [u8] = match iovs.into_iter().filter(|iov| iov.len() > 0).next() {
                Some(iov) => memory.as_slice_mut(iov)?.unwrap(),
                None => return Ok(0),
            };

            // Read directly into the Wasm memory.
            f.read_vectored_at(&mut [IoSliceMut::new(guest_slice)], offset)
                .await?
        };

        Ok(types::Size::try_from(bytes_read)?)
    }

    async fn fd_write(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        ciovs: types::CiovecArray,
    ) -> Result<types::Size, Error> {
        let f = self.table().get_file(u32::from(fd))?;
        // Access mode check normalizes error returned (windows would prefer ACCES here)
        if !f.access_mode.contains(FileAccessMode::WRITE) {
            Err(types::Errno::Badf)?
        }
        let f = &f.file;

        let guest_slices: Vec<Cow<[u8]>> = ciovs
            .iter()
            .map(|iov_ptr| {
                let iov_ptr = iov_ptr?;
                let iov: types::Ciovec = memory.read(iov_ptr)?;
                Ok(memory.as_cow(iov.buf.as_array(iov.buf_len))?)
            })
            .collect::<Result<_, Error>>()?;

        let ioslices: Vec<IoSlice> = guest_slices
            .iter()
            .map(|s| IoSlice::new(s.deref()))
            .collect();
        let bytes_written = f.write_vectored(&ioslices).await?;

        Ok(types::Size::try_from(bytes_written)?)
    }

    async fn fd_pwrite(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        ciovs: types::CiovecArray,
        offset: types::Filesize,
    ) -> Result<types::Size, Error> {
        let f = self.table().get_file(u32::from(fd))?;
        // Access mode check normalizes error returned (windows would prefer ACCES here)
        if !f.access_mode.contains(FileAccessMode::WRITE) {
            Err(types::Errno::Badf)?
        }
        let f = &f.file;

        let guest_slices: Vec<Cow<[u8]>> = ciovs
            .iter()
            .map(|iov_ptr| {
                let iov_ptr = iov_ptr?;
                let iov: types::Ciovec = memory.read(iov_ptr)?;
                Ok(memory.as_cow(iov.buf.as_array(iov.buf_len))?)
            })
            .collect::<Result<_, Error>>()?;

        let ioslices: Vec<IoSlice> = guest_slices
            .iter()
            .map(|s| IoSlice::new(s.deref()))
            .collect();
        let bytes_written = f.write_vectored_at(&ioslices, offset).await?;

        Ok(types::Size::try_from(bytes_written)?)
    }

    async fn fd_prestat_get(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: types::Fd,
    ) -> Result<types::Prestat, Error> {
        let table = self.table();
        let dir_entry: Arc<DirEntry> = table.get(u32::from(fd)).map_err(|_| Error::badf())?;
        if let Some(preopen) = dir_entry.preopen_path() {
            let path_str = preopen.to_str().ok_or_else(|| Error::not_supported())?;
            let pr_name_len = u32::try_from(path_str.as_bytes().len())?;
            Ok(types::Prestat::Dir(types::PrestatDir { pr_name_len }))
        } else {
            Err(Error::not_supported().context("file is not a preopen"))
        }
    }

    async fn fd_prestat_dir_name(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        path: GuestPtr<u8>,
        path_max_len: types::Size,
    ) -> Result<(), Error> {
        let table = self.table();
        let dir_entry: Arc<DirEntry> = table.get(u32::from(fd)).map_err(|_| Error::not_dir())?;
        if let Some(preopen) = dir_entry.preopen_path() {
            let path_bytes = preopen
                .to_str()
                .ok_or_else(|| Error::not_supported())?
                .as_bytes();
            let path_len = path_bytes.len();
            if path_len > path_max_len as usize {
                return Err(Error::name_too_long());
            }
            let path = path.as_array(path_len as u32);
            memory.copy_from_slice(path_bytes, path)?;
            Ok(())
        } else {
            Err(Error::not_supported())
        }
    }
    async fn fd_renumber(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        from: types::Fd,
        to: types::Fd,
    ) -> Result<(), Error> {
        let table = self.table();
        let from = u32::from(from);
        let to = u32::from(to);
        if !table.contains_key(from) {
            return Err(Error::badf());
        }
        table.renumber(from, to)
    }

    async fn fd_seek(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        offset: types::Filedelta,
        whence: types::Whence,
    ) -> Result<types::Filesize, Error> {
        use std::io::SeekFrom;
        let whence = match whence {
            types::Whence::Cur => SeekFrom::Current(offset),
            types::Whence::End => SeekFrom::End(offset),
            types::Whence::Set => {
                SeekFrom::Start(offset.try_into().map_err(|_| Error::invalid_argument())?)
            }
        };
        let newoffset = self
            .table()
            .get_file(u32::from(fd))?
            .file
            .seek(whence)
            .await?;
        Ok(newoffset)
    }

    async fn fd_sync(&mut self, _memory: &mut GuestMemory<'_>, fd: types::Fd) -> Result<(), Error> {
        self.table().get_file(u32::from(fd))?.file.sync().await?;
        Ok(())
    }

    async fn fd_tell(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: types::Fd,
    ) -> Result<types::Filesize, Error> {
        let offset = self
            .table()
            .get_file(u32::from(fd))?
            .file
            .seek(std::io::SeekFrom::Current(0))
            .await?;
        Ok(offset)
    }

    async fn fd_readdir(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        mut buf: GuestPtr<u8>,
        buf_len: types::Size,
        cookie: types::Dircookie,
    ) -> Result<types::Size, Error> {
        let mut bufused = 0;
        for entity in self
            .table()
            .get_dir(u32::from(fd))?
            .dir
            .readdir(ReaddirCursor::from(cookie))
            .await?
        {
            let entity = entity?;
            let dirent_raw = dirent_bytes(types::Dirent::try_from(&entity)?);
            let dirent_len: types::Size = dirent_raw.len().try_into()?;
            let name_raw = entity.name.as_bytes();
            let name_len: types::Size = name_raw.len().try_into()?;

            // Copy as many bytes of the dirent as we can, up to the end of the buffer
            let dirent_copy_len = std::cmp::min(dirent_len, buf_len - bufused);
            let raw = buf.as_array(dirent_copy_len);
            memory.copy_from_slice(&dirent_raw[..dirent_copy_len as usize], raw)?;

            // If the dirent struct wasn't compiled entirely, return that we filled the buffer, which
            // tells libc that we're not at EOF.
            if dirent_copy_len < dirent_len {
                return Ok(buf_len);
            }

            buf = buf.add(dirent_copy_len)?;
            bufused += dirent_copy_len;

            // Copy as many bytes of the name as we can, up to the end of the buffer
            let name_copy_len = std::cmp::min(name_len, buf_len - bufused);
            let raw = buf.as_array(name_copy_len);
            memory.copy_from_slice(&name_raw[..name_copy_len as usize], raw)?;

            // If the dirent struct wasn't copied entirely, return that we filled the buffer, which
            // tells libc that we're not at EOF

            if name_copy_len < name_len {
                return Ok(buf_len);
            }

            buf = buf.add(name_copy_len)?;
            bufused += name_copy_len;
        }
        Ok(bufused)
    }

    async fn path_create_directory(
        &mut self,
        memory: &mut GuestMemory<'_>,
        dirfd: types::Fd,
        path: GuestPtr<str>,
    ) -> Result<(), Error> {
        self.table()
            .get_dir(u32::from(dirfd))?
            .dir
            .create_dir(memory.as_cow_str(path)?.deref())
            .await
    }

    async fn path_filestat_get(
        &mut self,
        memory: &mut GuestMemory<'_>,
        dirfd: types::Fd,
        flags: types::Lookupflags,
        path: GuestPtr<str>,
    ) -> Result<types::Filestat, Error> {
        let filestat = self
            .table()
            .get_dir(u32::from(dirfd))?
            .dir
            .get_path_filestat(
                memory.as_cow_str(path)?.deref(),
                flags.contains(types::Lookupflags::SYMLINK_FOLLOW),
            )
            .await?;
        Ok(types::Filestat::from(filestat))
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
        let set_atim = fst_flags.contains(types::Fstflags::ATIM);
        let set_atim_now = fst_flags.contains(types::Fstflags::ATIM_NOW);
        let set_mtim = fst_flags.contains(types::Fstflags::MTIM);
        let set_mtim_now = fst_flags.contains(types::Fstflags::MTIM_NOW);

        let atim = systimespec(set_atim, atim, set_atim_now).map_err(|e| e.context("atim"))?;
        let mtim = systimespec(set_mtim, mtim, set_mtim_now).map_err(|e| e.context("mtim"))?;
        self.table()
            .get_dir(u32::from(dirfd))?
            .dir
            .set_times(
                memory.as_cow_str(path)?.deref(),
                atim,
                mtim,
                flags.contains(types::Lookupflags::SYMLINK_FOLLOW),
            )
            .await
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
        let table = self.table();
        let src_dir = table.get_dir(u32::from(src_fd))?;
        let target_dir = table.get_dir(u32::from(target_fd))?;
        let symlink_follow = src_flags.contains(types::Lookupflags::SYMLINK_FOLLOW);
        if symlink_follow {
            return Err(Error::invalid_argument()
                .context("symlink following on path_link is not supported"));
        }

        src_dir
            .dir
            .hard_link(
                memory.as_cow_str(src_path)?.deref(),
                target_dir.dir.deref(),
                memory.as_cow_str(target_path)?.deref(),
            )
            .await
    }

    async fn path_open(
        &mut self,
        memory: &mut GuestMemory<'_>,
        dirfd: types::Fd,
        dirflags: types::Lookupflags,
        path: GuestPtr<str>,
        oflags: types::Oflags,
        fs_rights_base: types::Rights,
        _fs_rights_inheriting: types::Rights,
        fdflags: types::Fdflags,
    ) -> Result<types::Fd, Error> {
        let table = self.table();
        let dirfd = u32::from(dirfd);
        if table.is::<FileEntry>(dirfd) {
            return Err(Error::not_dir());
        }
        let dir_entry = table.get_dir(dirfd)?;

        let symlink_follow = dirflags.contains(types::Lookupflags::SYMLINK_FOLLOW);

        let oflags = OFlags::from(&oflags);
        let fdflags = FdFlags::from(fdflags);
        let path = memory.as_cow_str(path)?;

        let read = fs_rights_base.contains(types::Rights::FD_READ);
        let write = fs_rights_base.contains(types::Rights::FD_WRITE);
        let access_mode = if read {
            FileAccessMode::READ
        } else {
            FileAccessMode::empty()
        } | if write {
            FileAccessMode::WRITE
        } else {
            FileAccessMode::empty()
        };

        let file = dir_entry
            .dir
            .open_file(symlink_follow, path.deref(), oflags, read, write, fdflags)
            .await?;
        drop(dir_entry);

        let fd = match file {
            OpenResult::File(file) => table.push(Arc::new(FileEntry::new(file, access_mode)))?,
            OpenResult::Dir(child_dir) => table.push(Arc::new(DirEntry::new(None, child_dir)))?,
        };
        Ok(types::Fd::from(fd))
    }

    async fn path_readlink(
        &mut self,
        memory: &mut GuestMemory<'_>,
        dirfd: types::Fd,
        path: GuestPtr<str>,
        buf: GuestPtr<u8>,
        buf_len: types::Size,
    ) -> Result<types::Size, Error> {
        let link = self
            .table()
            .get_dir(u32::from(dirfd))?
            .dir
            .read_link(memory.as_cow_str(path)?.deref())
            .await?
            .into_os_string()
            .into_string()
            .map_err(|_| Error::illegal_byte_sequence().context("link contents"))?;
        let link_bytes = link.as_bytes();
        // Like posix readlink(2), silently truncate links when they are larger than the
        // destination buffer:
        let link_len = std::cmp::min(link_bytes.len(), buf_len as usize);
        let buf = buf.as_array(link_len as u32);
        memory.copy_from_slice(&link_bytes[..link_len], buf)?;
        Ok(link_len as types::Size)
    }

    async fn path_remove_directory(
        &mut self,
        memory: &mut GuestMemory<'_>,
        dirfd: types::Fd,
        path: GuestPtr<str>,
    ) -> Result<(), Error> {
        self.table()
            .get_dir(u32::from(dirfd))?
            .dir
            .remove_dir(memory.as_cow_str(path)?.deref())
            .await
    }

    async fn path_rename(
        &mut self,
        memory: &mut GuestMemory<'_>,
        src_fd: types::Fd,
        src_path: GuestPtr<str>,
        dest_fd: types::Fd,
        dest_path: GuestPtr<str>,
    ) -> Result<(), Error> {
        let table = self.table();
        let src_dir = table.get_dir(u32::from(src_fd))?;
        let dest_dir = table.get_dir(u32::from(dest_fd))?;
        src_dir
            .dir
            .rename(
                memory.as_cow_str(src_path)?.deref(),
                dest_dir.dir.deref(),
                memory.as_cow_str(dest_path)?.deref(),
            )
            .await
    }

    async fn path_symlink(
        &mut self,
        memory: &mut GuestMemory<'_>,
        src_path: GuestPtr<str>,
        dirfd: types::Fd,
        dest_path: GuestPtr<str>,
    ) -> Result<(), Error> {
        self.table()
            .get_dir(u32::from(dirfd))?
            .dir
            .symlink(
                memory.as_cow_str(src_path)?.deref(),
                memory.as_cow_str(dest_path)?.deref(),
            )
            .await
    }

    async fn path_unlink_file(
        &mut self,
        memory: &mut GuestMemory<'_>,
        dirfd: types::Fd,
        path: GuestPtr<str>,
    ) -> Result<(), Error> {
        self.table()
            .get_dir(u32::from(dirfd))?
            .dir
            .unlink_file(memory.as_cow_str(path)?.deref())
            .await
    }

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
        // We need these refmuts to outlive Poll, which will hold the &mut dyn WasiFile inside
        let mut read_refs: Vec<(Arc<FileEntry>, Option<Userdata>)> = Vec::new();
        let mut write_refs: Vec<(Arc<FileEntry>, Option<Userdata>)> = Vec::new();

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
                    types::Clockid::Realtime => {
                        // POSIX specifies that functions like `nanosleep` and others use the
                        // `REALTIME` clock. But it also says that `clock_settime` has no effect
                        // on threads waiting in these functions. MONOTONIC should always have
                        // resolution at least as good as REALTIME, so we can translate a
                        // non-absolute `REALTIME` request into a `MONOTONIC` request.
                        let clock = self.clocks.monotonic()?;
                        let precision = Duration::from_nanos(clocksub.precision);
                        let duration = Duration::from_nanos(clocksub.timeout);
                        let deadline = if clocksub
                            .flags
                            .contains(types::Subclockflags::SUBSCRIPTION_CLOCK_ABSTIME)
                        {
                            return Err(Error::not_supported());
                        } else {
                            clock
                                .abs_clock
                                .now(precision)
                                .checked_add(duration)
                                .ok_or_else(|| Error::overflow().context("deadline"))?
                        };
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
                    let file_ref = table.get_file(u32::from(fd))?;
                    read_refs.push((file_ref, Some(sub.userdata.into())));
                }
                types::SubscriptionU::FdWrite(writesub) => {
                    let fd = writesub.file_descriptor;
                    let file_ref = table.get_file(u32::from(fd))?;
                    write_refs.push((file_ref, Some(sub.userdata.into())));
                }
            }
        }

        let mut read_mut_refs: Vec<(&dyn WasiFile, Userdata)> = Vec::new();
        for (file_lock, userdata) in read_refs.iter_mut() {
            read_mut_refs.push((file_lock.file.deref(), userdata.take().unwrap()));
        }

        for (f, ud) in read_mut_refs.iter_mut() {
            poll.subscribe_read(*f, *ud);
        }

        let mut write_mut_refs: Vec<(&dyn WasiFile, Userdata)> = Vec::new();
        for (file_lock, userdata) in write_refs.iter_mut() {
            write_mut_refs.push((file_lock.file.deref(), userdata.take().unwrap()));
        }

        for (f, ud) in write_mut_refs.iter_mut() {
            poll.subscribe_write(*f, *ud);
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
                                error: e.downcast().map_err(Error::trap)?,
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
                                error: e.downcast().map_err(Error::trap)?,
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
                                Err(e) => e.downcast().map_err(Error::trap)?,
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
        _memory: &mut GuestMemory<'_>,
        status: types::Exitcode,
    ) -> anyhow::Error {
        // Check that the status is within WASI's range.
        if status < 126 {
            I32Exit(status as i32).into()
        } else {
            anyhow::Error::msg("exit with invalid exit status outside of [0..126)")
        }
    }

    async fn proc_raise(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        _sig: types::Signal,
    ) -> Result<(), Error> {
        Err(Error::trap(anyhow::Error::msg("proc_raise unsupported")))
    }

    async fn sched_yield(&mut self, _memory: &mut GuestMemory<'_>) -> Result<(), Error> {
        self.sched.sched_yield().await
    }

    async fn random_get(
        &mut self,
        memory: &mut GuestMemory<'_>,
        buf: GuestPtr<u8>,
        buf_len: types::Size,
    ) -> Result<(), Error> {
        let buf = buf.as_array(buf_len);
        if memory.is_shared_memory() {
            // If the Wasm memory is shared, copy to an intermediate buffer to
            // avoid Rust unsafety (i.e., the called function could rely on
            // `&mut [u8]`'s exclusive ownership which is not guaranteed due to
            // potential access from other threads).
            let mut copied: u32 = 0;
            while copied < buf.len() {
                let len = (buf.len() - copied).min(MAX_SHARED_BUFFER_SIZE as u32);
                let mut tmp = vec![0; len as usize];
                self.random.lock().unwrap().try_fill_bytes(&mut tmp)?;
                let dest = buf.get_range(copied..copied + len).unwrap();
                memory.copy_from_slice(&tmp, dest)?;
                copied += len;
            }
        } else {
            // If the Wasm memory is non-shared, copy directly into the linear
            // memory.
            let mem = &mut memory.as_slice_mut(buf)?.unwrap();
            self.random.lock().unwrap().try_fill_bytes(mem)?;
        }
        Ok(())
    }

    async fn sock_accept(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        flags: types::Fdflags,
    ) -> Result<types::Fd, Error> {
        let table = self.table();
        let f = table.get_file(u32::from(fd))?;
        let file = f.file.sock_accept(FdFlags::from(flags)).await?;
        let fd = table.push(Arc::new(FileEntry::new(file, FileAccessMode::all())))?;
        Ok(types::Fd::from(fd))
    }

    async fn sock_recv(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        ri_data: types::IovecArray,
        ri_flags: types::Riflags,
    ) -> Result<(types::Size, types::Roflags), Error> {
        let f = self.table().get_file(u32::from(fd))?;

        let iovs: Vec<wiggle::GuestPtr<[u8]>> = ri_data
            .iter()
            .map(|iov_ptr| {
                let iov_ptr = iov_ptr?;
                let iov: types::Iovec = memory.read(iov_ptr)?;
                Ok(iov.buf.as_array(iov.buf_len))
            })
            .collect::<Result<_, Error>>()?;

        // If the first iov structure is from shared memory we can safely assume
        // all the rest will be. We then read into memory based on the memory's
        // shared-ness:
        // - if not shared, we copy directly into the Wasm memory
        // - if shared, we use an intermediate buffer; this avoids Rust unsafety
        //   due to holding on to a `&mut [u8]` of Wasm memory when we cannot
        //   guarantee the `&mut` exclusivity--other threads could be modifying
        //   the data as this functions writes to it. Though likely there is no
        //   issue with OS writing to io structs in multi-threaded scenarios,
        //   since we do not know here if `&dyn WasiFile` does anything else
        //   (e.g., read), we cautiously incur some performance overhead by
        //   copying twice.
        let is_shared_memory = memory.is_shared_memory();
        let (bytes_read, ro_flags) = if is_shared_memory {
            // For shared memory, read into an intermediate buffer. Only the
            // first iov will be filled and even then the read is capped by the
            // `MAX_SHARED_BUFFER_SIZE`, so users are expected to re-call.
            let iov = iovs.into_iter().next();
            if let Some(iov) = iov {
                let mut buffer = vec![0; (iov.len() as usize).min(MAX_SHARED_BUFFER_SIZE)];
                let (bytes_read, ro_flags) = f
                    .file
                    .sock_recv(&mut [IoSliceMut::new(&mut buffer)], RiFlags::from(ri_flags))
                    .await?;
                let iov = iov
                    .get_range(0..bytes_read.try_into()?)
                    .expect("it should always be possible to slice the iov smaller");
                memory.copy_from_slice(&buffer[0..bytes_read.try_into()?], iov)?;
                (bytes_read, ro_flags)
            } else {
                return Ok((0, RoFlags::empty().into()));
            }
        } else {
            // Convert all of the unsafe guest slices to safe ones--this uses
            // Wiggle's internal borrow checker to ensure no overlaps. We assume
            // here that, because the memory is not shared, there are no other
            // threads to access it while it is written to.
            let guest_slice: &mut [u8] = match iovs.into_iter().filter(|iov| iov.len() > 0).next() {
                Some(iov) => memory.as_slice_mut(iov)?.unwrap(),
                None => &mut [],
            };

            // Read directly into the Wasm memory.
            f.file
                .sock_recv(&mut [IoSliceMut::new(guest_slice)], RiFlags::from(ri_flags))
                .await?
        };

        Ok((types::Size::try_from(bytes_read)?, ro_flags.into()))
    }

    async fn sock_send(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        si_data: types::CiovecArray,
        _si_flags: types::Siflags,
    ) -> Result<types::Size, Error> {
        let f = self.table().get_file(u32::from(fd))?;

        let guest_slices: Vec<Cow<[u8]>> = si_data
            .iter()
            .map(|iov_ptr| {
                let iov_ptr = iov_ptr?;
                let iov: types::Ciovec = memory.read(iov_ptr)?;
                Ok(memory.as_cow(iov.buf.as_array(iov.buf_len))?)
            })
            .collect::<Result<_, Error>>()?;

        let ioslices: Vec<IoSlice> = guest_slices
            .iter()
            .map(|s| IoSlice::new(s.deref()))
            .collect();
        let bytes_written = f.file.sock_send(&ioslices, SiFlags::empty()).await?;

        Ok(types::Size::try_from(bytes_written)?)
    }

    async fn sock_shutdown(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: types::Fd,
        how: types::Sdflags,
    ) -> Result<(), Error> {
        let f = self.table().get_file(u32::from(fd))?;

        f.file.sock_shutdown(SdFlags::from(how)).await
    }
}

impl From<types::Advice> for Advice {
    fn from(advice: types::Advice) -> Advice {
        match advice {
            types::Advice::Normal => Advice::Normal,
            types::Advice::Sequential => Advice::Sequential,
            types::Advice::Random => Advice::Random,
            types::Advice::Willneed => Advice::WillNeed,
            types::Advice::Dontneed => Advice::DontNeed,
            types::Advice::Noreuse => Advice::NoReuse,
        }
    }
}

impl From<&FdStat> for types::Fdstat {
    fn from(fdstat: &FdStat) -> types::Fdstat {
        let mut fs_rights_base = types::Rights::empty();
        if fdstat.access_mode.contains(FileAccessMode::READ) {
            fs_rights_base |= types::Rights::FD_READ;
        }
        if fdstat.access_mode.contains(FileAccessMode::WRITE) {
            fs_rights_base |= types::Rights::FD_WRITE;
        }
        types::Fdstat {
            fs_filetype: types::Filetype::from(&fdstat.filetype),
            fs_rights_base,
            fs_rights_inheriting: types::Rights::empty(),
            fs_flags: types::Fdflags::from(fdstat.flags),
        }
    }
}

impl From<&FileType> for types::Filetype {
    fn from(ft: &FileType) -> types::Filetype {
        match ft {
            FileType::Directory => types::Filetype::Directory,
            FileType::BlockDevice => types::Filetype::BlockDevice,
            FileType::CharacterDevice => types::Filetype::CharacterDevice,
            FileType::RegularFile => types::Filetype::RegularFile,
            FileType::SocketDgram => types::Filetype::SocketDgram,
            FileType::SocketStream => types::Filetype::SocketStream,
            FileType::SymbolicLink => types::Filetype::SymbolicLink,
            FileType::Unknown => types::Filetype::Unknown,
            FileType::Pipe => types::Filetype::Unknown,
        }
    }
}

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

macro_rules! convert_flags_bidirectional {
    ($from:ty, $to:ty, $($rest:tt)*) => {
        convert_flags!($from, $to, $($rest)*);
        convert_flags!($to, $from, $($rest)*);
    }
}

convert_flags_bidirectional!(
    FdFlags,
    types::Fdflags,
    APPEND,
    DSYNC,
    NONBLOCK,
    RSYNC,
    SYNC
);

convert_flags_bidirectional!(RiFlags, types::Riflags, RECV_PEEK, RECV_WAITALL);

convert_flags_bidirectional!(RoFlags, types::Roflags, RECV_DATA_TRUNCATED);

convert_flags_bidirectional!(SdFlags, types::Sdflags, RD, WR);

impl From<&types::Oflags> for OFlags {
    fn from(oflags: &types::Oflags) -> OFlags {
        let mut out = OFlags::empty();
        if oflags.contains(types::Oflags::CREAT) {
            out = out | OFlags::CREATE;
        }
        if oflags.contains(types::Oflags::DIRECTORY) {
            out = out | OFlags::DIRECTORY;
        }
        if oflags.contains(types::Oflags::EXCL) {
            out = out | OFlags::EXCLUSIVE;
        }
        if oflags.contains(types::Oflags::TRUNC) {
            out = out | OFlags::TRUNCATE;
        }
        out
    }
}

impl From<&OFlags> for types::Oflags {
    fn from(oflags: &OFlags) -> types::Oflags {
        let mut out = types::Oflags::empty();
        if oflags.contains(OFlags::CREATE) {
            out = out | types::Oflags::CREAT;
        }
        if oflags.contains(OFlags::DIRECTORY) {
            out = out | types::Oflags::DIRECTORY;
        }
        if oflags.contains(OFlags::EXCLUSIVE) {
            out = out | types::Oflags::EXCL;
        }
        if oflags.contains(OFlags::TRUNCATE) {
            out = out | types::Oflags::TRUNC;
        }
        out
    }
}
impl From<Filestat> for types::Filestat {
    fn from(stat: Filestat) -> types::Filestat {
        types::Filestat {
            dev: stat.device_id,
            ino: stat.inode,
            filetype: types::Filetype::from(&stat.filetype),
            nlink: stat.nlink,
            size: stat.size,
            atim: stat
                .atim
                .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64)
                .unwrap_or(0),
            mtim: stat
                .mtim
                .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64)
                .unwrap_or(0),
            ctim: stat
                .ctim
                .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64)
                .unwrap_or(0),
        }
    }
}

impl TryFrom<&ReaddirEntity> for types::Dirent {
    type Error = Error;
    fn try_from(e: &ReaddirEntity) -> Result<types::Dirent, Error> {
        Ok(types::Dirent {
            d_ino: e.inode,
            d_namlen: e.name.as_bytes().len().try_into()?,
            d_type: types::Filetype::from(&e.filetype),
            d_next: e.next.into(),
        })
    }
}

fn dirent_bytes(dirent: types::Dirent) -> Vec<u8> {
    use wiggle::GuestType;
    assert_eq!(
        types::Dirent::guest_size(),
        std::mem::size_of::<types::Dirent>() as u32,
        "Dirent guest repr and host repr should match"
    );
    assert_eq!(
        1,
        std::mem::size_of_val(&dirent.d_type),
        "Dirent member d_type should be endian-invariant"
    );
    let size = types::Dirent::guest_size()
        .try_into()
        .expect("Dirent is smaller than 2^32");
    let mut bytes = Vec::with_capacity(size);
    bytes.resize(size, 0);
    let ptr = bytes.as_mut_ptr().cast::<types::Dirent>();
    let guest_dirent = types::Dirent {
        d_ino: dirent.d_ino.to_le(),
        d_namlen: dirent.d_namlen.to_le(),
        d_type: dirent.d_type, // endian-invariant
        d_next: dirent.d_next.to_le(),
    };
    unsafe { ptr.write_unaligned(guest_dirent) };
    bytes
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

fn systimespec(
    set: bool,
    ts: types::Timestamp,
    now: bool,
) -> Result<Option<SystemTimeSpec>, Error> {
    if set && now {
        Err(Error::invalid_argument())
    } else if set {
        Ok(Some(SystemTimeSpec::Absolute(
            SystemClock::UNIX_EPOCH + Duration::from_nanos(ts),
        )))
    } else if now {
        Ok(Some(SystemTimeSpec::SymbolicNow))
    } else {
        Ok(None)
    }
}

// This is the default subset of base Rights reported for directories prior to
// https://github.com/bytecodealliance/wasmtime/pull/6265. Some
// implementations still expect this set of rights to be reported.
pub(crate) fn directory_base_rights() -> types::Rights {
    types::Rights::PATH_CREATE_DIRECTORY
        | types::Rights::PATH_CREATE_FILE
        | types::Rights::PATH_LINK_SOURCE
        | types::Rights::PATH_LINK_TARGET
        | types::Rights::PATH_OPEN
        | types::Rights::FD_READDIR
        | types::Rights::PATH_READLINK
        | types::Rights::PATH_RENAME_SOURCE
        | types::Rights::PATH_RENAME_TARGET
        | types::Rights::PATH_SYMLINK
        | types::Rights::PATH_REMOVE_DIRECTORY
        | types::Rights::PATH_UNLINK_FILE
        | types::Rights::PATH_FILESTAT_GET
        | types::Rights::PATH_FILESTAT_SET_TIMES
        | types::Rights::FD_FILESTAT_GET
        | types::Rights::FD_FILESTAT_SET_TIMES
}

// This is the default subset of inheriting Rights reported for directories
// prior to https://github.com/bytecodealliance/wasmtime/pull/6265. Some
// implementations still expect this set of rights to be reported.
pub(crate) fn directory_inheriting_rights() -> types::Rights {
    types::Rights::FD_DATASYNC
        | types::Rights::FD_READ
        | types::Rights::FD_SEEK
        | types::Rights::FD_FDSTAT_SET_FLAGS
        | types::Rights::FD_SYNC
        | types::Rights::FD_TELL
        | types::Rights::FD_WRITE
        | types::Rights::FD_ADVISE
        | types::Rights::FD_ALLOCATE
        | types::Rights::FD_FILESTAT_GET
        | types::Rights::FD_FILESTAT_SET_SIZE
        | types::Rights::FD_FILESTAT_SET_TIMES
        | types::Rights::POLL_FD_READWRITE
        | directory_base_rights()
}
