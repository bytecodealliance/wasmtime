use cap_net_ext::{AddressFamily, Blocking, PoolExt, TcpListenerExt, UdpSocketExt};
use cap_std::net::{Pool, Shutdown, SocketAddr, TcpListener, TcpStream};
use io_extras::borrowed::BorrowedReadable;
#[cfg(windows)]
use io_extras::os::windows::{AsHandleOrSocket, BorrowedHandleOrSocket};
use io_lifetimes::AsSocketlike;
#[cfg(windows)]
use io_lifetimes::{AsSocket, BorrowedSocket};
use rustix::fd::{AsFd, BorrowedFd, OwnedFd};
use std::any::Any;
use std::convert::TryInto;
use std::io::{self, Read, Write};
use std::sync::Arc;
use system_interface::io::{IoExt, IsReadWrite, ReadReady};
use wasi_common::{
    network::WasiNetwork,
    stream::{InputStream, OutputStream},
    tcp_socket::WasiTcpSocket,
    udp_socket::{RiFlags, RoFlags, WasiUdpSocket},
    Error, ErrorExt,
};

pub struct Network(Pool);
pub struct TcpSocket(Arc<TcpListener>);
pub struct UdpSocket(Arc<cap_std::net::UdpSocket>);

impl Network {
    pub fn new(pool: Pool) -> Self {
        Self(pool)
    }
}

impl TcpSocket {
    pub fn new(family: AddressFamily) -> io::Result<Self> {
        Ok(Self(Arc::new(TcpListener::new(family, Blocking::Yes)?)))
    }

    pub fn sock(fd: OwnedFd) -> Self {
        Self(Arc::new(TcpListener::from(fd)))
    }

    pub fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl UdpSocket {
    pub fn new(family: AddressFamily) -> io::Result<Self> {
        Ok(Self(Arc::new(cap_std::net::UdpSocket::new(
            family,
            Blocking::Yes,
        )?)))
    }

    pub fn sock(owned: OwnedFd) -> Self {
        Self(Arc::new(cap_std::net::UdpSocket::from(owned)))
    }

    pub fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

#[async_trait::async_trait]
impl WasiTcpSocket for TcpSocket {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn pollable(&self) -> BorrowedFd<'_> {
        self.as_fd()
    }

    async fn bind(
        &self,
        network: &dyn WasiNetwork,
        local_address: SocketAddr,
    ) -> Result<(), Error> {
        network
            .pool()
            .bind_existing_tcp_listener(&self.0, local_address)?;
        Ok(())
    }

    async fn listen(
        &self,
        _network: &dyn WasiNetwork, // FIXME: Can we remove this from the wit?
    ) -> Result<(), Error> {
        self.0.listen(None)?;
        Ok(())
    }

    async fn accept(
        &self,
        nonblocking: bool,
    ) -> Result<
        (
            Box<dyn WasiTcpSocket>,
            Box<dyn InputStream>,
            Box<dyn OutputStream>,
            SocketAddr,
        ),
        Error,
    > {
        let blocking = match nonblocking {
            true => Blocking::No,
            false => Blocking::Yes,
        };
        let (connection, addr) = self.0.accept_with(blocking)?;
        let connection = TcpSocket::sock(connection.into());
        let input_stream = connection.clone();
        let output_stream = connection.clone();
        Ok((
            Box::new(connection),
            Box::new(input_stream),
            Box::new(output_stream),
            addr,
        ))
    }

    async fn connect(
        &self,
        network: &dyn WasiNetwork,
        remote_address: SocketAddr,
    ) -> Result<(Box<dyn InputStream>, Box<dyn OutputStream>), Error> {
        network
            .pool()
            .connect_existing_tcp_listener(&self.0, remote_address)?;
        let input_stream = self.clone();
        let output_stream = self.clone();
        Ok((Box::new(input_stream), Box::new(output_stream)))
    }

    async fn shutdown(&self, how: Shutdown) -> Result<(), Error> {
        self.as_socketlike_view::<TcpStream>().shutdown(how)?;
        Ok(())
    }

    fn local_address(&self) -> Result<SocketAddr, Error> {
        Ok(self.as_socketlike_view::<TcpStream>().local_addr()?)
    }

    fn remote_address(&self) -> Result<SocketAddr, Error> {
        Ok(self.as_socketlike_view::<TcpStream>().peer_addr()?)
    }

    fn nodelay(&self) -> Result<bool, Error> {
        let value = self.as_socketlike_view::<TcpStream>().nodelay()?;
        Ok(value)
    }

    fn set_nodelay(&self, flag: bool) -> Result<(), Error> {
        self.as_socketlike_view::<TcpStream>().set_nodelay(flag)?;
        Ok(())
    }

    fn v6_only(&self) -> Result<bool, Error> {
        let value = rustix::net::sockopt::get_ipv6_v6only(self).map_err(io::Error::from)?;
        Ok(value)
    }

    fn set_v6_only(&self, value: bool) -> Result<(), Error> {
        rustix::net::sockopt::set_ipv6_v6only(self, value).map_err(io::Error::from)?;
        Ok(())
    }

    fn set_nonblocking(&mut self, flag: bool) -> Result<(), Error> {
        self.as_socketlike_view::<TcpStream>()
            .set_nonblocking(flag)?;
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
impl WasiUdpSocket for UdpSocket {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn set_nonblocking(&mut self, flag: bool) -> Result<(), Error> {
        self.0
            .as_socketlike_view::<TcpStream>()
            .set_nonblocking(flag)?;
        Ok(())
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
                let n = self.0.as_socketlike_view::<TcpStream>().peek(first)?;
                return Ok((n as u64, RoFlags::empty()));
            } else {
                return Ok((0, RoFlags::empty()));
            }
        }

        if ri_flags.contains(RiFlags::RECV_WAITALL) {
            let n: usize = ri_data.iter().map(|buf| buf.len()).sum();
            self.0
                .as_socketlike_view::<TcpStream>()
                .read_exact_vectored(ri_data)?;
            return Ok((n as u64, RoFlags::empty()));
        }

        let n = self
            .0
            .as_socketlike_view::<TcpStream>()
            .read_vectored(ri_data)?;
        Ok((n as u64, RoFlags::empty()))
    }

    async fn sock_send<'a>(&mut self, si_data: &[io::IoSlice<'a>]) -> Result<u64, Error> {
        let n = self
            .0
            .as_socketlike_view::<TcpStream>()
            .write_vectored(si_data)?;
        Ok(n as u64)
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
impl InputStream for TcpSocket {
    fn as_any(&self) -> &dyn Any {
        self
    }
    #[cfg(unix)]
    fn pollable_read(&self) -> Option<BorrowedFd> {
        Some(self.as_fd())
    }

    #[cfg(windows)]
    fn pollable_read(&self) -> Option<io_extras::os::windows::BorrowedHandleOrSocket> {
        Some(BorrowedHandleOrSocket::from_socket(self.as_socket()))
    }

    async fn read(&mut self, buf: &mut [u8]) -> Result<(u64, bool), Error> {
        match Read::read(&mut &*self.as_socketlike_view::<TcpStream>(), buf) {
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
        match Read::read_vectored(&mut &*self.as_socketlike_view::<TcpStream>(), bufs) {
            Ok(0) => Ok((0, true)),
            Ok(n) => Ok((n as u64, false)),
            Err(err) if err.kind() == io::ErrorKind::Interrupted => Ok((0, false)),
            Err(err) => Err(err.into()),
        }
    }
    #[cfg(can_vector)]
    fn is_read_vectored(&self) -> bool {
        Read::is_read_vectored(&mut &*self.as_socketlike_view::<TcpStream>())
    }

    async fn skip(&mut self, nelem: u64) -> Result<(u64, bool), Error> {
        let num = io::copy(
            &mut io::Read::take(&*self.as_socketlike_view::<TcpStream>(), nelem),
            &mut io::sink(),
        )?;
        Ok((num, num < nelem))
    }

    async fn num_ready_bytes(&self) -> Result<u64, Error> {
        let val = self.as_socketlike_view::<TcpStream>().num_ready_bytes()?;
        Ok(val)
    }

    async fn readable(&self) -> Result<(), Error> {
        if is_read_write(&*self.as_socketlike_view::<TcpStream>())?.0 {
            Ok(())
        } else {
            Err(Error::badf())
        }
    }
}

#[async_trait::async_trait]
impl OutputStream for TcpSocket {
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[cfg(unix)]
    fn pollable_write(&self) -> Option<BorrowedFd> {
        Some(self.as_fd())
    }

    #[cfg(windows)]
    fn pollable_write(&self) -> Option<io_extras::os::windows::BorrowedHandleOrSocket> {
        Some(BorrowedHandleOrSocket::from_socket(self.as_socket()))
    }

    async fn write(&mut self, buf: &[u8]) -> Result<u64, Error> {
        let n = Write::write(&mut &*self.as_socketlike_view::<TcpStream>(), buf)?;
        Ok(n.try_into()?)
    }
    async fn write_vectored<'a>(&mut self, bufs: &[io::IoSlice<'a>]) -> Result<u64, Error> {
        let n = Write::write_vectored(&mut &*self.as_socketlike_view::<TcpStream>(), bufs)?;
        Ok(n.try_into()?)
    }
    #[cfg(can_vector)]
    fn is_write_vectored(&self) -> bool {
        Write::is_write_vectored(&mut &*self.as_socketlike_view::<TcpStream>())
    }
    async fn splice(
        &mut self,
        src: &mut dyn InputStream,
        nelem: u64,
    ) -> Result<(u64, bool), Error> {
        if let Some(readable) = src.pollable_read() {
            let num = io::copy(
                &mut io::Read::take(BorrowedReadable::borrow(readable), nelem),
                &mut &*self.as_socketlike_view::<TcpStream>(),
            )?;
            Ok((num, num < nelem))
        } else {
            OutputStream::splice(self, src, nelem).await
        }
    }
    async fn write_zeroes(&mut self, nelem: u64) -> Result<u64, Error> {
        let num = io::copy(
            &mut io::Read::take(io::repeat(0), nelem),
            &mut &*self.as_socketlike_view::<TcpStream>(),
        )?;
        Ok(num)
    }
    async fn writable(&self) -> Result<(), Error> {
        if is_read_write(&*self.as_socketlike_view::<TcpStream>())?.1 {
            Ok(())
        } else {
            Err(Error::badf())
        }
    }
}

#[async_trait::async_trait]
impl WasiNetwork for Network {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn pool(&self) -> &Pool {
        &self.0
    }
}

#[cfg(unix)]
impl AsFd for TcpSocket {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

#[cfg(unix)]
impl AsFd for UdpSocket {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

#[cfg(windows)]
impl AsSocket for TcpSocket {
    /// Borrows the socket.
    fn as_socket(&self) -> BorrowedSocket<'_> {
        self.0.as_socket()
    }
}

#[cfg(windows)]
impl AsHandleOrSocket for TcpSocket {
    #[inline]
    fn as_handle_or_socket(&self) -> BorrowedHandleOrSocket {
        BorrowedHandleOrSocket::from_socket(self.as_socket())
    }
}
#[cfg(windows)]
impl AsSocket for UdpSocket {
    /// Borrows the socket.
    fn as_socket(&self) -> BorrowedSocket<'_> {
        self.0.as_socket()
    }
}

#[cfg(windows)]
impl AsHandleOrSocket for UdpSocket {
    #[inline]
    fn as_handle_or_socket(&self) -> BorrowedHandleOrSocket {
        BorrowedHandleOrSocket::from_socket(self.0.as_socket())
    }
}

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
        f.as_socketlike_view::<cap_std::net::TcpStream>()
            .is_read_write()
    }
}
