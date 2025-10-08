use test_programs::wasi::cli::stdin;

fn main() {
    let stdin = stdin::get_stdin();
    let p1 = stdin.subscribe();
    let p2 = stdin.subscribe();

    // Should work:
    // - Exactly the same pollable passed in multiple times.
    // - Distinct pollables for the same backing resource (stdin in this case).
    test_programs::wasi::io::poll::poll(&[&p1, &p2, &p1, &p2]);
}
