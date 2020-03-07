use crate::fs::Metadata;
use crate::wasi::{self, WasiResult};
use crate::{host, hostcalls, hostcalls_impl, WasiCtx};
use std::io;

/// A reference to an open file on the filesystem.
///
/// This corresponds to [`std::fs::File`].
///
/// Note that this `File` has no `open` or `create` methods. To open or create
/// a file, you must first obtain a [`Dir`] containing the file, and then call
/// [`Dir::open_file`] or [`Dir::create_file`].
///
/// [`std::fs::File`]: https://doc.rust-lang.org/std/fs/struct.File.html
/// [`Dir`]: struct.Dir.html
/// [`Dir::open_file`]: struct.Dir.html#method.open_file
/// [`Dir::create_file`]: struct.Dir.html#method.create_file
pub struct File<'ctx> {
    ctx: &'ctx mut WasiCtx,
    fd: wasi::__wasi_fd_t,
}

impl<'ctx> File<'ctx> {
    /// Constructs a new instance of `Self` from the given raw WASI file descriptor.
    ///
    /// This corresponds to [`std::fs::File::from_raw_fd`].
    ///
    /// [`std::fs::File::from_raw_fd`]: https://doc.rust-lang.org/std/fs/struct.File.html#method.from_raw_fd
    pub unsafe fn from_raw_wasi_fd(ctx: &'ctx mut WasiCtx, fd: wasi::__wasi_fd_t) -> Self {
        Self { ctx, fd }
    }

    /// Attempts to sync all OS-internal metadata to disk.
    ///
    /// This corresponds to [`std::fs::File::sync_all`].
    ///
    /// [`std::fs::File::sync_all`]: https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all
    pub fn sync_all(&self) -> WasiResult<()> {
        unsafe {
            hostcalls_impl::fd_sync(self.ctx, &mut [], self.fd)?;
        }
        Ok(())
    }

    /// This function is similar to `sync_all`, except that it may not synchronize
    /// file metadata to the filesystem.
    ///
    /// This corresponds to [`std::fs::File::sync_data`].
    ///
    /// [`std::fs::File::sync_data`]: https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_data
    pub fn sync_data(&self) -> WasiResult<()> {
        unsafe {
            hostcalls_impl::fd_datasync(self.ctx, &mut [], self.fd)?;
        }
        Ok(())
    }

    /// Truncates or extends the underlying file, updating the size of this file
    /// to become size.
    ///
    /// This corresponds to [`std::fs::File::set_len`].
    ///
    /// [`std::fs::File::set_len`]: https://doc.rust-lang.org/std/fs/struct.File.html#method.set_len
    pub fn set_len(&self, size: u64) -> WasiResult<()> {
        unsafe {
            hostcalls_impl::fd_filestat_set_size(self.ctx, &mut [], self.fd, size)?;
        }
        Ok(())
    }

    /// Queries metadata about the underlying file.
    ///
    /// This corresponds to [`std::fs::File::metadata`].
    ///
    /// [`std::fs::File::metadata`]: https://doc.rust-lang.org/std/fs/struct.File.html#method.metadata
    pub fn metadata(&self) -> WasiResult<Metadata> {
        Ok(Metadata {})
    }
}

impl<'ctx> Drop for File<'ctx> {
    fn drop(&mut self) {
        // Note that errors are ignored when closing a file descriptor. The
        // reason for this is that if an error occurs we don't actually know if
        // the file descriptor was closed or not, and if we retried (for
        // something like EINTR), we might close another valid file descriptor
        // opened after we closed ours.
        let _ = unsafe { hostcalls::fd_close(self.ctx, &mut [], self.fd) };
    }
}

impl<'ctx> io::Read for File<'ctx> {
    /// TODO: Not yet implemented. See the comment in `Dir::open_file`.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let iov = [host::__wasi_iovec_t {
            buf: buf.as_mut_ptr() as *mut u8,
            buf_len: buf.len(),
        }];
        let mut nread = 0;

        // TODO: See the comment in `Dir::open_file`.
        unimplemented!("File::read");
        /*
        wasi_errno_to_io_error(unsafe {
            hostcalls::fd_read(self.ctx, self.fd, &iov, 1, &mut nread)
        })?;
        */

        Ok(nread)
    }
}

// TODO: traits to implement: Write, Seek

// TODO: functions from FileExt?

// TODO: impl Debug for File
