mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "error-context-usage",
    });

    use super::Component;
    export!(Component);
}
use bindings::exports::local::local::run::Guest;

use wit_bindgen_rt::async_support::ErrorContext;

struct Component;

impl Guest for Component {
    async fn run() {
        let err_ctx = ErrorContext::new("error");
        _ = err_ctx.debug_message();
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
