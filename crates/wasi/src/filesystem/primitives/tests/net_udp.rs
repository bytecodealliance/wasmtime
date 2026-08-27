// This file is derived from Rust's library/std/src/net/udp/tests.rs at
// revision 377d1a984cd2a53327092b90aa1d8b7e22d1e347.

mod net;
mod sys_common;

use cap_std::ambient_authority;
use cap_std::net::*;
use net::{next_test_ip4, next_test_ip6};
use std::io::ErrorKind;
use std::sync::mpsc::channel;
//use sys_common::AsInner;
use std::thread;
use std::time::{Duration, Instant};

fn each_ip(f: &mut dyn FnMut(SocketAddr, SocketAddr)) {
    f(next_test_ip4(), next_test_ip4());
    f(next_test_ip6(), next_test_ip6());
}

macro_rules! t {
    ($e:expr) => {
        match $e {
            Ok(t) => t,
            Err(e) => panic!("received error for `{}`: {}", stringify!($e), e),
        }
    };
}

#[test]
fn bind_error() {
    let mut pool = Pool::new();
    pool.insert_socket_addr("1.1.1.1:9999".parse().unwrap(), ambient_authority());

    match pool.bind_udp_socket("1.1.1.1:9999") {
        Ok(..) => panic!(),
        Err(e) => assert_eq!(e.kind(), ErrorKind::AddrNotAvailable),
    }
}

#[test]
fn socket_smoke_test_ip4() {
    each_ip(&mut |server_ip, client_ip| {
        let mut client_pool = Pool::new();
        client_pool.insert_socket_addr(client_ip, ambient_authority());
        let mut server_pool = Pool::new();
        server_pool.insert_socket_addr(server_ip, ambient_authority());

        let (tx1, rx1) = channel();
        let (tx2, rx2) = channel();

        let p = server_pool.clone();
        let _t = thread::spawn(move || {
            let client = t!(client_pool.bind_udp_socket(&client_ip));
            rx1.recv().unwrap();
            t!(p.send_to_udp_socket_addr(&client, &[99], &server_ip));
            tx2.send(()).unwrap();
        });

        let server = t!(server_pool.bind_udp_socket(&server_ip));
        tx1.send(()).unwrap();
        let mut buf = [0];
        let (nread, src) = t!(server.recv_from(&mut buf));
        assert_eq!(nread, 1);
        assert_eq!(buf[0], 99);
        assert_eq!(src, client_ip);
        rx2.recv().unwrap();
    })
}

#[test]
fn socket_name() {
    each_ip(&mut |addr, _| {
        let mut pool = Pool::new();
        pool.insert_socket_addr(addr, ambient_authority());

        let server = t!(pool.bind_udp_socket(&addr));
        assert_eq!(addr, t!(server.local_addr()));
    })
}

#[test]
fn socket_peer() {
    each_ip(&mut |addr1, addr2| {
        let mut pool1 = Pool::new();
        pool1.insert_socket_addr(addr1, ambient_authority());
        let mut pool2 = Pool::new();
        pool2.insert_socket_addr(addr2, ambient_authority());

        let server = t!(pool1.bind_udp_socket(&addr1));
        assert_eq!(
            server.peer_addr().unwrap_err().kind(),
            ErrorKind::NotConnected
        );
        t!(pool2.connect_udp_socket(&server, &addr2));
        assert_eq!(addr2, t!(server.peer_addr()));
    })
}

#[test]
fn udp_clone_smoke() {
    each_ip(&mut |addr1, addr2| {
        let mut pool1 = Pool::new();
        pool1.insert_socket_addr(addr1, ambient_authority());
        let mut pool2 = Pool::new();
        pool2.insert_socket_addr(addr2, ambient_authority());

        let sock1 = t!(pool1.bind_udp_socket(&addr1));
        let sock2 = t!(pool2.bind_udp_socket(&addr2));

        let _t = thread::spawn(move || {
            let mut buf = [0, 0];
            assert_eq!(sock2.recv_from(&mut buf).unwrap(), (1, addr1));
            assert_eq!(buf[0], 1);
            t!(pool1.send_to_udp_socket_addr(&sock2, &[2], &addr1));
        });

        let sock3 = t!(sock1.try_clone());

        let (tx1, rx1) = channel();
        let (tx2, rx2) = channel();
        let p = pool2.clone();
        let _t = thread::spawn(move || {
            rx1.recv().unwrap();
            t!(p.send_to_udp_socket_addr(&sock3, &[1], &addr2));
            tx2.send(()).unwrap();
        });
        tx1.send(()).unwrap();
        let mut buf = [0, 0];
        assert_eq!(sock1.recv_from(&mut buf).unwrap(), (1, addr2));
        rx2.recv().unwrap();
    })
}

#[test]
fn udp_clone_two_read() {
    each_ip(&mut |addr1, addr2| {
        let mut pool1 = Pool::new();
        pool1.insert_socket_addr(addr1, ambient_authority());
        let mut pool2 = Pool::new();
        pool2.insert_socket_addr(addr2, ambient_authority());

        let sock1 = t!(pool1.bind_udp_socket(&addr1));
        let sock2 = t!(pool2.bind_udp_socket(&addr2));
        let (tx1, rx) = channel();
        let tx2 = tx1.clone();

        let _t = thread::spawn(move || {
            t!(pool1.send_to_udp_socket_addr(&sock2, &[1], &addr1));
            rx.recv().unwrap();
            t!(pool1.send_to_udp_socket_addr(&sock2, &[2], &addr1));
            rx.recv().unwrap();
        });

        let sock3 = t!(sock1.try_clone());

        let (done, rx) = channel();
        let _t = thread::spawn(move || {
            let mut buf = [0, 0];
            t!(sock3.recv_from(&mut buf));
            tx2.send(()).unwrap();
            done.send(()).unwrap();
        });
        let mut buf = [0, 0];
        t!(sock1.recv_from(&mut buf));
        tx1.send(()).unwrap();

        rx.recv().unwrap();
    })
}

#[test]
fn udp_clone_two_write() {
    each_ip(&mut |addr1, addr2| {
        let mut pool1 = Pool::new();
        pool1.insert_socket_addr(addr1, ambient_authority());
        let mut pool2 = Pool::new();
        pool2.insert_socket_addr(addr2, ambient_authority());

        let sock1 = t!(pool1.bind_udp_socket(&addr1));
        let sock2 = t!(pool2.bind_udp_socket(&addr2));

        let (tx, rx) = channel();
        let (serv_tx, serv_rx) = channel();

        let _t = thread::spawn(move || {
            let mut buf = [0, 1];
            rx.recv().unwrap();
            t!(sock2.recv_from(&mut buf));
            serv_tx.send(()).unwrap();
        });

        let sock3 = t!(sock1.try_clone());

        let (done, rx) = channel();
        let tx2 = tx.clone();
        let p = pool2.clone();
        let _t = thread::spawn(move || {
            if p.send_to_udp_socket_addr(&sock3, &[1], &addr2).is_ok() {
                let _ = tx2.send(());
            }
            done.send(()).unwrap();
        });
        if pool2.send_to_udp_socket_addr(&sock1, &[2], &addr2).is_ok() {
            let _ = tx.send(());
        }
        drop(tx);

        rx.recv().unwrap();
        serv_rx.recv().unwrap();
    })
}

/* Disable this test, as it depends on Rust-internal details.
#[test]
fn debug() {
    let name = if cfg!(windows) { "socket" } else { "fd" };
    let socket_addr = next_test_ip4();

    let mut pool = Pool::new();

    let udpsock_inner = udpsock.0.socket().as_inner();
    let compare = format!("UdpSocket {{ addr: {:?}, {}: {:?} }}", socket_addr, name, udpsock_inner);
    assert_eq!(format!("{:?}", udpsock), compare);
}
*/

// FIXME: re-enabled openbsd/netbsd tests once their socket timeout code
//        no longer has rounding errors.
// VxWorks ignores SO_SNDTIMEO.
#[cfg_attr(
    any(target_os = "netbsd", target_os = "openbsd", target_os = "vxworks"),
    ignore
)]
#[test]
fn timeouts() {
    let addr = next_test_ip4();

    let mut pool = Pool::new();
    pool.insert_socket_addr(addr, ambient_authority());

    let stream = t!(pool.bind_udp_socket(&addr));
    let dur = Duration::new(15410, 0);

    assert_eq!(None, t!(stream.read_timeout()));

    t!(stream.set_read_timeout(Some(dur)));
    assert_eq!(Some(dur), t!(stream.read_timeout()));

    assert_eq!(None, t!(stream.write_timeout()));

    t!(stream.set_write_timeout(Some(dur)));
    assert_eq!(Some(dur), t!(stream.write_timeout()));

    t!(stream.set_read_timeout(None));
    assert_eq!(None, t!(stream.read_timeout()));

    t!(stream.set_write_timeout(None));
    assert_eq!(None, t!(stream.write_timeout()));
}

#[test]
fn test_read_timeout() {
    let addr = next_test_ip4();

    let mut pool = Pool::new();
    pool.insert_socket_addr(addr, ambient_authority());

    let stream = t!(pool.bind_udp_socket(&addr));
    t!(stream.set_read_timeout(Some(Duration::from_millis(1000))));

    let mut buf = [0; 10];

    let start = Instant::now();
    loop {
        let kind = stream
            .recv_from(&mut buf)
            .err()
            .expect("expected error")
            .kind();
        if kind != ErrorKind::Interrupted {
            assert!(
                kind == ErrorKind::WouldBlock || kind == ErrorKind::TimedOut,
                "unexpected_error: {:?}",
                kind
            );
            break;
        }
    }
    assert!(start.elapsed() > Duration::from_millis(400));
}

#[test]
fn test_read_with_timeout() {
    let addr = next_test_ip4();

    let mut pool = Pool::new();
    pool.insert_socket_addr(addr, ambient_authority());

    let stream = t!(pool.bind_udp_socket(&addr));
    t!(stream.set_read_timeout(Some(Duration::from_millis(1000))));

    t!(pool.send_to_udp_socket_addr(&stream, b"hello world", &addr));

    let mut buf = [0; 11];
    t!(stream.recv_from(&mut buf));
    assert_eq!(b"hello world", &buf[..]);

    let start = Instant::now();
    loop {
        let kind = stream
            .recv_from(&mut buf)
            .err()
            .expect("expected error")
            .kind();
        if kind != ErrorKind::Interrupted {
            assert!(
                kind == ErrorKind::WouldBlock || kind == ErrorKind::TimedOut,
                "unexpected_error: {:?}",
                kind
            );
            break;
        }
    }
    assert!(start.elapsed() > Duration::from_millis(400));
}

// Ensure the `set_read_timeout` and `set_write_timeout` calls return errors
// when passed zero Durations
#[test]
fn test_timeout_zero_duration() {
    let addr = next_test_ip4();

    let mut pool = Pool::new();
    pool.insert_socket_addr(addr, ambient_authority());

    let socket = t!(pool.bind_udp_socket(&addr));

    let result = socket.set_write_timeout(Some(Duration::new(0, 0)));
    let err = result.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidInput);

    let result = socket.set_read_timeout(Some(Duration::new(0, 0)));
    let err = result.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
}

#[test]
fn connect_send_recv() {
    let addr = next_test_ip4();

    let mut pool = Pool::new();
    pool.insert_socket_addr(addr, ambient_authority());

    let socket = t!(pool.bind_udp_socket(&addr));
    t!(pool.connect_udp_socket(&socket, addr));

    t!(socket.send(b"hello world"));

    let mut buf = [0; 11];
    t!(socket.recv(&mut buf));
    assert_eq!(b"hello world", &buf[..]);
}

#[test]
fn connect_send_peek_recv() {
    each_ip(&mut |addr, _| {
        let mut pool = Pool::new();
        pool.insert_socket_addr(addr, ambient_authority());

        let socket = t!(pool.bind_udp_socket(&addr));
        t!(pool.connect_udp_socket(&socket, addr));

        t!(socket.send(b"hello world"));

        for _ in 1..3 {
            let mut buf = [0; 11];
            let size = t!(socket.peek(&mut buf));
            assert_eq!(b"hello world", &buf[..]);
            assert_eq!(size, 11);
        }

        let mut buf = [0; 11];
        let size = t!(socket.recv(&mut buf));
        assert_eq!(b"hello world", &buf[..]);
        assert_eq!(size, 11);
    })
}

#[test]
fn peek_from() {
    each_ip(&mut |addr, _| {
        let mut pool = Pool::new();
        pool.insert_socket_addr(addr, ambient_authority());

        let socket = t!(pool.bind_udp_socket(&addr));
        t!(pool.send_to_udp_socket_addr(&socket, b"hello world", &addr));

        for _ in 1..3 {
            let mut buf = [0; 11];
            let (size, _) = t!(socket.peek_from(&mut buf));
            assert_eq!(b"hello world", &buf[..]);
            assert_eq!(size, 11);
        }

        let mut buf = [0; 11];
        let (size, _) = t!(socket.recv_from(&mut buf));
        assert_eq!(b"hello world", &buf[..]);
        assert_eq!(size, 11);
    })
}

#[test]
fn ttl() {
    let ttl = 100;

    let addr = next_test_ip4();

    let mut pool = Pool::new();
    pool.insert_socket_addr(addr, ambient_authority());

    let stream = t!(pool.bind_udp_socket(&addr));

    t!(stream.set_ttl(ttl));
    assert_eq!(ttl, t!(stream.ttl()));
}

#[test]
fn set_nonblocking() {
    each_ip(&mut |addr, _| {
        let mut pool = Pool::new();
        pool.insert_socket_addr(addr, ambient_authority());

        let socket = t!(pool.bind_udp_socket(&addr));

        t!(socket.set_nonblocking(true));
        t!(socket.set_nonblocking(false));

        t!(pool.connect_udp_socket(&socket, addr));

        t!(socket.set_nonblocking(false));
        t!(socket.set_nonblocking(true));

        let mut buf = [0];
        match socket.recv(&mut buf) {
            Ok(_) => panic!("expected error"),
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {}
            Err(e) => panic!("unexpected error {}", e),
        }
    })
}
