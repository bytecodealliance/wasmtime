#![no_main]

use libfuzzer_sys::fuzz_target;
use std::sync::Once;
use wasmtime_fuzzing::{generators::api::ApiCalls, oracles};

fuzz_target!(|api: ApiCalls| {
    static INIT_LOGGING: Once = Once::new();
    INIT_LOGGING.call_once(|| env_logger::init());

    log::debug!(
        "If this fuzz test fails, here is a regression tests:
```
#[test]
fn my_regression_test() {{
    use wasmtime_fuzzing::generators::{{
        api::{{ApiCall::*, ApiCalls}},
        WasmOptTtf,
    }};
    wasmtime_fuzzing::oracles::make_api_calls({:#?});
}}
```",
        api
    );

    oracles::make_api_calls(api);
});
