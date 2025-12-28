use test_programs::p3::wasi::sockets::types::{
    IpAddress, IpAddressFamily, IpSocketAddress, UdpSocket,
};

struct Component;

test_programs::p3::export!(Component);

// Send without prior `bind` or `connect` performs an implicit bind.
async fn test_udp_send_without_bind_or_connect(family: IpAddressFamily) {
    let message = b"Hello, world!";
    let remote_addr = IpSocketAddress::new(IpAddress::new_loopback(family), 42);

    let sock = UdpSocket::create(family).unwrap();

    assert!(matches!(sock.get_local_address(), Err(_)));

    assert!(matches!(
        sock.send(message.to_vec(), Some(remote_addr)).await,
        Ok(_)
    ));

    assert!(matches!(sock.get_local_address(), Ok(_)));
    assert!(matches!(sock.get_remote_address(), Err(_)));
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        test_udp_send_without_bind_or_connect(IpAddressFamily::Ipv4).await;
        test_udp_send_without_bind_or_connect(IpAddressFamily::Ipv6).await;

        Ok(())
    }
}

fn main() {}
