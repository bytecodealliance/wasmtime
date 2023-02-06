use io_extras::borrowed::BorrowedReadable;
#[cfg(windows)]
use io_extras::os::windows::{AsHandleOrSocket, BorrowedHandleOrSocket};
use io_lifetimes::AsSocketlike;
#[cfg(unix)]
use io_lifetimes::{AsFd, BorrowedFd};
#[cfg(windows)]
use io_lifetimes::{AsSocket, BorrowedSocket};
use std::any::Any;
use std::convert::TryInto;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::sync::Arc;
use system_interface::io::IoExt;
use system_interface::io::IsReadWrite;
use system_interface::io::ReadReady;
use wasi_common::{
    connection::{RiFlags, RoFlags, SdFlags, SiFlags, WasiConnection},
    listener::WasiListener,
    stream::{InputStream, OutputStream},
    tcp_listener::WasiTcpListener,
    Error, ErrorExt,
};

pub enum Listener {
    TcpListener(cap_std::net::TcpListener),
    #[cfg(unix)]
    UnixListener(cap_std::os::unix::net::UnixListener),
}

pub enum Connection {
    TcpStream(cap_std::net::TcpStream),
    #[cfg(unix)]
    UnixStream(cap_std::os::unix::net::UnixStream),
}

impl From<cap_std::net::TcpListener> for Listener {
    fn from(listener: cap_std::net::TcpListener) -> Self {
        Self::TcpListener(listener)
    }
}

impl From<cap_std::net::TcpStream> for Connection {
    fn from(stream: cap_std::net::TcpStream) -> Self {
        Self::TcpStream(stream)
    }
}

#[cfg(unix)]
impl From<cap_std::os::unix::net::UnixListener> for Listener {
    fn from(listener: cap_std::os::unix::net::UnixListener) -> Self {
        Self::UnixListener(listener)
    }
}

#[cfg(unix)]
impl From<cap_std::os::unix::net::UnixStream> for Connection {
    fn from(stream: cap_std::os::unix::net::UnixStream) -> Self {
        Self::UnixStream(stream)
    }
}

#[cfg(unix)]
impl From<Listener> for Box<dyn WasiListener> {
    fn from(listener: Listener) -> Self {
        match listener {
            Listener::TcpListener(l) => Box::new(crate::net::TcpListener::from_cap_std(l)),
            Listener::UnixListener(l) => Box::new(crate::net::UnixListener::from_cap_std(l)),
        }
    }
}

#[cfg(windows)]
impl From<Listener> for Box<dyn WasiListener> {
    fn from(listener: Listener) -> Self {
        match listener {
            Listener::TcpListener(l) => Box::new(crate::net::TcpListener::from_cap_std(l)),
        }
    }
}

#[cfg(unix)]
impl From<Connection> for Box<dyn WasiConnection> {
    fn from(listener: Connection) -> Self {
        match listener {
            Connection::TcpStream(l) => Box::new(crate::net::TcpStream::from_cap_std(l)),
            Connection::UnixStream(l) => Box::new(crate::net::UnixStream::from_cap_std(l)),
        }
    }
}

#[cfg(windows)]
impl From<Connection> for Box<dyn WasiConnection> {
    fn from(listener: Connection) -> Self {
        match listener {
            Connection::TcpStream(l) => Box::new(crate::net::TcpStream::from_cap_std(l)),
        }
    }
}

macro_rules! wasi_listener_impl {
    ($ty:ty, $stream:ty) => {
        #[async_trait::async_trait]
        impl WasiListener for $ty {
            fn as_any(&self) -> &dyn Any {
                self
            }

            async fn accept(
                &mut self,
                nonblocking: bool,
            ) -> Result<
                (
                    Box<dyn WasiConnection>,
                    Box<dyn InputStream>,
                    Box<dyn OutputStream>,
                ),
                Error,
            > {
                let (stream, _) = self.0.accept()?;
                stream.set_nonblocking(nonblocking)?;
                let connection = <$stream>::from_cap_std(stream);
                let input_stream = connection.clone();
                let output_stream = connection.clone();
                Ok((
                    Box::new(connection),
                    Box::new(input_stream),
                    Box::new(output_stream),
                ))
            }

            fn set_nonblocking(&mut self, flag: bool) -> Result<(), Error> {
                self.0.set_nonblocking(flag)?;
                Ok(())
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
        impl AsHandleOrSocket for $ty {
            #[inline]
            fn as_handle_or_socket(&self) -> BorrowedHandleOrSocket {
                self.0.as_handle_or_socket()
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

macro_rules! wasi_tcp_listener_impl {
    ($ty:ty, $stream:ty) => {
        #[async_trait::async_trait]
        impl WasiTcpListener for $ty {
            fn as_any(&self) -> &dyn Any {
                self
            }

            async fn accept(
                &mut self,
                nonblocking: bool,
            ) -> Result<
                (
                    Box<dyn WasiConnection>,
                    Box<dyn InputStream>,
                    Box<dyn OutputStream>,
                    SocketAddr,
                ),
                Error,
            > {
                let (stream, addr) = self.0.accept()?;
                stream.set_nonblocking(nonblocking)?;
                let connection = <$stream>::from_cap_std(stream);
                let input_stream = connection.clone();
                let output_stream = connection.clone();
                Ok((
                    Box::new(connection),
                    Box::new(input_stream),
                    Box::new(output_stream),
                    addr,
                ))
            }

            fn set_nonblocking(&mut self, flag: bool) -> Result<(), Error> {
                self.0.set_nonblocking(flag)?;
                Ok(())
            }

            fn into_listener(self) -> Box<dyn WasiListener> {
                Box::new(self)
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
wasi_listener_impl!(TcpListener, TcpStream);
wasi_tcp_listener_impl!(TcpListener, TcpStream);

#[cfg(unix)]
pub struct UnixListener(cap_std::os::unix::net::UnixListener);

#[cfg(unix)]
impl UnixListener {
    pub fn from_cap_std(cap_std: cap_std::os::unix::net::UnixListener) -> Self {
        UnixListener(cap_std)
    }
}

#[cfg(unix)]
wasi_listener_impl!(UnixListener, UnixStream);

macro_rules! wasi_stream_write_impl {
    ($ty:ty, $std_ty:ty) => {
        #[async_trait::async_trait]
        impl WasiConnection for $ty {
            fn as_any(&self) -> &dyn Any {
                self
            }

            async fn sock_recv<'a>(
                &mut self,
                ri_data: &mut [io::IoSliceMut<'a>],
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
                &mut self,
                si_data: &[io::IoSlice<'a>],
                si_flags: SiFlags,
            ) -> Result<u64, Error> {
                if si_flags != SiFlags::empty() {
                    return Err(Error::not_supported());
                }

                let n = self.0.write_vectored(si_data)?;
                Ok(n as u64)
            }

            async fn sock_shutdown(&mut self, how: SdFlags) -> Result<(), Error> {
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

            fn set_nonblocking(&mut self, flag: bool) -> Result<(), Error> {
                self.0.set_nonblocking(flag)?;
                Ok(())
            }

            async fn readable(&self) -> Result<(), Error> {
                if is_read_write(&*self.0)?.0 {
                    Ok(())
                } else {
                    Err(Error::badf())
                }
            }

            async fn writable(&self) -> Result<(), Error> {
                if is_read_write(&*self.0)?.1 {
                    Ok(())
                } else {
                    Err(Error::badf())
                }
            }
        }

        #[async_trait::async_trait]
        impl InputStream for $ty {
            fn as_any(&self) -> &dyn Any {
                self
            }
            #[cfg(unix)]
            fn pollable_read(&self) -> Option<rustix::fd::BorrowedFd> {
                Some(self.0.as_fd())
            }

            #[cfg(windows)]
            fn pollable_read(&self) -> Option<io_extras::os::windows::BorrowedHandleOrSocket> {
                Some(self.0.as_handle_or_socket())
            }

            async fn read(&mut self, buf: &mut [u8]) -> Result<(u64, bool), Error> {
                match Read::read(&mut &*self.as_socketlike_view::<$std_ty>(), buf) {
                    Ok(0) => Ok((0, true)),
                    Ok(n) => Ok((n as u64, false)),
                    Err(err) if err.kind() == io::ErrorKind::Interrupted => Ok((0, false)),
                    Err(err) => Err(err.into()),
                }
            }
            async fn read_vectored<'a>(
                &mut self,
                bufs: &mut [io::IoSliceMut<'a>],
            ) -> Result<(u64, bool), Error> {
                match Read::read_vectored(&mut &*self.as_socketlike_view::<$std_ty>(), bufs) {
                    Ok(0) => Ok((0, true)),
                    Ok(n) => Ok((n as u64, false)),
                    Err(err) if err.kind() == io::ErrorKind::Interrupted => Ok((0, false)),
                    Err(err) => Err(err.into()),
                }
            }
            #[cfg(can_vector)]
            fn is_read_vectored(&self) -> bool {
                Read::is_read_vectored(&mut &*self.as_socketlike_view::<$std_ty>())
            }

            async fn skip(&mut self, nelem: u64) -> Result<(u64, bool), Error> {
                let num = io::copy(&mut io::Read::take(&*self.0, nelem), &mut io::sink())?;
                Ok((num, num < nelem))
            }

            async fn num_ready_bytes(&self) -> Result<u64, Error> {
                let val = self.as_socketlike_view::<$std_ty>().num_ready_bytes()?;
                Ok(val)
            }

            async fn readable(&self) -> Result<(), Error> {
                if is_read_write(&*self.0)?.0 {
                    Ok(())
                } else {
                    Err(Error::badf())
                }
            }
        }
        #[async_trait::async_trait]
        impl OutputStream for $ty {
            fn as_any(&self) -> &dyn Any {
                self
            }

            #[cfg(unix)]
            fn pollable_write(&self) -> Option<rustix::fd::BorrowedFd> {
                Some(self.0.as_fd())
            }

            #[cfg(windows)]
            fn pollable_write(&self) -> Option<io_extras::os::windows::BorrowedHandleOrSocket> {
                Some(self.0.as_handle_or_socket())
            }

            async fn write(&mut self, buf: &[u8]) -> Result<u64, Error> {
                let n = Write::write(&mut &*self.as_socketlike_view::<$std_ty>(), buf)?;
                Ok(n.try_into()?)
            }
            async fn write_vectored<'a>(&mut self, bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
                let n = Write::write_vectored(&mut &*self.as_socketlike_view::<$std_ty>(), bufs)?;
                Ok(n.try_into()?)
            }
            #[cfg(can_vector)]
            fn is_write_vectored(&self) -> bool {
                Write::is_write_vectored(&mut &*self.as_socketlike_view::<$std_ty>())
            }
            async fn splice(
                &mut self,
                src: &mut dyn InputStream,
                nelem: u64,
            ) -> Result<(u64, bool), Error> {
                if let Some(readable) = src.pollable_read() {
                    let num = io::copy(
                        &mut io::Read::take(BorrowedReadable::borrow(readable), nelem),
                        &mut &*self.0,
                    )?;
                    Ok((num, num < nelem))
                } else {
                    OutputStream::splice(self, src, nelem).await
                }
            }
            async fn write_repeated(&mut self, byte: u8, nelem: u64) -> Result<u64, Error> {
                let num = io::copy(&mut io::Read::take(io::repeat(byte), nelem), &mut &*self.0)?;
                Ok(num)
            }
            async fn writable(&self) -> Result<(), Error> {
                if is_read_write(&*self.0)?.1 {
                    Ok(())
                } else {
                    Err(Error::badf())
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
        impl AsHandleOrSocket for TcpStream {
            #[inline]
            fn as_handle_or_socket(&self) -> BorrowedHandleOrSocket {
                self.0.as_handle_or_socket()
            }
        }
    };
}

pub struct TcpStream(Arc<cap_std::net::TcpStream>);

impl TcpStream {
    pub fn from_cap_std(socket: cap_std::net::TcpStream) -> Self {
        Self(Arc::new(socket))
    }

    pub fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

wasi_stream_write_impl!(TcpStream, std::net::TcpStream);

#[cfg(unix)]
pub struct UnixStream(Arc<cap_std::os::unix::net::UnixStream>);

#[cfg(unix)]
impl UnixStream {
    pub fn from_cap_std(socket: cap_std::os::unix::net::UnixStream) -> Self {
        Self(Arc::new(socket))
    }

    pub fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

#[cfg(unix)]
wasi_stream_write_impl!(UnixStream, std::os::unix::net::UnixStream);

/// Return the file-descriptor flags for a given file-like object.
///
/// This returns the flags needed to implement [`wasi_common::WasiFile::get_fdflags`].
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
