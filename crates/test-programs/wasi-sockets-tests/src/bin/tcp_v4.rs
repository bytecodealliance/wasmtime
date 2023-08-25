//! A simple TCP testcase, using IPv4.

use core::cmp::min;
use wasi::io::streams::{self, FlushResult, StreamStatus, WriteReadiness};
use wasi::poll::poll;
use wasi::sockets::network::{IpAddressFamily, IpSocketAddress, Ipv4SocketAddress};
use wasi::sockets::{instance_network, network, tcp, tcp_create_socket};
use wasi_sockets_tests::*;

fn wait(sub: poll::Pollable) {
    loop {
        let wait = poll::poll_oneoff(&[sub]);
        if wait[0] {
            break;
        }
    }
}

fn write(output: streams::OutputStream, bytes: &[u8]) -> (usize, StreamStatus) {
    let sub = streams::subscribe_to_write_ready(output);
    let num_wanted = loop {
        wait(sub);
        match streams::check_write(output) {
            Some(WriteReadiness::Ready(num_wanted)) => break num_wanted,
            Some(WriteReadiness::Closed) => return (0, StreamStatus::Ended),
            None => (),
        }
    };
    poll::drop_pollable(sub);

    let num_to_write = min(num_wanted, bytes.len() as u64) as usize;
    let bytes = &bytes[..num_to_write];

    let mut num_written = 0;

    while num_to_write != 0 {
        let slice_to_write = &bytes[num_written..];
        let num_written_this_iteration = match streams::write(output, slice_to_write) {
            Some(WriteReadiness::Ready(num_written)) => num_written as usize,
            Some(WriteReadiness::Closed) => return (num_written, StreamStatus::Ended),
            None => {
                match streams::flush(output) {
                    Some(FlushResult::Done) => (),
                    Some(FlushResult::Closed) => return (num_written, StreamStatus::Ended),
                    None => {
                        let sub = streams::subscribe_to_flush(output);
                        loop {
                            wait(sub);
                            match streams::check_flush(output) {
                                Some(FlushResult::Done) => break,
                                Some(FlushResult::Closed) => {
                                    return (num_written, StreamStatus::Ended)
                                }
                                None => (),
                            }
                        }
                        poll::drop_pollable(sub);
                    }
                }
                slice_to_write.len()
            }
        };
        num_written += num_written_this_iteration;
    }

    (num_written, StreamStatus::Open)
}

fn main() {
    let first_message = b"Hello, world!";
    let second_message = b"Greetings, planet!";

    let net = instance_network::instance_network();

    let sock = tcp_create_socket::create_tcp_socket(IpAddressFamily::Ipv4).unwrap();

    let addr = IpSocketAddress::Ipv4(Ipv4SocketAddress {
        port: 0,                 // use any free port
        address: (127, 0, 0, 1), // localhost
    });

    let sub = tcp::subscribe(sock);

    tcp::start_bind(sock, net, addr).unwrap();
    wait(sub);
    tcp::finish_bind(sock).unwrap();

    tcp::start_listen(sock).unwrap();
    wait(sub);
    tcp::finish_listen(sock).unwrap();

    let addr = tcp::local_address(sock).unwrap();

    let client = tcp_create_socket::create_tcp_socket(IpAddressFamily::Ipv4).unwrap();
    let client_sub = tcp::subscribe(client);

    tcp::start_connect(client, net, addr).unwrap();
    wait(client_sub);
    let (client_input, client_output) = tcp::finish_connect(client).unwrap();

    let (n, status) = write(client_output, &[]);
    assert_eq!(n, 0);
    assert_eq!(status, streams::StreamStatus::Open);

    let (n, status) = write(client_output, first_message);
    assert_eq!(n, first_message.len());
    assert_eq!(status, streams::StreamStatus::Open);

    streams::drop_input_stream(client_input);
    streams::drop_output_stream(client_output);
    poll::drop_pollable(client_sub);
    tcp::drop_tcp_socket(client);

    wait(sub);
    let (accepted, input, output) = tcp::accept(sock).unwrap();

    let (empty_data, status) = streams::read(input, 0).unwrap();
    assert!(empty_data.is_empty());
    assert_eq!(status, streams::StreamStatus::Open);

    let (data, status) = streams::read(input, first_message.len() as u64).unwrap();
    assert_eq!(status, streams::StreamStatus::Open);

    tcp::drop_tcp_socket(accepted);
    streams::drop_input_stream(input);
    streams::drop_output_stream(output);

    // Check that we sent and recieved our message!
    assert_eq!(data, first_message); // Not guaranteed to work but should work in practice.

    // Another client
    let client = tcp_create_socket::create_tcp_socket(IpAddressFamily::Ipv4).unwrap();
    let client_sub = tcp::subscribe(client);

    tcp::start_connect(client, net, addr).unwrap();
    wait(client_sub);
    let (client_input, client_output) = tcp::finish_connect(client).unwrap();

    let (n, status) = write(client_output, second_message);
    assert_eq!(n, second_message.len());
    assert_eq!(status, streams::StreamStatus::Open);

    streams::drop_input_stream(client_input);
    streams::drop_output_stream(client_output);
    poll::drop_pollable(client_sub);
    tcp::drop_tcp_socket(client);

    wait(sub);
    let (accepted, input, output) = tcp::accept(sock).unwrap();
    let (data, status) = streams::read(input, second_message.len() as u64).unwrap();
    assert_eq!(status, streams::StreamStatus::Open);

    streams::drop_input_stream(input);
    streams::drop_output_stream(output);
    tcp::drop_tcp_socket(accepted);

    // Check that we sent and recieved our message!
    assert_eq!(data, second_message); // Not guaranteed to work but should work in practice.

    poll::drop_pollable(sub);
    tcp::drop_tcp_socket(sock);
    network::drop_network(net);
}
