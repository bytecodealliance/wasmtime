mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "post-return-caller",
        async: {
            imports: [
                "local:local/post-return#foo"
            ],
            exports: [
                "local:local/run#run"
            ]
        }
    });

    use super::Component;
    export!(Component);
}

use bindings::{
    exports::local::local::run::Guest,
    local::local::post_return::{foo, get_post_return_value},
};

struct Component;

impl Guest for Component {
    async fn run() {
        let s = "All mimsy were the borogoves";
        assert_eq!(s, &foo(s).await);
        assert_eq!(s, &get_post_return_value());
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
