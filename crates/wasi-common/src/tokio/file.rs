use crate::tokio::block_on_dummy_executor;
use crate::{
    Error,
    file::{Advice, FdFlags, FileType, Filestat, WasiFile},
};
#[cfg(windows)]
use io_extras::os::windows::{AsRawHandleOrSocket, RawHandleOrSocket};
#[cfg(not(windows))]
use io_lifetimes::AsFd;
use std::any::Any;
use std::borrow::Borrow;
use std::io;

pub struct File(crate::sync::file::File);

impl File {
    pub(crate) fn from_inner(file: crate::sync::file::File) -> Self {
        File(file)
    }
    pub fn from_cap_std(file: cap_std::fs::File) -> Self {
        Self::from_inner(crate::sync::file::File::from_cap_std(file))
    }
}

pub struct TcpListener(crate::sync::net::TcpListener);

impl TcpListener {
    pub(crate) fn from_inner(listener: crate::sync::net::TcpListener) -> Self {
        TcpListener(listener)
    }
    pub fn from_cap_std(listener: cap_std::net::TcpListener) -> Self {
        Self::from_inner(crate::sync::net::TcpListener::from_cap_std(listener))
    }
}

pub struct TcpStream(crate::sync::net::TcpStream);

impl TcpStream {
    pub(crate) fn from_inner(stream: crate::sync::net::TcpStream) -> Self {
        TcpStream(stream)
    }
    pub fn from_cap_std(stream: cap_std::net::TcpStream) -> Self {
        Self::from_inner(crate::sync::net::TcpStream::from_cap_std(stream))
    }
}

#[cfg(unix)]
pub struct UnixListener(crate::sync::net::UnixListener);

#[cfg(unix)]
impl UnixListener {
    pub(crate) fn from_inner(listener: crate::sync::net::UnixListener) -> Self {
        UnixListener(listener)
    }
    pub fn from_cap_std(listener: cap_std::os::unix::net::UnixListener) -> Self {
        Self::from_inner(crate::sync::net::UnixListener::from_cap_std(listener))
    }
}

#[cfg(unix)]
pub struct UnixStream(crate::sync::net::UnixStream);

#[cfg(unix)]
impl UnixStream {
    fn from_inner(stream: crate::sync::net::UnixStream) -> Self {
        UnixStream(stream)
    }
    pub fn from_cap_std(stream: cap_std::os::unix::net::UnixStream) -> Self {
        Self::from_inner(crate::sync::net::UnixStream::from_cap_std(stream))
    }
}

pub struct Stdin(crate::sync::stdio::Stdin);

pub fn stdin() -> Stdin {
    Stdin(crate::sync::stdio::stdin())
}

pub struct Stdout(crate::sync::stdio::Stdout);

pub fn stdout() -> Stdout {
    Stdout(crate::sync::stdio::stdout())
}

pub struct Stderr(crate::sync::stdio::Stderr);

pub fn stderr() -> Stderr {
    Stderr(crate::sync::stdio::stderr())
}

macro_rules! wasi_file_impl {
    ($ty:ty) => {
        #[wiggle::async_trait]
        impl WasiFile for $ty {
            fn as_any(&self) -> &dyn Any {
                self
            }
            #[cfg(unix)]
            fn pollable(&self) -> Option<rustix::fd::BorrowedFd> {
                Some(self.0.as_fd())
            }
            #[cfg(windows)]
            fn pollable(&self) -> Option<io_extras::os::windows::RawHandleOrSocket> {
                Some(self.0.as_raw_handle_or_socket())
            }
            async fn datasync(&self) -> Result<(), Error> {
                block_on_dummy_executor(|| self.0.datasync())
            }
            async fn sync(&self) -> Result<(), Error> {
                block_on_dummy_executor(|| self.0.sync())
            }
            async fn get_filetype(&self) -> Result<FileType, Error> {
                block_on_dummy_executor(|| self.0.get_filetype())
            }
            async fn get_fdflags(&self) -> Result<FdFlags, Error> {
                block_on_dummy_executor(|| self.0.get_fdflags())
            }
            async fn set_fdflags(&mut self, fdflags: FdFlags) -> Result<(), Error> {
                block_on_dummy_executor(|| self.0.set_fdflags(fdflags))
            }
            async fn get_filestat(&self) -> Result<Filestat, Error> {
                block_on_dummy_executor(|| self.0.get_filestat())
            }
            async fn set_filestat_size(&self, size: u64) -> Result<(), Error> {
                block_on_dummy_executor(move || self.0.set_filestat_size(size))
            }
            async fn advise(&self, offset: u64, len: u64, advice: Advice) -> Result<(), Error> {
                block_on_dummy_executor(move || self.0.advise(offset, len, advice))
            }
            async fn read_vectored<'a>(
                &self,
                bufs: &mut [io::IoSliceMut<'a>],
            ) -> Result<u64, Error> {
                block_on_dummy_executor(move || self.0.read_vectored(bufs))
            }
            async fn read_vectored_at<'a>(
                &self,
                bufs: &mut [io::IoSliceMut<'a>],
                offset: u64,
            ) -> Result<u64, Error> {
                block_on_dummy_executor(move || self.0.read_vectored_at(bufs, offset))
            }
            async fn write_vectored<'a>(&self, bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
                block_on_dummy_executor(move || self.0.write_vectored(bufs))
            }
            async fn write_vectored_at<'a>(
                &self,
                bufs: &[io::IoSlice<'a>],
                offset: u64,
            ) -> Result<u64, Error> {
                if bufs.iter().map(|i| i.len()).sum::<usize>() == 0 {
                    return Ok(0);
                }
                block_on_dummy_executor(move || self.0.write_vectored_at(bufs, offset))
            }
            async fn seek(&self, pos: std::io::SeekFrom) -> Result<u64, Error> {
                block_on_dummy_executor(move || self.0.seek(pos))
            }
            async fn peek(&self, buf: &mut [u8]) -> Result<u64, Error> {
                block_on_dummy_executor(move || self.0.peek(buf))
            }
            async fn set_times(
                &self,
                atime: Option<crate::SystemTimeSpec>,
                mtime: Option<crate::SystemTimeSpec>,
            ) -> Result<(), Error> {
                block_on_dummy_executor(move || self.0.set_times(atime, mtime))
            }
            fn num_ready_bytes(&self) -> Result<u64, Error> {
                self.0.num_ready_bytes()
            }
            fn isatty(&self) -> bool {
                self.0.isatty()
            }

            #[cfg(not(windows))]
            async fn readable(&self) -> Result<(), Error> {
                // The Inner impls OwnsRaw, which asserts exclusive use of the handle by the owned object.
                // AsyncFd needs to wrap an owned `impl std::os::unix::io::AsRawFd`. Rather than introduce
                // mutability to let it own the `Inner`, we are depending on the `&mut self` bound on this
                // async method to ensure this is the only Future which can access the RawFd during the
                // lifetime of the AsyncFd.
                use std::os::unix::io::AsRawFd;
                use tokio::io::{Interest, unix::AsyncFd};
                let rawfd = self.0.borrow().as_fd().as_raw_fd();
                match AsyncFd::with_interest(rawfd, Interest::READABLE) {
                    Ok(asyncfd) => {
                        let _ = asyncfd.readable().await?;
                        Ok(())
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                        // if e is EPERM, this file isn't supported by epoll because it is immediately
                        // available for reading:
                        Ok(())
                    }
                    Err(e) => Err(e.into()),
                }
            }

            #[cfg(not(windows))]
            async fn writable(&self) -> Result<(), Error> {
                // The Inner impls OwnsRaw, which asserts exclusive use of the handle by the owned object.
                // AsyncFd needs to wrap an owned `impl std::os::unix::io::AsRawFd`. Rather than introduce
                // mutability to let it own the `Inner`, we are depending on the `&mut self` bound on this
                // async method to ensure this is the only Future which can access the RawFd during the
                // lifetime of the AsyncFd.
                use std::os::unix::io::AsRawFd;
                use tokio::io::{Interest, unix::AsyncFd};
                let rawfd = self.0.borrow().as_fd().as_raw_fd();
                match AsyncFd::with_interest(rawfd, Interest::WRITABLE) {
                    Ok(asyncfd) => {
                        let _ = asyncfd.writable().await?;
                        Ok(())
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                        // if e is EPERM, this file isn't supported by epoll because it is immediately
                        // available for writing:
                        Ok(())
                    }
                    Err(e) => Err(e.into()),
                }
            }

            async fn sock_accept(&self, fdflags: FdFlags) -> Result<Box<dyn WasiFile>, Error> {
                block_on_dummy_executor(|| self.0.sock_accept(fdflags))
            }
        }
        #[cfg(windows)]
        impl AsRawHandleOrSocket for $ty {
            #[inline]
            fn as_raw_handle_or_socket(&self) -> RawHandleOrSocket {
                self.0.borrow().as_raw_handle_or_socket()
            }
        }
    };
}

wasi_file_impl!(File);
wasi_file_impl!(TcpListener);
wasi_file_impl!(TcpStream);
#[cfg(unix)]
wasi_file_impl!(UnixListener);
#[cfg(unix)]
wasi_file_impl!(UnixStream);
wasi_file_impl!(Stdin);
wasi_file_impl!(Stdout);
wasi_file_impl!(Stderr);
