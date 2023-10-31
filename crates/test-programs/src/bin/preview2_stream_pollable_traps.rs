use test_programs::wasi::cli::stdin;
use test_programs::wasi::io::poll;
use test_programs::wasi::io::streams;

fn main() {
    let stdin: streams::InputStream = stdin::get_stdin();
    let stdin_pollable = stdin.subscribe();
    let ready = poll::poll_list(&[&stdin_pollable]);
    assert_eq!(ready, &[0]);
    drop(stdin);
    unreachable!("execution should have trapped in line above when stream dropped before pollable");
}
