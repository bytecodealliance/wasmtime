mod bindings {
    wit_bindgen::generate!({
        path: "../wasi-http/src/p3/wit",
        world: "local:local/middleware-with-chain",
        inline: "
package local:local;

world middleware-with-chain {
  include wasi:http/proxy@0.3.0-draft;

  import chain-http;
}

interface chain-http {
  use wasi:http/types@0.3.0-draft.{request, response, error-code};

  handle: async func(request: request) -> result<response, error-code>;
}
        ",
        generate_all,
    });

    use super::Component;
    export!(Component);
}

use bindings::{
    exports::wasi::http::handler::Guest as Handler,
    local::local::chain_http,
    wasi::clocks::monotonic_clock,
    wasi::http::types::{ErrorCode, Request, Response},
};
use std::time::Duration;

struct Component;

impl Handler for Component {
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        // First, sleep briefly.  This will ensure the next call happens via a
        // host->guest call to the `wit_bindgen_rt::async_support::callback`
        // function, which exercises different code paths in both the host and
        // the guest, which we want to test here.
        let duration = Duration::from_millis(10);
        monotonic_clock::wait_for(duration.as_nanos().try_into().unwrap()).await;

        chain_http::handle(request).await
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
