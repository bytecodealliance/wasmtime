//! Tuning Wasmtime for fast Wasm execution.

use wasmtime::{Config, Engine, Result, Strategy};

fn main() -> Result<()> {
    let mut config = Config::new();

    // Enable the Cranelift optimizing compiler.
    config.strategy(Strategy::Cranelift);

    // Enable signals-based traps. This is required to elide explicit
    // bounds-checking.
    config.signals_based_traps(true);

    // Configure linear memories such that explicit bounds-checking can be
    // elided.
    config.memory_reservation(1 << 32);
    config.memory_guard_size(1 << 32);

    if CROSS_COMPILING {
        // When cross-compiling, force-enable various ISA extensions.
        //
        // Warning: it is wildly unsafe to run a pre-compiled Wasm program that
        // enabled a particular ISA extension on a machine that does not
        // actually support that ISA extension!!!
        unsafe {
            if let Err(error) = config.target("x86_64") {
                eprintln!(
                    "Wasmtime was not compiled with the x86_64 backend for \
                     Cranelift enabled: {error:?}",
                );
                return Ok(());
            }
            config.cranelift_flag_enable("has_sse41");
            config.cranelift_flag_enable("has_avx");
            config.cranelift_flag_enable("has_avx2");
            config.cranelift_flag_enable("has_lzcnt");
        }
    }

    // Build an `Engine` with our `Config` that is tuned for fast Wasm
    // execution.
    let engine = match Engine::new(&config) {
        Ok(engine) => engine,
        Err(error) => {
            eprintln!("Configuration is incompatible with this host platform: {error:?}");
            return Ok(());
        }
    };

    // Now we can use `engine` to compile and/or run some Wasm programs...

    let _ = engine;
    Ok(())
}

const CROSS_COMPILING: bool = false;
