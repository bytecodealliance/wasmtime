use test_programs::p3::{wasi as wasip3, wit_stream};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        for _ in 0..200 {
            let (tx, rx) = wit_stream::new();
            let _future = wasip3::cli::stdout::write_via_stream(rx);
            drop(tx);
        }
        Ok(())
    }
}

fn main() {
    unreachable!();
}
