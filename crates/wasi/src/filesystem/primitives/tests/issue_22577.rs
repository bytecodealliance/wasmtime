// This test module derived from Rust's src/test/ui/issues/issue-22577.rs
// at revision 108e90ca78f052c0c1c49c42a22c85620be19712.

// run-pass
#![allow(dead_code)]
// pretty-expanded FIXME #23616
// ignore-cloudabi no std::fs

use cap_std::{fs, net};

fn assert_both<T: Send + Sync>() {}
fn assert_send<T: Send>() {}

#[test]
fn issue_22577() {
    assert_both::<fs::File>();
    assert_both::<fs::Metadata>();
    assert_both::<fs::ReadDir>();
    assert_both::<fs::DirEntry>();
    assert_both::<fs::OpenOptions>();
    assert_both::<fs::Permissions>();

    assert_both::<net::TcpStream>();
    assert_both::<net::TcpListener>();
    assert_both::<net::UdpSocket>();
    assert_both::<net::SocketAddr>();
    assert_both::<net::SocketAddrV4>();
    assert_both::<net::SocketAddrV6>();
    assert_both::<net::Ipv4Addr>();
    assert_both::<net::Ipv6Addr>();
}
