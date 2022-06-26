#[cfg(windows)]
use io_extras::os::windows::{AsRawHandleOrSocket, RawHandleOrSocket};
#[cfg(unix)]
use io_lifetimes::AsFilelike;
use io_lifetimes::AsSocketlike;
#[cfg(unix)]
use io_lifetimes::{AsFd, BorrowedFd};
#[cfg(windows)]
use io_lifetimes::{AsSocket, BorrowedSocket};
use std::any::Any;
use std::convert::TryInto;
use std::io;
#[cfg(unix)]
use system_interface::fs::FileIoExt;
#[cfg(unix)]
use system_interface::fs::GetSetFdFlags;
use system_interface::io::IsReadWrite;
use system_interface::io::ReadReady;
use wasi_common::{
    file::{FdFlags, FileType, WasiFile},
    Error, ErrorExt,
};

pub enum Socket {
    TcpListener(cap_std::net::TcpListener),
    TcpStream(cap_std::net::TcpStream),
    #[cfg(unix)]
    UnixStream(cap_std::os::unix::net::UnixStream),
    #[cfg(unix)]
    UnixListener(cap_std::os::unix::net::UnixListener),
}

impl From<cap_std::net::TcpListener> for Socket {
    fn from(listener: cap_std::net::TcpListener) -> Self {
        Self::TcpListener(listener)
    }
}

impl From<cap_std::net::TcpStream> for Socket {
    fn from(stream: cap_std::net::TcpStream) -> Self {
        Self::TcpStream(stream)
    }
}

#[cfg(unix)]
impl From<cap_std::os::unix::net::UnixListener> for Socket {
    fn from(listener: cap_std::os::unix::net::UnixListener) -> Self {
        Self::UnixListener(listener)
    }
}

#[cfg(unix)]
impl From<cap_std::os::unix::net::UnixStream> for Socket {
    fn from(stream: cap_std::os::unix::net::UnixStream) -> Self {
        Self::UnixStream(stream)
    }
}

#[cfg(unix)]
impl From<Socket> for Box<dyn WasiFile> {
    fn from(listener: Socket) -> Self {
        match listener {
            Socket::TcpListener(l) => Box::new(crate::net::TcpListener::from_cap_std(l)),
            Socket::UnixListener(l) => Box::new(crate::net::UnixListener::from_cap_std(l)),
            Socket::TcpStream(l) => Box::new(crate::net::TcpStream::from_cap_std(l)),
            Socket::UnixStream(l) => Box::new(crate::net::UnixStream::from_cap_std(l)),
        }
    }
}

#[cfg(windows)]
impl From<Socket> for Box<dyn WasiFile> {
    fn from(listener: Socket) -> Self {
        match listener {
            Socket::TcpListener(l) => Box::new(crate::net::TcpListener::from_cap_std(l)),
            Socket::TcpStream(l) => Box::new(crate::net::TcpStream::from_cap_std(l)),
        }
    }
}

macro_rules! wasi_listen_write_impl {
    ($ty:ty, $stream:ty) => {
        #[async_trait::async_trait]
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
            async fn sock_accept(&mut self, fdflags: FdFlags) -> Result<Box<dyn WasiFile>, Error> {
                let (stream, _) = self.0.accept()?;
                let mut stream = <$stream>::from_cap_std(stream);
                stream.set_fdflags(fdflags).await?;
                Ok(Box::new(stream))
            }
            async fn get_filetype(&mut self) -> Result<FileType, Error> {
                Ok(FileType::SocketStream)
            }
            #[cfg(unix)]
            async fn get_fdflags(&mut self) -> Result<FdFlags, Error> {
                let fdflags = self.0.as_filelike().get_fd_flags()?;
                Ok(from_sysif_fdflags(fdflags))
            }
            async fn set_fdflags(&mut self, fdflags: FdFlags) -> Result<(), Error> {
                if fdflags == wasi_common::file::FdFlags::NONBLOCK {
                    self.0.set_nonblocking(true)?;
                } else if fdflags.is_empty() {
                    self.0.set_nonblocking(false)?;
                } else {
                    return Err(
                        Error::invalid_argument().context("cannot set anything else than NONBLOCK")
                    );
                }
                Ok(())
            }
            async fn num_ready_bytes(&self) -> Result<u64, Error> {
                Ok(1)
            }
        }

        #[cfg(windows)]
        impl AsSocket for $ty {
            #[inline]
            fn as_socket(&self) -> BorrowedSocket<'_> {
                self.0.as_socket()
            }
        }

        #[cfg(windows)]
        impl AsRawHandleOrSocket for $ty {
            #[inline]
            fn as_raw_handle_or_socket(&self) -> RawHandleOrSocket {
                self.0.as_raw_handle_or_socket()
            }
        }

        #[cfg(unix)]
        impl AsFd for $ty {
            fn as_fd(&self) -> BorrowedFd<'_> {
                self.0.as_fd()
            }
        }
    };
}

pub struct TcpListener(cap_std::net::TcpListener);

impl TcpListener {
    pub fn from_cap_std(cap_std: cap_std::net::TcpListener) -> Self {
        TcpListener(cap_std)
    }
}
wasi_listen_write_impl!(TcpListener, TcpStream);

#[cfg(unix)]
pub struct UnixListener(cap_std::os::unix::net::UnixListener);

#[cfg(unix)]
impl UnixListener {
    pub fn from_cap_std(cap_std: cap_std::os::unix::net::UnixListener) -> Self {
        UnixListener(cap_std)
    }
}

#[cfg(unix)]
wasi_listen_write_impl!(UnixListener, UnixStream);

macro_rules! wasi_stream_write_impl {
    ($ty:ty, $std_ty:ty) => {
        #[async_trait::async_trait]
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
            async fn get_filetype(&mut self) -> Result<FileType, Error> {
                Ok(FileType::SocketStream)
            }
            #[cfg(unix)]
            async fn get_fdflags(&mut self) -> Result<FdFlags, Error> {
                let fdflags = self.0.as_filelike().get_fd_flags()?;
                Ok(from_sysif_fdflags(fdflags))
            }
            async fn set_fdflags(&mut self, fdflags: FdFlags) -> Result<(), Error> {
                if fdflags == wasi_common::file::FdFlags::NONBLOCK {
                    self.0.set_nonblocking(true)?;
                } else if fdflags.is_empty() {
                    self.0.set_nonblocking(false)?;
                } else {
                    return Err(
                        Error::invalid_argument().context("cannot set anything else than NONBLOCK")
                    );
                }
                Ok(())
            }
            async fn read_vectored<'a>(
                &mut self,
                bufs: &mut [io::IoSliceMut<'a>],
            ) -> Result<u64, Error> {
                use std::io::Read;
                let n = Read::read_vectored(&mut &*self.as_socketlike_view::<$std_ty>(), bufs)?;
                Ok(n.try_into()?)
            }
            async fn write_vectored<'a>(&mut self, bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
                use std::io::Write;
                let n = Write::write_vectored(&mut &*self.as_socketlike_view::<$std_ty>(), bufs)?;
                Ok(n.try_into()?)
            }
            async fn peek(&mut self, buf: &mut [u8]) -> Result<u64, Error> {
                let n = self.0.peek(buf)?;
                Ok(n.try_into()?)
            }
            async fn num_ready_bytes(&self) -> Result<u64, Error> {
                let val = self.as_socketlike_view::<$std_ty>().num_ready_bytes()?;
                Ok(val)
            }
            async fn readable(&self) -> Result<(), Error> {
                let (readable, _writeable) = self.0.is_read_write()?;
                if readable {
                    Ok(())
                } else {
                    Err(Error::io())
                }
            }
            async fn writable(&self) -> Result<(), Error> {
                let (_readable, writeable) = self.0.is_read_write()?;
                if writeable {
                    Ok(())
                } else {
                    Err(Error::io())
                }
            }
        }
        #[cfg(unix)]
        impl AsFd for $ty {
            fn as_fd(&self) -> BorrowedFd<'_> {
                self.0.as_fd()
            }
        }

        #[cfg(windows)]
        impl AsSocket for $ty {
            /// Borrows the socket.
            fn as_socket(&self) -> BorrowedSocket<'_> {
                self.0.as_socket()
            }
        }

        #[cfg(windows)]
        impl AsRawHandleOrSocket for TcpStream {
            #[inline]
            fn as_raw_handle_or_socket(&self) -> RawHandleOrSocket {
                self.0.as_raw_handle_or_socket()
            }
        }
    };
}

pub struct TcpStream(cap_std::net::TcpStream);

impl TcpStream {
    pub fn from_cap_std(socket: cap_std::net::TcpStream) -> Self {
        TcpStream(socket)
    }
}

wasi_stream_write_impl!(TcpStream, std::net::TcpStream);

#[cfg(unix)]
pub struct UnixStream(cap_std::os::unix::net::UnixStream);

#[cfg(unix)]
impl UnixStream {
    pub fn from_cap_std(socket: cap_std::os::unix::net::UnixStream) -> Self {
        UnixStream(socket)
    }
}

#[cfg(unix)]
wasi_stream_write_impl!(UnixStream, std::os::unix::net::UnixStream);

pub fn filetype_from(ft: &cap_std::fs::FileType) -> FileType {
    use cap_fs_ext::FileTypeExt;
    if ft.is_block_device() {
        FileType::SocketDgram
    } else {
        FileType::SocketStream
    }
}

pub fn from_sysif_fdflags(f: system_interface::fs::FdFlags) -> wasi_common::file::FdFlags {
    let mut out = wasi_common::file::FdFlags::empty();
    if f.contains(system_interface::fs::FdFlags::NONBLOCK) {
        out |= wasi_common::file::FdFlags::NONBLOCK;
    }
    out
}
