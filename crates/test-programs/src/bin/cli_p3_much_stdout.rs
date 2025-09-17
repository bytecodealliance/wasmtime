use test_programs::p3::{wasi, wit_stream};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let mut args = std::env::args().skip(1);
        let string_to_write = args.next().unwrap();
        let times_to_write = args.next().unwrap().parse::<u32>().unwrap();

        let bytes = string_to_write.as_bytes();
        let (mut tx, rx) = wit_stream::new();
        futures::join!(
            async { wasi::cli::stdout::write_via_stream(rx).await.unwrap() },
            async {
                for _ in 0..times_to_write {
                    let result = tx.write_all(bytes.to_vec()).await;
                    assert!(result.is_empty());
                }
                drop(tx);
            }
        );
        Ok(())
    }
}

fn main() {
    unreachable!();
}
