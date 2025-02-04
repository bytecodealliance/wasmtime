mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "round-trip-direct",
        async: true,
    });

    use super::Component;
    export!(Component);
}

struct Component;

impl bindings::Guest for Component {
    async fn foo(s: String) -> String {
        format!(
            "{} - exited guest",
            bindings::foo(&format!("{s} - entered guest")).await
        )
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
