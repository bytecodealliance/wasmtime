mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "closed-stream-guest",
    });

    use super::Component;
    export!(Component);
}

use {bindings::exports::local::local::closed_stream::Guest, wit_bindgen::StreamReader};

struct Component;

impl Guest for Component {
    fn get() -> StreamReader<u8> {
        bindings::wit_stream::new().1
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
