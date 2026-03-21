use test_programs::p3::wasi::random;

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        test_big_random_buf().await;
        Ok(())
    }
}

fn main() {
    unreachable!()
}

async fn test_big_random_buf() {
    let buf = random::random::get_random_bytes(1024);
    // Chances are pretty good that at least *one* byte will be non-zero in
    // any meaningful random function producing 1024 u8 values.
    assert!(buf.iter().any(|x| *x != 0), "random_get returned all zeros");
}
