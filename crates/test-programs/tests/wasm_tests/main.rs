#![cfg(feature = "test_programs")]
mod runtime;
mod utils;

use std::sync::Once;

static LOG_INIT: Once = Once::new();

fn setup_log() {
    LOG_INIT.call_once(tracing_subscriber::fmt::init)
}

include!(concat!(env!("OUT_DIR"), "/wasi_tests.rs"));
