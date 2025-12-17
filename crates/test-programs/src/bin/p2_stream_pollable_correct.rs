use test_programs::wasi::cli::{stdin, stdout};

fn main() {
    let stdin = stdin::get_stdin();
    let stdin_pollable = stdin.subscribe();
    stdin_pollable.block();
    assert!(stdin_pollable.ready(), "after blocking, pollable is ready");
    drop(stdin_pollable);
    drop(stdin);

    // Pollables can be used many times over their lifetime
    let stdout = stdout::get_stdout();
    let stdout_pollable = stdout.subscribe();

    let chunk = [b'a'; 50];
    for _ in 1..10 {
        stdout_pollable.block();
        assert!(stdout_pollable.ready(), "after blocking, pollable is ready");

        let n = stdout.check_write().unwrap() as usize;
        assert!(n >= chunk.len());
        stdout.write(&chunk).unwrap();
    }

    drop(stdout_pollable);
    drop(stdout);
}
