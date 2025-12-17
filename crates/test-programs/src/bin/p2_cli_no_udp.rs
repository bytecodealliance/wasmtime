//! This test assumes that it will be run without udp support enabled

#![deny(warnings)]
use test_programs::wasi::sockets::udp::{ErrorCode, IpAddressFamily, UdpSocket};

fn main() {
    assert!(matches!(
        UdpSocket::new(IpAddressFamily::Ipv4),
        Err(ErrorCode::AccessDenied)
    ));
}
