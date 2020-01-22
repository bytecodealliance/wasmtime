use crate::fs::{File, OpenOptions, ReadDir};
use crate::{host, hostcalls, wasi, WasiCtx};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::{io, path::Path};

/// A reference to an open directory on the filesystem.
///
/// TODO: Implement `Dir`-using versions of `std::fs`'s free functions:
/// `copy`, `create_dir`, `create_dir_all`, `hard_link`, `metadata`,
/// `read_link`, `read_to_string`, `remove_dir`, `remove_dir_all`,
/// `remove_file`, `rename`, `set_permissions`, `symlink_metadata`, and
/// `write`.
///
/// Unlike `std::fs`, this API has no `canonicalize`, because absolute paths
/// don't interoperate well with the capability-oriented security model.
pub struct Dir<'ctx> {
    ctx: &'ctx mut WasiCtx,
    fd: wasi::__wasi_fd_t,
}

impl<'ctx> Dir<'ctx> {
    /// Constructs a new instance of `Self` from the given raw WASI file descriptor.
    pub unsafe fn from_raw_wasi_fd(ctx: &'ctx mut WasiCtx, fd: wasi::__wasi_fd_t) -> Self {
        Self { ctx, fd }
    }

    /// Attempts to open a file in read-only mode.
    ///
    /// This corresponds to [`std::fs::File::open`], but only accesses paths
    /// relative to and within `self`.
    ///
    /// TODO: Not yet implemented. Refactor the hostcalls functions to split out the
    /// encoding/decoding parts from the underlying functionality, so that we can call
    /// into the underlying functionality directly.
    ///
    /// [`std::fs::File::open`]: https://doc.rust-lang.org/std/fs/struct.File.html#method.open
    pub fn open_file<P: AsRef<Path>>(&mut self, path: P) -> io::Result<File> {
        let path = path.as_ref();
        let mut fd = 0;

        // TODO: Refactor the hostcalls functions to split out the encoding/decoding
        // parts from the underlying functionality, so that we can call into the
        // underlying functionality directly.
        //
        // TODO: Set the requested rights to be readonly.
        //
        // TODO: Handle paths for non-Unix platforms which don't have `as_bytes()`
        // on `OsStrExt`.
        unimplemented!("Dir::open_file");
        /*
        wasi_errno_to_io_error(hostcalls::path_open(
            self.ctx,
            self.fd,
            wasi::__WASI_LOOKUPFLAGS_SYMLINK_FOLLOW,
            path.as_os_str().as_bytes(),
            path.as_os_str().len(),
            0,
            !0,
            !0,
            0,
            &mut fd,
        ))?;
        */

        let ctx = self.ctx;
        Ok(unsafe { File::from_raw_wasi_fd(ctx, fd) })
    }

    /// Opens a file at `path` with the options specified by `self`.
    ///
    /// This corresponds to [`std::fs::OpenOptions::open`].
    ///
    /// Instead of being a method on `OpenOptions`, this is a method on `Dir`,
    /// and it only accesses functions relative to and within `self`.
    ///
    /// TODO: Not yet implemented.
    ///
    /// [`std::fs::OpenOptions::open`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.open
    pub fn open_file_with<P: AsRef<Path>>(
        &mut self,
        path: P,
        options: &OpenOptions,
    ) -> io::Result<File> {
        unimplemented!("Dir::open_file_with");
    }

    /// Attempts to open a directory.
    ///
    /// TODO: Not yet implemented. See the comment in `open_file`.
    pub fn open_dir<P: AsRef<Path>>(&mut self, path: P) -> io::Result<Self> {
        let path = path.as_ref();
        let mut fd = 0;

        // TODO: See the comment in `open_file`.
        unimplemented!("Dir::open_dir");
        /*
        wasi_errno_to_io_error(hostcalls::path_open(
            self.ctx,
            self.fd,
            wasi::__WASI_LOOKUPFLAGS_SYMLINK_FOLLOW,
            path.as_os_str().as_bytes(),
            wasi::__WASI_OFLAGS_DIRECTORY,
            !0,
            !0,
            0,
            &mut fd,
        ))?;
        */

        let ctx = self.ctx;
        Ok(unsafe { Dir::from_raw_wasi_fd(ctx, fd) })
    }

    /// Opens a file in write-only mode.
    ///
    /// This corresponds to [`std::fs::File::create`], but only accesses paths
    /// relative to and within `self`.
    ///
    /// TODO: Not yet implemented. See the comment in `open_file`.
    ///
    /// [`std::fs::File::create`]: https://doc.rust-lang.org/std/fs/struct.File.html#method.create
    pub fn create_file<P: AsRef<Path>>(&mut self, path: P) -> io::Result<File> {
        let path = path.as_ref();
        let mut fd = 0;

        // TODO: See the comments in `open_file`.
        //
        // TODO: Set the requested rights to be read+write.
        unimplemented!("Dir::create_file");
        /*
        wasi_errno_to_io_error(hostcalls::path_open(
            self.ctx,
            self.fd,
            wasi::__WASI_LOOKUPFLAGS_SYMLINK_FOLLOW,
            path.as_os_str().as_bytes(),
            path.as_os_str().len(),
            wasi::__WASI_OFLAGS_CREAT | wasi::__WASI_OFLAGS_TRUNC,
            !0,
            !0,
            0,
            &mut fd,
        ))?;
        */

        let ctx = self.ctx;
        Ok(unsafe { File::from_raw_wasi_fd(ctx, fd) })
    }

    /// Returns an iterator over the entries within a directory.
    ///
    /// This corresponds to [`std::fs::read_dir`], but reads the directory
    /// represented by `self`.
    ///
    /// TODO: Not yet implemented. We may need to wait until we have the ability
    /// to duplicate file descriptors before we can implement read safely. For
    /// now, use `into_read` instead.
    ///
    /// [`std::fs::read_dir`]: https://doc.rust-lang.org/std/fs/fn.read_dir.html
    pub fn read(&mut self) -> io::Result<ReadDir> {
        unimplemented!("Dir::read")
    }

    /// Consumes self and returns an iterator over the entries within a directory
    /// in the manner of `read`.
    pub fn into_read(self) -> ReadDir {
        unsafe { ReadDir::from_raw_wasi_fd(self.fd) }
    }

    /// Read the entire contents of a file into a bytes vector.
    ///
    /// This corresponds to [`std::fs::read`], but only accesses paths
    /// relative to and within `self`.
    ///
    /// [`std::fs::read`]: https://doc.rust-lang.org/std/fs/fn.read.html
    pub fn read_file<P: AsRef<Path>>(&mut self, path: P) -> io::Result<Vec<u8>> {
        use io::Read;
        let mut file = self.open_file(path)?;
        let mut bytes = Vec::with_capacity(initial_buffer_size(&file));
        file.read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    /// Returns an iterator over the entries within a directory.
    ///
    /// This corresponds to [`std::fs::read_dir`], but only accesses paths
    /// relative to and within `self`.
    ///
    /// [`std::fs::read_dir`]: https://doc.rust-lang.org/std/fs/fn.read_dir.html
    pub fn read_dir<P: AsRef<Path>>(&mut self, path: P) -> io::Result<ReadDir> {
        self.open_dir(path)?.read()
    }
}

impl<'ctx> Drop for Dir<'ctx> {
    fn drop(&mut self) {
        // Note that errors are ignored when closing a file descriptor. The
        // reason for this is that if an error occurs we don't actually know if
        // the file descriptor was closed or not, and if we retried (for
        // something like EINTR), we might close another valid file descriptor
        // opened after we closed ours.
        let _ = unsafe { hostcalls::fd_close(self.ctx, &mut [], self.fd) };
    }
}

/// Indicates how large a buffer to pre-allocate before reading the entire file.
///
/// Derived from the function of the same name in libstd.
fn initial_buffer_size(file: &File) -> usize {
    // Allocate one extra byte so the buffer doesn't need to grow before the
    // final `read` call at the end of the file.  Don't worry about `usize`
    // overflow because reading will fail regardless in that case.
    file.metadata().map(|m| m.len() as usize + 1).unwrap_or(0)
}

// TODO: impl Debug for Dir
