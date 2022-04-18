#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime::{Config, Engine, Module};

fn create_engine() -> Engine {
    let mut config = Config::default();
    // Safety: the Cranelift option `regalloc_checker` does not alter
    // the generated code at all; it only does extra checking after
    // compilation.
    unsafe {
        config.cranelift_flag_enable("regalloc_checker").unwrap();
    }
    Engine::new(&config).expect("Could not construct Engine")
}

fuzz_target!(|data: &[u8]| {
    let engine = create_engine();
    wasmtime_fuzzing::oracles::log_wasm(data);
    drop(Module::new(&engine, data));
});
