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
    let stdin_pollable = stdin.subscribe();
    stdin_pollable.block();
    assert!(stdin_pollable.ready());
    let bytes = stdin.read(CHUNK.len() as u64).unwrap();
    assert_eq!(&bytes, CHUNK);
}
