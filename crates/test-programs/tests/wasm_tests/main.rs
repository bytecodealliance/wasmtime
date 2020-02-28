#![cfg(feature = "test_programs")]
mod runtime;
mod utils;

use std::sync::Once;

static LOG_INIT: Once = Once::new();

fn setup_log() {
    LOG_INIT.call_once(|| {
        pretty_env_logger::init();
    })
}

include!(concat!(env!("OUT_DIR"), "/wasi_tests.rs"));
