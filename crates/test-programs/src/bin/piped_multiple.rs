use test_programs::wasi::cli::{stdin, stdout};

fn main() {
    match std::env::var("PIPED_SIDE")
        .expect("piped tests require the PIPED_SIDE env var")
        .as_str()
    {
        "PRODUCER" => producer(),
        "CONSUMER" => consumer(),
        side => panic!("unknown piped test side: {side}"),
    }
}

const CHUNK: &[u8] = &[b'a'; 50];

fn producer() {
    let out = stdout::get_stdout();
    let n = out.check_write().unwrap() as usize;
    assert!(n > CHUNK.len());
    out.write(CHUNK).unwrap();
}

fn consumer() {
    let stdin = stdin::get_stdin();
    let stdin_pollable1 = stdin.subscribe();
    let stdin_pollable2 = stdin.subscribe();

    // The two pollables are subscribed to the same resource, and must report the same readiness
    stdin_pollable1.block();
    assert!(stdin_pollable1.ready() && stdin_pollable2.ready());

    let bytes = stdin.read(CHUNK.len() as u64).unwrap();
    assert_eq!(&bytes, CHUNK);
}
