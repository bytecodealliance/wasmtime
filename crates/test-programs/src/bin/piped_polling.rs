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

fn producer() {
    let out = stdout::get_stdout();
    let out_pollable = out.subscribe();

    for i in 1..100 {
        let message = format!("{i}");
        loop {
            let available = out.check_write().unwrap() as usize;
            if available >= message.len() {
                break;
            }

            out_pollable.block();
            assert!(out_pollable.ready());
        }

        out.write(message.as_bytes()).unwrap()
    }

    drop(out_pollable);
}

fn consumer() {
    let stdin = stdin::get_stdin();
    let stdin_pollable = stdin.subscribe();

    for i in 1..100 {
        let expected = format!("{i}");

        stdin_pollable.block();
        assert!(stdin_pollable.ready());

        let bytes = stdin.read(expected.len() as u64).unwrap();
        assert_eq!(&bytes, expected.as_bytes());
    }

    drop(stdin_pollable);
}
