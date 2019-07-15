mod runtime;
mod utils;

use std::sync::{Once, ONCE_INIT};

static LOG_INIT: Once = ONCE_INIT;

fn setup_log() {
    LOG_INIT.call_once(|| {
        pretty_env_logger::init();
    })
}

include!(concat!(env!("OUT_DIR"), "/misc_testsuite_tests.rs"));
