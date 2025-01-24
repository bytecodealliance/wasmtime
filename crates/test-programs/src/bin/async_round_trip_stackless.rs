mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "round-trip",
        async: true,
    });

    use super::Component;
    export!(Component);
}

use bindings::{exports::local::local::baz::Guest as Baz, local::local::baz};

struct Component;

impl Baz for Component {
    async fn foo(s: String) -> String {
        format!(
            "{} - exited guest",
            baz::foo(&format!("{s} - entered guest")).await
        )
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
