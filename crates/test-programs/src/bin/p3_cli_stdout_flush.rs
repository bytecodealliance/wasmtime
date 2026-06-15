//! Writes a prompt without a newline, then waits for stdin.

use test_programs::p3::{wasi, wit_stream};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let (mut tx, rx) = wit_stream::new();
        futures::join!(
            async { wasi::cli::stdout::write_via_stream(rx).await.unwrap() },
            async {
                assert!(tx.write_all(b"READY".to_vec()).await.is_empty());

                let (mut stdin, _result) = wasi::cli::stdin::read_via_stream();
                let _ = stdin.read(Vec::with_capacity(1)).await;

                drop(tx);
            },
        );
        Ok(())
    }
}

fn main() {
    unreachable!();
}
