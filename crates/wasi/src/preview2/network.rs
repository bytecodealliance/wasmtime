use crate::preview2::bindings::sockets::network::{Ipv4Address, Ipv6Address};
use crate::preview2::bindings::wasi::sockets::network::ErrorCode;
use crate::preview2::TrappableError;
use cap_net_ext::{PoolExt, TcpBinder, TcpConnecter, UdpBinder, UdpConnecter};
use cap_std::ipnet::IpNet;
use cap_std::net as cap_net;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;

pub struct Network {
    pub pool: Arc<Pool>,
    pub allow_ip_name_lookup: bool,
}

pub struct Pool {
    inner: cap_net::Pool,
    all_allowed: cap_net::Pool,
    check: Option<Box<dyn Fn(SocketAddr) -> bool + Send + Sync>>,
}

impl Pool {
    pub fn new() -> Self {
        let mut all_allowed = cap_net::Pool::new();
        all_allowed.insert_ip_net_port_any(
            IpNet::new(Ipv4Addr::UNSPECIFIED.into(), 0).unwrap(),
            cap_std::ambient_authority(),
        );
        all_allowed.insert_ip_net_port_any(
            IpNet::new(Ipv6Addr::UNSPECIFIED.into(), 0).unwrap(),
            cap_std::ambient_authority(),
        );

        Self {
            inner: cap_net::Pool::new(),
            all_allowed,
            check: None,
        }
    }

    pub fn check_addr(&self, addr: SocketAddr) -> std::io::Result<()> {
        // We don't actually use the connecter, we just use it to verify that `addr` is allowed
        let _ = self.checked_inner(addr)?.udp_binder(addr)?;
        Ok(())
    }

    pub fn tcp_binder(&self, addr: SocketAddr) -> std::io::Result<TcpBinder> {
        self.checked_inner(addr)?.tcp_binder(addr)
    }

    pub fn tcp_connecter(&self, addr: SocketAddr) -> std::io::Result<TcpConnecter> {
        self.checked_inner(addr)?.tcp_connecter(addr)
    }

    pub fn udp_binder(&self, addr: SocketAddr) -> std::io::Result<UdpBinder> {
        self.checked_inner(addr)?.udp_binder(addr)
    }

    pub fn udp_connecter(&self, addr: SocketAddr) -> std::io::Result<UdpConnecter> {
        self.checked_inner(addr)?.udp_connecter(addr)
    }

    pub fn add_dynamic_check(
        &mut self,
        func: Box<dyn Fn(SocketAddr) -> bool + Send + Sync + 'static>,
    ) {
        self.check = Some(func)
    }

    pub fn inherit_network(&mut self) {
        self.inner.insert_ip_net_port_any(
            IpNet::new(Ipv4Addr::UNSPECIFIED.into(), 0).unwrap(),
            cap_std::ambient_authority(),
        );
        self.inner.insert_ip_net_port_any(
            IpNet::new(Ipv6Addr::UNSPECIFIED.into(), 0).unwrap(),
            cap_std::ambient_authority(),
        );
    }

    pub fn insert<A: cap_std::net::ToSocketAddrs>(&mut self, addrs: A) -> std::io::Result<()> {
        self.inner.insert(addrs, cap_std::ambient_authority())
    }

    pub(crate) fn insert_ip_net_port_any(&mut self, ip_net: IpNet) {
        self.inner
            .insert_ip_net_port_any(ip_net, cap_std::ambient_authority());
    }

    pub(crate) fn insert_ip_net_port_range(
        &mut self,
        ip_net: IpNet,
        ports_start: u16,
        ports_end: Option<u16>,
    ) {
        self.inner.insert_ip_net_port_range(
            ip_net,
            ports_start,
            ports_end,
            cap_std::ambient_authority(),
        )
    }

    pub(crate) fn insert_ip_net(&mut self, ip_net: IpNet, port: u16) {
        self.inner
            .insert_ip_net(ip_net, port, cap_std::ambient_authority())
    }

    fn checked_inner(&self, addr: SocketAddr) -> std::io::Result<&cap_net::Pool> {
        if let Some(check) = &self.check {
            if check(addr) {
                return Ok(&self.all_allowed);
            }
        }
        Ok(&self.inner)
    }
}

pub type SocketResult<T> = Result<T, SocketError>;

pub type SocketError = TrappableError<ErrorCode>;

impl From<wasmtime::component::ResourceTableError> for SocketError {
    fn from(error: wasmtime::component::ResourceTableError) -> Self {
        Self::trap(error)
    }
}

impl From<std::io::Error> for SocketError {
    fn from(error: std::io::Error) -> Self {
        ErrorCode::from(error).into()
    }
}

impl From<rustix::io::Errno> for SocketError {
    fn from(error: rustix::io::Errno) -> Self {
        ErrorCode::from(error).into()
    }
}

#[derive(Copy, Clone)]
pub enum SocketAddressFamily {
    Ipv4,
    Ipv6 { v6only: bool },
}

pub(crate) fn to_ipv4_addr(addr: Ipv4Address) -> std::net::Ipv4Addr {
    let (x0, x1, x2, x3) = addr;
    std::net::Ipv4Addr::new(x0, x1, x2, x3)
}

pub(crate) fn from_ipv4_addr(addr: std::net::Ipv4Addr) -> Ipv4Address {
    let [x0, x1, x2, x3] = addr.octets();
    (x0, x1, x2, x3)
}

pub(crate) fn to_ipv6_addr(addr: Ipv6Address) -> std::net::Ipv6Addr {
    let (x0, x1, x2, x3, x4, x5, x6, x7) = addr;
    std::net::Ipv6Addr::new(x0, x1, x2, x3, x4, x5, x6, x7)
}

pub(crate) fn from_ipv6_addr(addr: std::net::Ipv6Addr) -> Ipv6Address {
    let [x0, x1, x2, x3, x4, x5, x6, x7] = addr.segments();
    (x0, x1, x2, x3, x4, x5, x6, x7)
}
