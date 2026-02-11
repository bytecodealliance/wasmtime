use test_programs::p3::*;

struct Component;

export!(Component);

impl exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let (mut tx, rx) = wit_stream::new();
        wit_bindgen::spawn(async move {
            wasi::cli::stdout::write_via_stream(rx).await.unwrap();
        });
        tx.write_all(b"hello, world\n".to_vec()).await;
        wit_bindgen::spawn(async move {
            // Yield a few times to allow the host to accept and process
            // the `run` result.
            for _ in 0..10 {
                wit_bindgen::yield_async().await;
            }
            tx.write_all(b"hello again, after return\n".to_vec()).await;
            drop(tx);
        });
        Ok(())
    }
}

fn main() {
    panic!("should call p3 entrypoint");
}
