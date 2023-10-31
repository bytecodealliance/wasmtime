wit_bindgen::generate!("test-command-with-sockets" in "../../wasi/wit");

use wasi::io::streams;
use wasi::poll::poll;
use wasi::sockets::{network, tcp, tcp_create_socket};

pub fn wait(sub: poll::Pollable) {
    loop {
        let wait = poll::poll_oneoff(&[sub]);
        if wait[0] {
            break;
        }
    }
}

pub struct DropPollable {
    pub pollable: poll::Pollable,
}

impl Drop for DropPollable {
    fn drop(&mut self) {
        poll::drop_pollable(self.pollable);
    }
}

pub fn write(output: streams::OutputStream, mut bytes: &[u8]) -> (usize, streams::StreamStatus) {
    let total = bytes.len();
    let mut written = 0;

    let s = DropPollable {
        pollable: streams::subscribe_to_output_stream(output),
    };

    while !bytes.is_empty() {
        poll::poll_oneoff(&[s.pollable]);

        let permit = match streams::check_write(output) {
            Ok(n) => n,
            Err(_) => return (written, streams::StreamStatus::Ended),
        };

        let len = bytes.len().min(permit as usize);
        let (chunk, rest) = bytes.split_at(len);

        match streams::write(output, chunk) {
            Ok(()) => {}
            Err(_) => return (written, streams::StreamStatus::Ended),
        }

        match streams::blocking_flush(output) {
            Ok(()) => {}
            Err(_) => return (written, streams::StreamStatus::Ended),
        }

        bytes = rest;
        written += len;
    }

    (total, streams::StreamStatus::Open)
}

pub fn example_body(net: tcp::Network, sock: tcp::TcpSocket, family: network::IpAddressFamily) {
    let first_message = b"Hello, world!";
    let second_message = b"Greetings, planet!";

    let sub = tcp::subscribe(sock);

    tcp::start_listen(sock).unwrap();
    wait(sub);
    tcp::finish_listen(sock).unwrap();

    let addr = tcp::local_address(sock).unwrap();

    let client = tcp_create_socket::create_tcp_socket(family).unwrap();
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

    let (data, status) = streams::blocking_read(input, first_message.len() as u64).unwrap();
    assert_eq!(status, streams::StreamStatus::Open);

    streams::drop_input_stream(input);
    streams::drop_output_stream(output);
    tcp::drop_tcp_socket(accepted);

    // Check that we sent and recieved our message!
    assert_eq!(data, first_message); // Not guaranteed to work but should work in practice.

    // Another client
    let client = tcp_create_socket::create_tcp_socket(family).unwrap();
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
    let (data, status) = streams::blocking_read(input, second_message.len() as u64).unwrap();
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
