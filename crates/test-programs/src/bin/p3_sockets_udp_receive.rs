use test_programs::p3::wasi::sockets::types::{ErrorCode, IpAddressFamily, UdpSocket};

struct Component;

test_programs::p3::export!(Component);

// Receive requires the socket to be bound.
async fn test_udp_receive_without_bind_or_connect(family: IpAddressFamily) {
    let sock = UdpSocket::create(family).unwrap();

    assert!(matches!(sock.get_local_address(), Err(_)));

    assert!(matches!(sock.receive().await, Err(ErrorCode::InvalidState)));

    assert!(matches!(sock.get_local_address(), Err(_)));
    assert!(matches!(sock.get_remote_address(), Err(_)));
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        test_udp_receive_without_bind_or_connect(IpAddressFamily::Ipv4).await;
        test_udp_receive_without_bind_or_connect(IpAddressFamily::Ipv6).await;

        Ok(())
    }
}

fn main() {}
