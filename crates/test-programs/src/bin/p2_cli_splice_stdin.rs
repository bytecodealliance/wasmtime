use test_programs::wasi::cli::{stdin, stdout};
use test_programs::wasi::io::streams::StreamError;

fn main() {
    println!("before splice");
    let stdout = stdout::get_stdout();
    let stdin = stdin::get_stdin();

    let mut spliced = 0;
    loop {
        match stdout.blocking_splice(&stdin, 4096) {
            Ok(n) => spliced += n as usize,
            Err(StreamError::Closed) => break,
            Err(StreamError::LastOperationFailed(f)) => {
                panic!("stream failure: {}", f.to_debug_string())
            }
        }
    }
    let _ = stdin;
    stdout.blocking_flush().unwrap();
    let _ = stdout;

    println!("\ncompleted splicing {spliced} bytes");
}
