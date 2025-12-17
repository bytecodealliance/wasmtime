use {
    bindings::{
        exports::local::local::short_reads::{self, Guest, GuestThing},
        wit_stream,
    },
    wit_bindgen::{StreamReader, StreamResult, rt::async_support},
};

mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "short-reads-guest",
    });

    use super::Component;
    export!(Component);
}

struct Thing {
    value: String,
}

impl GuestThing for Thing {
    fn new(value: String) -> Self {
        Self { value }
    }

    async fn get(&self) -> String {
        self.value.clone()
    }
}

struct Component;

impl Guest for Component {
    type Thing = Thing;

    async fn short_reads(
        mut stream: StreamReader<short_reads::Thing>,
    ) -> StreamReader<short_reads::Thing> {
        let (mut tx, rx) = wit_stream::new();

        async_support::spawn(async move {
            // Read the things one at a time, forcing the host to re-take
            // ownership of any unwritten items between writes.
            let mut things = Vec::new();
            loop {
                let (status, buffer) = stream.read(Vec::with_capacity(1)).await;
                match status {
                    StreamResult::Complete(_) => {
                        things.extend(buffer);
                    }
                    StreamResult::Dropped => break,
                    StreamResult::Cancelled => unreachable!(),
                }
            }
            // Write the things all at once.  The host will read them only one
            // at a time, forcing us to re-take ownership of any unwritten
            // items between writes.
            things = tx.write_all(things).await;
            assert!(things.is_empty());
        });

        rx
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
