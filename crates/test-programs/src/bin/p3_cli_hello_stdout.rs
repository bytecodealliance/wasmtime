use test_programs::p3::*;

struct Component;

export!(Component);

impl exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let (mut tx, rx) = wit_stream::new();
        futures::join!(
            async {
                wasi::cli::stdout::write_via_stream(rx).await.unwrap();
            },
            async {
                tx.write(b"hello, world\n".to_vec()).await;
                drop(tx);
            },
        );
        Ok(())
    }
}

fn main() {
    panic!("should call p3 entrypoint");
}
