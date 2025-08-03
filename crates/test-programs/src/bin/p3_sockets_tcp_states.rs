use futures::join;
use test_programs::p3::wasi::sockets::types::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, TcpSocket,
};

struct Component;

test_programs::p3::export!(Component);

fn test_tcp_unbound_state_invariants(family: IpAddressFamily) {
    let sock = TcpSocket::new(family);

    // TODO: Test send and receive
    //assert!(matches!(
    //    sock.shutdown(ShutdownType::Both),
    //    Err(ErrorCode::InvalidState)
    //));
    assert_eq!(sock.local_address(), Err(ErrorCode::InvalidState));
    assert_eq!(sock.remote_address(), Err(ErrorCode::InvalidState));
    assert!(!sock.is_listening());
    assert_eq!(sock.address_family(), family);

    assert_eq!(sock.set_listen_backlog_size(32), Ok(()));

    assert!(sock.keep_alive_enabled().is_ok());
    assert_eq!(sock.set_keep_alive_enabled(false), Ok(()));
    assert_eq!(sock.keep_alive_enabled(), Ok(false));

    assert!(sock.keep_alive_idle_time().is_ok());
    assert_eq!(sock.set_keep_alive_idle_time(1), Ok(()));

    assert!(sock.keep_alive_interval().is_ok());
    assert_eq!(sock.set_keep_alive_interval(1), Ok(()));

    assert!(sock.keep_alive_count().is_ok());
    assert_eq!(sock.set_keep_alive_count(1), Ok(()));

    assert!(sock.hop_limit().is_ok());
    assert_eq!(sock.set_hop_limit(255), Ok(()));
    assert_eq!(sock.hop_limit(), Ok(255));

    assert!(sock.receive_buffer_size().is_ok());
    assert_eq!(sock.set_receive_buffer_size(16000), Ok(()));

    assert!(sock.send_buffer_size().is_ok());
    assert_eq!(sock.set_send_buffer_size(16000), Ok(()));
}

fn test_tcp_bound_state_invariants(family: IpAddressFamily) {
    let bind_address = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let sock = TcpSocket::new(family);
    sock.bind(bind_address).unwrap();

    assert_eq!(sock.bind(bind_address), Err(ErrorCode::InvalidState));
    // TODO: Test send and receive
    //assert!(matches!(
    //    sock.shutdown(ShutdownType::Both),
    //    Err(ErrorCode::InvalidState)
    //));

    assert!(sock.local_address().is_ok());
    assert_eq!(sock.remote_address(), Err(ErrorCode::InvalidState));
    assert!(!sock.is_listening());
    assert_eq!(sock.address_family(), family);

    assert_eq!(sock.set_listen_backlog_size(32), Ok(()));

    assert!(sock.keep_alive_enabled().is_ok());
    assert_eq!(sock.set_keep_alive_enabled(false), Ok(()));
    assert_eq!(sock.keep_alive_enabled(), Ok(false));

    assert!(sock.keep_alive_idle_time().is_ok());
    assert_eq!(sock.set_keep_alive_idle_time(1), Ok(()));

    assert!(sock.keep_alive_interval().is_ok());
    assert_eq!(sock.set_keep_alive_interval(1), Ok(()));

    assert!(sock.keep_alive_count().is_ok());
    assert_eq!(sock.set_keep_alive_count(1), Ok(()));

    assert!(sock.hop_limit().is_ok());
    assert_eq!(sock.set_hop_limit(255), Ok(()));
    assert_eq!(sock.hop_limit(), Ok(255));

    assert!(sock.receive_buffer_size().is_ok());
    assert_eq!(sock.set_receive_buffer_size(16000), Ok(()));

    assert!(sock.send_buffer_size().is_ok());
    assert_eq!(sock.set_send_buffer_size(16000), Ok(()));
}

async fn test_tcp_listening_state_invariants(family: IpAddressFamily) {
    let bind_address = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let sock = TcpSocket::new(family);
    sock.bind(bind_address).unwrap();
    sock.listen().unwrap();

    assert_eq!(sock.bind(bind_address), Err(ErrorCode::InvalidState));
    assert_eq!(
        sock.connect(IpSocketAddress::new(IpAddress::new_loopback(family), 1))
            .await,
        Err(ErrorCode::InvalidState)
    );
    assert!(matches!(sock.listen(), Err(ErrorCode::InvalidState)));
    // Skipping: tcp::accept
    // TODO: Test send and receive
    //assert!(matches!(
    //    sock.shutdown(ShutdownType::Both),
    //    Err(ErrorCode::InvalidState)
    //));

    assert!(sock.local_address().is_ok());
    assert_eq!(sock.remote_address(), Err(ErrorCode::InvalidState));
    assert!(sock.is_listening());
    assert_eq!(sock.address_family(), family);

    assert!(matches!(
        sock.set_listen_backlog_size(32),
        Ok(_) | Err(ErrorCode::NotSupported)
    ));

    assert!(sock.keep_alive_enabled().is_ok());
    assert_eq!(sock.set_keep_alive_enabled(false), Ok(()));
    assert_eq!(sock.keep_alive_enabled(), Ok(false));

    assert!(sock.keep_alive_idle_time().is_ok());
    assert_eq!(sock.set_keep_alive_idle_time(1), Ok(()));

    assert!(sock.keep_alive_interval().is_ok());
    assert_eq!(sock.set_keep_alive_interval(1), Ok(()));

    assert!(sock.keep_alive_count().is_ok());
    assert_eq!(sock.set_keep_alive_count(1), Ok(()));

    assert!(sock.hop_limit().is_ok());
    assert_eq!(sock.set_hop_limit(255), Ok(()));
    assert_eq!(sock.hop_limit(), Ok(255));

    assert!(sock.receive_buffer_size().is_ok());
    assert_eq!(sock.set_receive_buffer_size(16000), Ok(()));

    assert!(sock.send_buffer_size().is_ok());
    assert_eq!(sock.set_send_buffer_size(16000), Ok(()));
}

async fn test_tcp_connected_state_invariants(family: IpAddressFamily) {
    let bind_address = IpSocketAddress::new(IpAddress::new_loopback(family), 0);
    let sock_listener = TcpSocket::new(family);
    sock_listener.bind(bind_address).unwrap();
    let mut accept = sock_listener.listen().unwrap();
    let addr_listener = sock_listener.local_address().unwrap();
    let sock = TcpSocket::new(family);
    join!(
        async {
            sock.connect(addr_listener).await.unwrap();
        },
        async {
            accept.next().await.unwrap();
        }
    );

    assert_eq!(sock.bind(bind_address), Err(ErrorCode::InvalidState));
    assert_eq!(
        sock.connect(addr_listener).await,
        Err(ErrorCode::InvalidState)
    );
    assert!(matches!(sock.listen(), Err(ErrorCode::InvalidState)));
    // Skipping: tcp::shutdown

    assert!(sock.local_address().is_ok());
    assert!(sock.remote_address().is_ok());
    assert!(!sock.is_listening());
    assert_eq!(sock.address_family(), family);

    assert!(sock.keep_alive_enabled().is_ok());
    assert_eq!(sock.set_keep_alive_enabled(false), Ok(()));
    assert_eq!(sock.keep_alive_enabled(), Ok(false));

    assert!(sock.keep_alive_idle_time().is_ok());
    assert_eq!(sock.set_keep_alive_idle_time(1), Ok(()));

    assert!(sock.keep_alive_interval().is_ok());
    assert_eq!(sock.set_keep_alive_interval(1), Ok(()));

    assert!(sock.keep_alive_count().is_ok());
    assert_eq!(sock.set_keep_alive_count(1), Ok(()));

    assert!(sock.hop_limit().is_ok());
    assert_eq!(sock.set_hop_limit(255), Ok(()));
    assert_eq!(sock.hop_limit(), Ok(255));

    assert!(sock.receive_buffer_size().is_ok());
    assert_eq!(sock.set_receive_buffer_size(16000), Ok(()));

    assert!(sock.send_buffer_size().is_ok());
    assert_eq!(sock.set_send_buffer_size(16000), Ok(()));
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        test_tcp_unbound_state_invariants(IpAddressFamily::Ipv4);
        test_tcp_unbound_state_invariants(IpAddressFamily::Ipv6);

        test_tcp_bound_state_invariants(IpAddressFamily::Ipv4);
        test_tcp_bound_state_invariants(IpAddressFamily::Ipv6);

        test_tcp_listening_state_invariants(IpAddressFamily::Ipv4).await;
        test_tcp_listening_state_invariants(IpAddressFamily::Ipv6).await;

        test_tcp_connected_state_invariants(IpAddressFamily::Ipv4).await;
        test_tcp_connected_state_invariants(IpAddressFamily::Ipv6).await;
        Ok(())
    }
}

fn main() {}
