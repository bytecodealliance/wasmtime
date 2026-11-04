use core::future::Future as _;
use core::pin::pin;
use core::task::{Context, Waker};

use test_programs::p3::wasi::sockets::types::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, UdpSocket,
};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let port = |name| std::env::var(name).unwrap().parse().unwrap();
        let loopback = IpAddress::new_loopback(IpAddressFamily::Ipv4);
        let allowed = IpSocketAddress::new(loopback, port("ALLOWED_PORT"));
        let forbidden = IpSocketAddress::new(loopback, port("FORBIDDEN_PORT"));

        let sock = UdpSocket::create(IpAddressFamily::Ipv4).unwrap();
        sock.connect(allowed).unwrap();

        let mut send = pin!(sock.send(b"x".to_vec(), None));
        let poll = send.as_mut().poll(&mut Context::from_waker(Waker::noop()));
        assert!(poll.is_pending());
        assert!(matches!(
            sock.connect(forbidden),
            Err(ErrorCode::AccessDenied)
        ));
        send.await.unwrap();

        sock.connect(allowed).unwrap();
        let mut send = pin!(sock.send(b"z".to_vec(), None));
        let poll = send.as_mut().poll(&mut Context::from_waker(Waker::noop()));
        assert!(poll.is_pending());
        sock.disconnect().unwrap();
        assert!(matches!(send.await, Err(ErrorCode::InvalidState)));

        Ok(())
    }
}

fn main() {}
