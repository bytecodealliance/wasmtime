use crate::{
    Error, ErrorExt,
    file::{FdFlags, FileType, RiFlags, RoFlags, SdFlags, SiFlags, WasiFile},
};
#[cfg(windows)]
use io_extras::os::windows::{AsRawHandleOrSocket, RawHandleOrSocket};
use io_lifetimes::AsSocketlike;
#[cfg(unix)]
use io_lifetimes::{AsFd, BorrowedFd};
#[cfg(windows)]
use io_lifetimes::{AsSocket, BorrowedSocket};
use std::any::Any;
use std::io;
#[cfg(unix)]
use system_interface::fs::GetSetFdFlags;
use system_interface::io::IoExt;
use system_interface::io::IsReadWrite;
use system_interface::io::ReadReady;

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
            Socket::TcpListener(l) => Box::new(crate::sync::net::TcpListener::from_cap_std(l)),
            Socket::UnixListener(l) => Box::new(crate::sync::net::UnixListener::from_cap_std(l)),
            Socket::TcpStream(l) => Box::new(crate::sync::net::TcpStream::from_cap_std(l)),
            Socket::UnixStream(l) => Box::new(crate::sync::net::UnixStream::from_cap_std(l)),
        }
    }
}

#[cfg(windows)]
impl From<Socket> for Box<dyn WasiFile> {
    fn from(listener: Socket) -> Self {
        match listener {
            Socket::TcpListener(l) => Box::new(crate::sync::net::TcpListener::from_cap_std(l)),
            Socket::TcpStream(l) => Box::new(crate::sync::net::TcpStream::from_cap_std(l)),
        }
    }
}

macro_rules! wasi_listen_write_impl {
    ($ty:ty, $stream:ty) => {
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
            async fn sock_accept(&self, fdflags: FdFlags) -> Result<Box<dyn WasiFile>, Error> {
                let (stream, _) = self.0.accept()?;
                let mut stream = <$stream>::from_cap_std(stream);
                stream.set_fdflags(fdflags).await?;
                Ok(Box::new(stream))
            }
            async fn get_filetype(&self) -> Result<FileType, Error> {
                Ok(FileType::SocketStream)
            }
            #[cfg(unix)]
            async fn get_fdflags(&self) -> Result<FdFlags, Error> {
                let fdflags = get_fd_flags(&self.0)?;
                Ok(fdflags)
            }
            async fn set_fdflags(&mut self, fdflags: FdFlags) -> Result<(), Error> {
                if fdflags == crate::file::FdFlags::NONBLOCK {
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
            fn num_ready_bytes(&self) -> Result<u64, Error> {
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
            async fn get_filetype(&self) -> Result<FileType, Error> {
                Ok(FileType::SocketStream)
            }
            #[cfg(unix)]
            async fn get_fdflags(&self) -> Result<FdFlags, Error> {
                let fdflags = get_fd_flags(&self.0)?;
                Ok(fdflags)
            }
            async fn set_fdflags(&mut self, fdflags: FdFlags) -> Result<(), Error> {
                if fdflags == crate::file::FdFlags::NONBLOCK {
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
                &self,
                bufs: &mut [io::IoSliceMut<'a>],
            ) -> Result<u64, Error> {
                use std::io::Read;
                let n = Read::read_vectored(&mut &*self.as_socketlike_view::<$std_ty>(), bufs)?;
                Ok(n.try_into()?)
            }
            async fn write_vectored<'a>(&self, bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
                use std::io::Write;
                let n = Write::write_vectored(&mut &*self.as_socketlike_view::<$std_ty>(), bufs)?;
                Ok(n.try_into()?)
            }
            async fn peek(&self, buf: &mut [u8]) -> Result<u64, Error> {
                let n = self.0.peek(buf)?;
                Ok(n.try_into()?)
            }
            fn num_ready_bytes(&self) -> Result<u64, Error> {
                let val = self.as_socketlike_view::<$std_ty>().num_ready_bytes()?;
                Ok(val)
            }
            async fn readable(&self) -> Result<(), Error> {
                let (readable, _writeable) = is_read_write(&self.0)?;
                if readable { Ok(()) } else { Err(Error::io()) }
            }
            async fn writable(&self) -> Result<(), Error> {
                let (_readable, writeable) = is_read_write(&self.0)?;
                if writeable { Ok(()) } else { Err(Error::io()) }
            }

            async fn sock_recv<'a>(
                &self,
                ri_data: &mut [std::io::IoSliceMut<'a>],
                ri_flags: RiFlags,
            ) -> Result<(u64, RoFlags), Error> {
                if (ri_flags & !(RiFlags::RECV_PEEK | RiFlags::RECV_WAITALL)) != RiFlags::empty() {
                    return Err(Error::not_supported());
                }

                if ri_flags.contains(RiFlags::RECV_PEEK) {
                    if let Some(first) = ri_data.iter_mut().next() {
                        let n = self.0.peek(first)?;
                        return Ok((n as u64, RoFlags::empty()));
                    } else {
                        return Ok((0, RoFlags::empty()));
                    }
                }

                if ri_flags.contains(RiFlags::RECV_WAITALL) {
                    let n: usize = ri_data.iter().map(|buf| buf.len()).sum();
                    self.0.read_exact_vectored(ri_data)?;
                    return Ok((n as u64, RoFlags::empty()));
                }

                let n = self.0.read_vectored(ri_data)?;
                Ok((n as u64, RoFlags::empty()))
            }

            async fn sock_send<'a>(
                &self,
                si_data: &[std::io::IoSlice<'a>],
                si_flags: SiFlags,
            ) -> Result<u64, Error> {
                if si_flags != SiFlags::empty() {
                    return Err(Error::not_supported());
                }

                let n = self.0.write_vectored(si_data)?;
                Ok(n as u64)
            }

            async fn sock_shutdown(&self, how: SdFlags) -> Result<(), Error> {
                let how = if how == SdFlags::RD | SdFlags::WR {
                    cap_std::net::Shutdown::Both
                } else if how == SdFlags::RD {
                    cap_std::net::Shutdown::Read
                } else if how == SdFlags::WR {
                    cap_std::net::Shutdown::Write
                } else {
                    return Err(Error::invalid_argument());
                };
                self.0.shutdown(how)?;
                Ok(())
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

/// Return the file-descriptor flags for a given file-like object.
///
/// This returns the flags needed to implement [`WasiFile::get_fdflags`].
pub fn get_fd_flags<Socketlike: AsSocketlike>(f: Socketlike) -> io::Result<crate::file::FdFlags> {
    // On Unix-family platforms, we can use the same system call that we'd use
    // for files on sockets here.
    #[cfg(not(windows))]
    {
        let mut out = crate::file::FdFlags::empty();
        if f.get_fd_flags()?
            .contains(system_interface::fs::FdFlags::NONBLOCK)
        {
            out |= crate::file::FdFlags::NONBLOCK;
        }
        Ok(out)
    }

    // On Windows, sockets are different, and there is no direct way to
    // query for the non-blocking flag. We can get a sufficient approximation
    // by testing whether a zero-length `recv` appears to block.
    #[cfg(windows)]
    match rustix::net::recv(f, &mut [], rustix::net::RecvFlags::empty()) {
        Ok(_) => Ok(crate::file::FdFlags::empty()),
        Err(rustix::io::Errno::WOULDBLOCK) => Ok(crate::file::FdFlags::NONBLOCK),
        Err(e) => Err(e.into()),
    }
}

/// Return the file-descriptor flags for a given file-like object.
///
/// This returns the flags needed to implement [`WasiFile::get_fdflags`].
pub fn is_read_write<Socketlike: AsSocketlike>(f: Socketlike) -> io::Result<(bool, bool)> {
    // On Unix-family platforms, we have an `IsReadWrite` impl.
    #[cfg(not(windows))]
    {
        f.is_read_write()
    }

    // On Windows, we only have a `TcpStream` impl, so make a view first.
    #[cfg(windows)]
    {
        f.as_socketlike_view::<std::net::TcpStream>()
            .is_read_write()
    }
}
