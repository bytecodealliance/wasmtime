#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::{generators::api::ApiCalls, oracles};

fuzz_target!(|api: ApiCalls| {
    oracles::make_api_calls(api);
});
