mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "yield-callee",
        async: ["-local:local/run#run"],
    });

    use super::Component;
    export!(Component);
}

use bindings::{exports::local::local::run::Guest, local::local::continue_};

#[cfg(not(target_arch = "wasm32"))]
unsafe fn yield_cancellable() -> bool {
    unreachable!();
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "$root")]
unsafe extern "C" {
    #[link_name = "[cancellable][thread-yield]"]
    fn yield_cancellable() -> bool;
}

struct Component;

impl Guest for Component {
    fn run() {
        while continue_::get_continue() && unsafe { !yield_cancellable() } {}
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
