wit_bindgen::generate!("test-command-with-sockets" in "../../wasi/wit");

use wasi::io::poll;
use wasi::io::streams;
use wasi::sockets::{network, tcp, tcp_create_socket};

pub fn write(output: &streams::OutputStream, mut bytes: &[u8]) -> Result<(), streams::StreamError> {
    let pollable = output.subscribe();

    while !bytes.is_empty() {
        poll::poll_list(&[&pollable]);

        let permit = output.check_write()?;

        let len = bytes.len().min(permit as usize);
        let (chunk, rest) = bytes.split_at(len);

        output.write(chunk)?;

        output.blocking_flush()?;

        bytes = rest;
    }
    Ok(())
}

pub fn example_body(net: tcp::Network, sock: tcp::TcpSocket, family: network::IpAddressFamily) {
    let first_message = b"Hello, world!";
    let second_message = b"Greetings, planet!";

    let sub = sock.subscribe();

    sock.set_listen_backlog_size(32).unwrap();

    sock.start_listen().unwrap();
    poll::poll_one(&sub);
    sock.finish_listen().unwrap();

    let addr = sock.local_address().unwrap();

    let client = tcp_create_socket::create_tcp_socket(family).unwrap();
    let client_sub = client.subscribe();

    client.start_connect(&net, addr).unwrap();
    poll::poll_one(&client_sub);
    let (client_input, client_output) = client.finish_connect().unwrap();

    write(&client_output, &[]).unwrap();

    write(&client_output, first_message).unwrap();

    drop(client_input);
    drop(client_output);
    drop(client_sub);
    drop(client);

    poll::poll_one(&sub);
    let (accepted, input, output) = sock.accept().unwrap();

    let empty_data = input.read(0).unwrap();
    assert!(empty_data.is_empty());

    let data = input.blocking_read(first_message.len() as u64).unwrap();

    drop(input);
    drop(output);
    drop(accepted);

    // Check that we sent and recieved our message!
    assert_eq!(data, first_message); // Not guaranteed to work but should work in practice.

    // Another client
    let client = tcp_create_socket::create_tcp_socket(family).unwrap();
    let client_sub = client.subscribe();

    client.start_connect(&net, addr).unwrap();
    poll::poll_one(&client_sub);
    let (client_input, client_output) = client.finish_connect().unwrap();

    write(&client_output, second_message).unwrap();

    drop(client_input);
    drop(client_output);
    drop(client_sub);
    drop(client);

    poll::poll_one(&sub);
    let (accepted, input, output) = sock.accept().unwrap();
    let data = input.blocking_read(second_message.len() as u64).unwrap();

    drop(input);
    drop(output);
    drop(accepted);

    // Check that we sent and recieved our message!
    assert_eq!(data, second_message); // Not guaranteed to work but should work in practice.
}
