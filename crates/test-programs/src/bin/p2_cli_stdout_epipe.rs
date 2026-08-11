//! Guest program that writes to stdout until the pipe is closed, then verifies
//! it gets `StreamError::Closed` (which maps to EPIPE) rather than a trap or
//! generic EIO.

use test_programs::wasi::cli::stdout;
use test_programs::wasi::io::streams::StreamError;

fn main() {
    let stdout = stdout::get_stdout();
    let chunk = vec![b'x'; 4096];

    loop {
        match stdout.blocking_write_and_flush(&chunk) {
            Ok(()) => continue,
            Err(StreamError::Closed) => {
                // This is the expected outcome: the pipe was closed by the
                // reader, and wasmtime correctly reports it as Closed (EPIPE).
                eprintln!("got expected StreamError::Closed");
                return;
            }
            Err(StreamError::LastOperationFailed(err)) => {
                panic!(
                    "unexpected LastOperationFailed (should have been Closed): {}",
                    err.to_debug_string()
                );
            }
        }
    }
}
