use test_programs::p3::wasi;

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let (mut stream, result) = wasi::cli::stdin::read_via_stream();
        let (sresult, buf) = stream.read(Vec::with_capacity(100)).await;
        assert_eq!(buf, b"hello!".to_vec());
        assert_eq!(sresult, wit_bindgen::StreamResult::Complete(6));

        let (sresult, buf) = stream.read(Vec::with_capacity(100)).await;
        assert!(buf.is_empty());
        assert_eq!(sresult, wit_bindgen::StreamResult::Dropped);

        result.await.unwrap();
        Ok(())
    }
}

fn main() {
    unreachable!();
}
