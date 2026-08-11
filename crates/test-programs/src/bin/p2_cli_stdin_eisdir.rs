//! Guest program that reads from stdin expecting an `IsADirectory` error.
//!
//! When stdin is redirected from a directory (e.g. `< /some/dir`), the host
//! should surface the error as `StreamError::LastOperationFailed` with the
//! original `io::Error` preserved, recoverable via `filesystem-error-code`.

use test_programs::wasi::cli::stdin;
use test_programs::wasi::filesystem::types::{self as filesystem, ErrorCode};
use test_programs::wasi::io::streams::StreamError;

fn main() {
    let stdin = stdin::get_stdin();

    // Keep polling until data or an error is available.
    loop {
        stdin.subscribe().block();
        match stdin.read(1024) {
            Ok(bytes) if bytes.is_empty() => continue,
            Ok(_) => panic!("expected an error reading from a directory, got data"),
            Err(StreamError::Closed) => {
                panic!("expected LastOperationFailed(IsDirectory), got Closed")
            }
            Err(StreamError::LastOperationFailed(err)) => {
                // Use filesystem-error-code to recover the specific error.
                let code = filesystem::filesystem_error_code(&err);
                assert_eq!(
                    code,
                    Some(ErrorCode::IsDirectory),
                    "expected IsDirectory, got {code:?}"
                );
                eprintln!("got expected ErrorCode::IsDirectory");
                return;
            }
        }
    }
}
