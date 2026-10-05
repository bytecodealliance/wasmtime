use test_programs::p3::{wasi, wit_stream};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let size = std::env::args().nth(2).unwrap().parse::<usize>().unwrap();

        let (mut stdout_tx, stdout_rx) = wit_stream::new();
        let (mut stderr_tx, stderr_rx) = wit_stream::new();
        futures::join!(
            async {
                wasi::cli::stdout::write_via_stream(stdout_rx)
                    .await
                    .unwrap()
            },
            async {
                wasi::cli::stderr::write_via_stream(stderr_rx)
                    .await
                    .unwrap()
            },
            async {
                let rest = stdout_tx.write_all(vec![0; size]).await;
                assert!(rest.is_empty());
                drop(stdout_tx);
            },
            async {
                let rest = stderr_tx.write_all(vec![1; size]).await;
                assert!(rest.is_empty());
                drop(stderr_tx);
            },
        );
        Ok(())
    }
}

fn main() {
    unreachable!();
}
