#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use wasmtime::{Config, Engine, Store};
use wasmtime_fuzzing::generators::OptLevel;
use wasmtime_wast::WastContext;

include!(concat!(env!("OUT_DIR"), "/spectests.rs"));

#[derive(Arbitrary, Debug)]
pub struct SpecCompliantConfig {
    opt_level: OptLevel,
    debug_verifier: bool,
    debug_info: bool,
    canonicalize_nans: bool,
    spectest: usize,
}

fuzz_target!(|config: SpecCompliantConfig| {
    let mut cfg = Config::new();
    cfg.debug_info(config.debug_info)
        .cranelift_nan_canonicalization(config.canonicalize_nans)
        .cranelift_debug_verifier(config.debug_verifier)
        .cranelift_opt_level(config.opt_level.to_wasmtime());

    let (file, contents) = FILES[config.spectest % FILES.len()];
    println!("run test {:?}", file);
    let store = Store::new(&Engine::new(&cfg));
    let mut wast_context = WastContext::new(store);
    wast_context.register_spectest().unwrap();
    wast_context.run_buffer(file, contents.as_bytes()).unwrap();
});
