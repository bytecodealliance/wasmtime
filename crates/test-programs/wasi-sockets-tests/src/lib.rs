wit_bindgen::generate!("test-command-with-sockets" in "../../wasi/wit");

use wasi::io::poll;
use wasi::io::streams;
use wasi::sockets::{network, tcp, tcp_create_socket};

pub fn write(output: &streams::OutputStream, mut bytes: &[u8]) -> (usize, streams::StreamStatus) {
    let total = bytes.len();
    let mut written = 0;

    let pollable = output.subscribe();

    while !bytes.is_empty() {
        crate::poll_list(&[&pollable]);

        let permit = match output.check_write() {
            Ok(n) => n,
            Err(_) => return (written, streams::StreamStatus::Ended),
        };

        let len = bytes.len().min(permit as usize);
        let (chunk, rest) = bytes.split_at(len);

        match output.write(chunk) {
            Ok(()) => {}
            Err(_) => return (written, streams::StreamStatus::Ended),
        }

        match output.blocking_flush() {
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

    let sub = sock.subscribe();

    sock.start_listen().unwrap();
    poll::poll_one(&sub);
    sock.finish_listen().unwrap();

    let addr = sock.local_address().unwrap();

    let client = tcp_create_socket::create_tcp_socket(family).unwrap();
    let client_sub = client.subscribe();

    client.start_connect(&net, addr).unwrap();
    poll::poll_one(&client_sub);
    let (client_input, client_output) = client.finish_connect().unwrap();

    let (n, status) = write(&client_output, &[]);
    assert_eq!(n, 0);
    assert_eq!(status, streams::StreamStatus::Open);

    let (n, status) = write(&client_output, first_message);
    assert_eq!(n, first_message.len());
    assert_eq!(status, streams::StreamStatus::Open);

    drop(client_input);
    drop(client_output);
    drop(client_sub);
    drop(client);

    poll::poll_one(&sub);
    let (accepted, input, output) = sock.accept().unwrap();

    let (empty_data, status) = input.read(0).unwrap();
    assert!(empty_data.is_empty());
    assert_eq!(status, streams::StreamStatus::Open);

    let (data, status) = input.blocking_read(first_message.len() as u64).unwrap();
    assert_eq!(status, streams::StreamStatus::Open);

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

    let (n, status) = write(&client_output, second_message);
    assert_eq!(n, second_message.len());
    assert_eq!(status, streams::StreamStatus::Open);

    drop(client_input);
    drop(client_output);
    drop(client_sub);
    drop(client);

    poll::poll_one(&sub);
    let (accepted, input, output) = sock.accept().unwrap();
    let (data, status) = input.blocking_read(second_message.len() as u64).unwrap();
    assert_eq!(status, streams::StreamStatus::Open);

    drop(input);
    drop(output);
    drop(accepted);

    // Check that we sent and recieved our message!
    assert_eq!(data, second_message); // Not guaranteed to work but should work in practice.
}

/// FIXME: This is a copy of the `poll_list` bindings generated with a
/// wit-bindgen with this fix:
/// <https://github.com/bytecodealliance/wit-bindgen/pull/670>
///
/// One that PR is in a published release, delete this code and use the
/// bindings.
///
/// Poll for completion on a set of pollables.
///
/// This function takes a list of pollables, which identify I/O sources of
/// interest, and waits until one or more of the events is ready for I/O.
///
/// The result `list<u32>` contains one or more indices of handles in the
/// argument list that is ready for I/O.
///
/// If the list contains more elements than can be indexed with a `u32`
/// value, this function traps.
///
/// A timeout can be implemented by adding a pollable from the
/// wasi-clocks API to the list.
///
/// This function does not return a `result`; polling in itself does not
/// do any I/O so it doesn't fail. If any of the I/O sources identified by
/// the pollables has an error, it is indicated by marking the source as
/// being reaedy for I/O.
pub fn poll_list(in_: &[&poll::Pollable]) -> wit_bindgen::rt::vec::Vec<u32> {
    #[allow(unused_imports)]
    use wit_bindgen::rt::{alloc, string::String, vec::Vec};
    unsafe {
        #[repr(align(4))]
        struct RetArea([u8; 8]);
        let mut ret_area = ::core::mem::MaybeUninit::<RetArea>::uninit();
        let vec0 = in_;
        let len0 = vec0.len() as i32;
        let layout0 = alloc::Layout::from_size_align_unchecked(vec0.len() * 4, 4);
        let result0 = if layout0.size() != 0 {
            let ptr = alloc::alloc(layout0);
            if ptr.is_null() {
                alloc::handle_alloc_error(layout0);
            }
            ptr
        } else {
            ::core::ptr::null_mut()
        };
        for (i, e) in vec0.into_iter().enumerate() {
            let base = result0 as i32 + (i as i32) * 4;
            {
                *((base + 0) as *mut i32) = (e).handle() as i32;
            }
        }
        let ptr1 = ret_area.as_mut_ptr() as i32;
        #[cfg(target_arch = "wasm32")]
        #[link(wasm_import_module = "wasi:io/poll")]
        extern "C" {
            #[link_name = "poll-list"]
            fn wit_import(_: i32, _: i32, _: i32);
        }

        #[cfg(not(target_arch = "wasm32"))]
        fn wit_import(_: i32, _: i32, _: i32) {
            unreachable!()
        }
        wit_import(result0 as i32, len0, ptr1);
        let l2 = *((ptr1 + 0) as *const i32);
        let l3 = *((ptr1 + 4) as *const i32);
        let len4 = l3 as usize;
        if layout0.size() != 0 {
            alloc::dealloc(result0, layout0);
        }
        Vec::from_raw_parts(l2 as *mut _, len4, len4)
    }
}
