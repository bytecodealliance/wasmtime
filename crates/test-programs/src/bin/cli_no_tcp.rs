//! This test assumes that it will be run without tcp support enabled

#![deny(warnings)]

use test_programs::wasi::sockets::tcp::{ErrorCode, IpAddressFamily, TcpSocket};

fn main() {
    assert!(matches!(
        TcpSocket::new(IpAddressFamily::Ipv4),
        Err(ErrorCode::AccessDenied)
    ));
}
