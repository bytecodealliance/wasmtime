mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "unit-stream-callee",
    });

    use super::Component;
    export!(Component);
}

use {
    bindings::{exports::local::local::unit_stream::Guest, wit_stream},
    wit_bindgen_rt::async_support::{self, StreamReader},
};

struct Component;

impl Guest for Component {
    async fn run(count: u32) -> StreamReader<()> {
        let (mut tx, rx) = wit_stream::new();

        async_support::spawn(async move {
            let mut sent = 0;
            let mut chunk_size = 1;
            while sent < count {
                let n = (count - sent).min(chunk_size);
                let remaining = tx.write_all(vec![(); usize::try_from(n).unwrap()]).await;
                assert!(remaining.is_empty());
                sent += n;
                chunk_size *= 2;
            }
        });

        rx
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
