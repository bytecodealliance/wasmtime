wit_bindgen::generate!("test-command-with-sockets" in "../../wasi/wit");

use wasi::io::streams;
use wasi::poll::poll;

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
