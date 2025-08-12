use test_programs::p3::*;

struct Component;

export!(Component);

impl exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let (mut tx, mut rx) = wit_stream::new();
        wasi::cli::stdout::set_stdout(rx);
        tx.write(b"hello, world\n".to_vec()).await;
        Ok(())
    }
}

fn main() {
    panic!("should call p3 entrypoint");
}
