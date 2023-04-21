#![cfg(feature = "test_programs_http")]
use std::sync::Once;
mod runtime;
mod utils;

static LOG_INIT: Once = Once::new();

fn setup_log() {
    LOG_INIT.call_once(tracing_subscriber::fmt::init)
}

include!(concat!(env!("OUT_DIR"), "/wasi_http_tests.rs"));
