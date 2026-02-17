use wasip2::filesystem::types::*;
use wasip2::sockets::network::*;

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("poll") => poll(),
        Some("read") => read(),
        Some("write") => write(),
        Some("write-stream") => write_stream(),
        Some("write-stream-blocking") => write_stream_blocking(),
        Some("mkdir") => mkdir(),
        Some("resolve") => resolve(),
        Some("udp-send-many") => udp_send_many(),
        Some("udp-send-big") => udp_send_big(),
        Some("write-zeroes") => write_zeroes(),
        Some("write-stream-buffer-too-large") => write_stream_buffer_too_large(),
        Some("write-zeroes-buffer-too-large") => write_zeroes_buffer_too_large(),
        Some("read-file-big") => read_file_big(),
        Some("read-tcp-big") => read_tcp_big(),
        other => panic!("unknown arg {other:?}"),
    }
}

fn poll() {
    let mut events = Vec::new();
    let sub = wasip2::clocks::monotonic_clock::subscribe_duration(0);
    for _ in 0..5000 {
        events.push(&sub);
    }

    wasip2::io::poll::poll(&events);
    unreachable!()
}

fn preopen() -> Descriptor {
    let mut dirs = wasip2::filesystem::preopens::get_directories();
    assert_eq!(dirs.len(), 1);
    dirs.pop().unwrap().0
}

fn read() {
    let f = preopen()
        .open_at(
            PathFlags::empty(),
            "1mb",
            OpenFlags::empty(),
            DescriptorFlags::empty(),
        )
        .unwrap();

    // 0-length is ok
    f.read(0, 0).unwrap();

    // This isn't transferring data from the guest to the host, so this is ok.
    f.read(1 << 20, 0).unwrap();

    // Host shouldn't allocate based on what the guest asks for...
    f.read(u64::MAX, 0).unwrap();

    f.read_via_stream(0)
        .unwrap()
        .blocking_read(1 << 20)
        .unwrap();
}

fn write() {
    let f = preopen()
        .open_at(
            PathFlags::empty(),
            "hi",
            OpenFlags::CREATE,
            DescriptorFlags::empty(),
        )
        .unwrap();
    f.write(&[0; 5001], 0).unwrap();
    unreachable!()
}

fn write_stream() {
    preopen()
        .open_at(
            PathFlags::empty(),
            "hi",
            OpenFlags::CREATE,
            DescriptorFlags::empty(),
        )
        .unwrap()
        .write_via_stream(0)
        .unwrap()
        .write(&[0; 5001])
        .unwrap();
    unreachable!()
}

fn write_stream_blocking() {
    preopen()
        .open_at(
            PathFlags::empty(),
            "hi",
            OpenFlags::CREATE,
            DescriptorFlags::empty(),
        )
        .unwrap()
        .write_via_stream(0)
        .unwrap()
        .blocking_write_and_flush(&[0; 5001])
        .unwrap();
    unreachable!()
}

fn mkdir() {
    let mut name = String::new();
    for _ in 0..5001 {
        name.push_str("a");
    }
    preopen().create_directory_at(&name).unwrap();
    unreachable!()
}

fn resolve() {
    let network = wasip2::sockets::instance_network::instance_network();
    let mut name = String::new();
    for _ in 0..5001 {
        name.push_str("a");
    }
    wasip2::sockets::ip_name_lookup::resolve_addresses(&network, &name).unwrap();
    unreachable!();
}

fn udp_socket() -> wasip2::sockets::udp::UdpSocket {
    let socket =
        wasip2::sockets::udp_create_socket::create_udp_socket(IpAddressFamily::Ipv4).unwrap();
    let network = wasip2::sockets::instance_network::instance_network();

    socket
        .start_bind(
            &network,
            IpSocketAddress::Ipv4(Ipv4SocketAddress {
                address: (127, 0, 0, 1),
                port: 0,
            }),
        )
        .unwrap();
    socket.finish_bind().unwrap();
    socket
}

fn udp_send_many() {
    let socket = udp_socket();
    let (_incoming, outgoing) = socket.stream(None).unwrap();
    let mut dgrams = Vec::new();

    for _ in 0..5000 {
        dgrams.push(wasip2::sockets::udp::OutgoingDatagram {
            data: Vec::new(),
            remote_address: None,
        });
    }

    outgoing.send(&dgrams).unwrap();
    unreachable!()
}

fn udp_send_big() {
    let socket = udp_socket();
    let (_incoming, outgoing) = socket.stream(None).unwrap();
    let mut dgrams = Vec::new();

    for _ in 0..2 {
        dgrams.push(wasip2::sockets::udp::OutgoingDatagram {
            data: vec![0; 2500],
            remote_address: None,
        });
    }

    outgoing.send(&dgrams).unwrap();
    unreachable!()
}

fn write_zeroes() {
    preopen()
        .open_at(
            PathFlags::empty(),
            "hi",
            OpenFlags::CREATE,
            DescriptorFlags::empty(),
        )
        .unwrap()
        .write_via_stream(0)
        .unwrap()
        .write_zeroes(u64::MAX)
        .unwrap();
    unreachable!()
}

fn write_stream_buffer_too_large() {
    preopen()
        .open_at(
            PathFlags::empty(),
            "hi",
            OpenFlags::CREATE,
            DescriptorFlags::empty(),
        )
        .unwrap()
        .write_via_stream(0)
        .unwrap()
        .blocking_write_and_flush(&[0; 5000])
        .unwrap();
    unreachable!()
}

fn write_zeroes_buffer_too_large() {
    preopen()
        .open_at(
            PathFlags::empty(),
            "hi",
            OpenFlags::CREATE,
            DescriptorFlags::empty(),
        )
        .unwrap()
        .write_via_stream(0)
        .unwrap()
        .blocking_write_zeroes_and_flush(5000)
        .unwrap();
    unreachable!()
}

fn read_file_big() {
    preopen()
        .open_at(
            PathFlags::empty(),
            "1mb",
            OpenFlags::empty(),
            DescriptorFlags::empty(),
        )
        .unwrap()
        .read_via_stream(0)
        .unwrap()
        .blocking_read(u64::MAX)
        .unwrap();
}

fn read_tcp_big() {
    let server =
        wasip2::sockets::tcp_create_socket::create_tcp_socket(IpAddressFamily::Ipv4).unwrap();
    let client =
        wasip2::sockets::tcp_create_socket::create_tcp_socket(IpAddressFamily::Ipv4).unwrap();

    server
        .start_bind(
            &wasip2::sockets::instance_network::instance_network(),
            IpSocketAddress::Ipv4(Ipv4SocketAddress {
                address: (127, 0, 0, 1),
                port: 0,
            }),
        )
        .unwrap();
    server.finish_bind().unwrap();
    server.start_listen().unwrap();
    server.finish_listen().unwrap();

    client
        .start_connect(
            &wasip2::sockets::instance_network::instance_network(),
            server.local_address().unwrap(),
        )
        .unwrap();
    client.subscribe().block();
    let (input, _output) = client.finish_connect().unwrap();

    {
        server.subscribe().block();
        let (socket, input, output) = server.accept().unwrap();
        drop((input, output));
        drop(socket);
    }

    match input.blocking_read(u64::MAX) {
        Err(wasip2::io::streams::StreamError::Closed) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}
