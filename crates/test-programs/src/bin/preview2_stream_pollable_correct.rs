use test_programs::wasi::cli::stdin;
use test_programs::wasi::io::streams;

fn main() {
    let stdin: streams::InputStream = stdin::get_stdin();
    let stdin_pollable = stdin.subscribe();
    stdin_pollable.block();
    assert!(stdin_pollable.ready(), "after blocking, pollable is ready");
    drop(stdin_pollable);
    drop(stdin);
}
