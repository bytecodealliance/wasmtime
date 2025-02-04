mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "error-context-caller",
        async: {
            imports: [
                "local:local/run-result#run-fail",
            ],
            exports: [
                "local:local/run#run",
            ],
        }
    });

    use super::Component;
    export!(Component);
}
use bindings::exports::local::local::run::Guest;

struct Component;

impl Guest for Component {
    async fn run() {
        let Err(err_ctx) = bindings::local::local::run_result::run_fail().await else {
            panic!("callee failure run should have produced an error");
        };
        _ = err_ctx.debug_message();
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
