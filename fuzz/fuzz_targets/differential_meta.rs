#![no_main]

use libfuzzer_sys::arbitrary::{Result, Unstructured};
use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::generators::{DiffValue, SingleInstModule};
use wasmtime_fuzzing::oracles::engine::{
    get_exported_function_signatures, DiffEngine, DiffIgnorable,
};
use wasmtime_fuzzing::oracles::{diff_wasmtime, differential, engine};

const NUM_INVOCATIONS: usize = 5;

fuzz_target!(|data: &[u8]| {
    // errors in `run` have to do with not enough input in `data`, which we
    // ignore here since it doesn't affect how we'd like to fuzz.
    drop(run(&data));
});

fn run(data: &[u8]) -> Result<()> {
    let mut u = Unstructured::new(data);

    // Generate the Wasm module. TODO eventually, this should pick between the
    // single-instruction and wasm-smith modules, but currently the wasm-smith
    // module generation will eat up all of the random data, leaving none for
    // the remaining decisions that follow (e.g., choosing an engine, generating
    // arguments).
    let module: &SingleInstModule = u.arbitrary()?;
    let wasm = module.to_bytes();
    let features = module.to_features();

    // Choose a right-hand side Wasm engine--this will always be Wasmtime. The
    // order (execute `lhs` first, then `rhs`) is important because, in some
    // cases (e.g., OCaml spec interpreter), both sides register signal
    // handlers; Wasmtime uses these signal handlers for catching various
    // WebAssembly failures. On certain OSes (e.g. Linux x86_64), the signal
    // handlers interfere, observable as an uncaught `SIGSEGV`--not even caught
    // by libFuzzer. By always running Wasmtime second, its signal handlers are
    // registered most recently and they catch failures appropriately. We create
    // `rhs` first, however, so we have the option of creating a compatible
    // Wasmtime engine (e.g., pooling allocator memory differences).
    let rhs = diff_wasmtime::WasmtimeEngine::arbitrary_with_features(&mut u, &features)?;

    // Choose a left-hand side Wasm engine.
    let lhs = engine::choose(&mut u, &features, &rhs)?;

    // Instantiate each engine and try each exported functions with various
    // values.
    let mut lhs_instance = lhs
        .instantiate(&module.to_bytes())
        .expect_or_ignore("failed to instantiate `lhs` module")?;
    let mut rhs_instance = rhs
        .instantiate(&module.to_bytes())
        .expect_or_ignore("failed to instantiate `rhs` module")?;
    for (name, signature) in get_exported_function_signatures(&wasm)
        .expect("failed to extract exported function signatures")
    {
        let mut invocations = 0;
        loop {
            let arguments = signature
                .params
                .iter()
                .map(|&t| DiffValue::arbitrary_of_type(&mut u, t.into()))
                .collect::<Result<Vec<_>>>()?;
            differential(
                lhs_instance.as_mut(),
                rhs_instance.as_mut(),
                &name,
                &arguments,
            )
            .expect("failed to run differential evaluation");

            // We evaluate the same function with different arguments until we
            // hit a predetermined limit or we run out of unstructured data--it
            // does not make sense to re-evaluate the same arguments over and
            // over.
            invocations += 1;
            if invocations > NUM_INVOCATIONS || u.is_empty() {
                break;
            }
        }
    }

    Ok(())
}
