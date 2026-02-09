mod bindings {
    wit_bindgen::generate!({
        path: "../wasi-http/src/p3/wit",
        world: "local:local/middleware-with-chain",
        inline: "
package local:local;

world middleware-with-chain {
  include wasi:http/service@0.3.0-rc-2026-02-09;

  import chain-http;
}

interface chain-http {
  use wasi:http/types@0.3.0-rc-2026-02-09.{request, response, error-code};

  handle: async func(request: request) -> result<response, error-code>;
}
        ",
        // workaround https://github.com/bytecodealliance/wit-bindgen/issues/1544
        // generate_all
        with: {
            "wasi:http/types@0.3.0-rc-2026-02-09": test_programs::p3::wasi::http::types,
            "wasi:http/client@0.3.0-rc-2026-02-09": test_programs::p3::wasi::http::client,
            "wasi:random/random@0.3.0-rc-2026-02-09": test_programs::p3::wasi::random::random,
            "wasi:random/insecure@0.3.0-rc-2026-02-09": test_programs::p3::wasi::random::insecure,
            "wasi:random/insecure-seed@0.3.0-rc-2026-02-09": test_programs::p3::wasi::random::insecure_seed,
            "wasi:cli/stdout@0.3.0-rc-2026-02-09": test_programs::p3::wasi::cli::stdout,
            "wasi:cli/stderr@0.3.0-rc-2026-02-09": test_programs::p3::wasi::cli::stderr,
            "wasi:cli/stdin@0.3.0-rc-2026-02-09": test_programs::p3::wasi::cli::stdin,
            "wasi:cli/types@0.3.0-rc-2026-02-09": test_programs::p3::wasi::cli::types,
            "wasi:clocks/monotonic-clock@0.3.0-rc-2026-02-09": test_programs::p3::wasi::clocks::monotonic_clock,
            "wasi:clocks/system-clock@0.3.0-rc-2026-02-09": test_programs::p3::wasi::clocks::system_clock,
            "wasi:clocks/types@0.3.0-rc-2026-02-09": test_programs::p3::wasi::clocks::types,
        },
    });

    use super::Component;
    export!(Component);
}

use bindings::{exports::wasi::http::handler::Guest as Handler, local::local::chain_http};
use std::time::Duration;
use test_programs::p3::wasi::clocks::monotonic_clock;
use test_programs::p3::wasi::http::types::{ErrorCode, Request, Response};

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
