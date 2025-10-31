use test_programs::wasi::cli::stdin;
use test_programs::wasi::io::streams;

fn main() {
    let stdin: streams::InputStream = stdin::get_stdin();
    let stdin_pollable = stdin.subscribe();
    stdin_pollable.block();
    drop(stdin);
    unreachable!("execution should have trapped in line above when stream dropped before pollable");
}
