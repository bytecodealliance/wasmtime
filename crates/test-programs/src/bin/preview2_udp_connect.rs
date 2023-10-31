use test_programs::wasi::sockets::network::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, Network,
};
use test_programs::wasi::sockets::udp::UdpSocket;

fn test_udp_connect_disconnect_reconnect(net: &Network, family: IpAddressFamily) {
    let unspecified_addr = IpSocketAddress::new(IpAddress::new_unspecified(family), 0);
    let remote1 = IpSocketAddress::new(IpAddress::new_loopback(family), 4321);
    let remote2 = IpSocketAddress::new(IpAddress::new_loopback(family), 4320);

    let client = UdpSocket::new(family).unwrap();
    client.blocking_bind(&net, unspecified_addr).unwrap();

    _ = client.stream(None).unwrap();
    assert_eq!(client.remote_address(), Err(ErrorCode::InvalidState));

    _ = client.stream(None).unwrap();
    assert_eq!(client.remote_address(), Err(ErrorCode::InvalidState));

    _ = client.stream(Some(remote1)).unwrap();
    assert_eq!(client.remote_address(), Ok(remote1));

    _ = client.stream(Some(remote1)).unwrap();
    assert_eq!(client.remote_address(), Ok(remote1));

    _ = client.stream(Some(remote2)).unwrap();
    assert_eq!(client.remote_address(), Ok(remote2));

    _ = client.stream(None).unwrap();
    assert_eq!(client.remote_address(), Err(ErrorCode::InvalidState));

    _ = client.stream(Some(remote1)).unwrap();
    assert_eq!(client.remote_address(), Ok(remote1));
}

fn main() {
    let net = Network::default();

    test_udp_connect_disconnect_reconnect(&net, IpAddressFamily::Ipv4);
    test_udp_connect_disconnect_reconnect(&net, IpAddressFamily::Ipv6);
}
