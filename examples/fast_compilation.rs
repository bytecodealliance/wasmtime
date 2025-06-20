//! Tuning Wasmtime for fast compilation.
//!
//! If your application design is compatible with pre-compiling Wasm programs,
//! prefer doing that.

use wasmtime::{Cache, Config, Engine, Result, Strategy};

fn main() -> Result<()> {
    let mut config = Config::new();

    // Enable the compilation cache, using the default cache configuration
    // settings.
    config.cache(Some(Cache::from_file(None)?));

    // Enable Winch, Wasmtime's baseline compiler.
    config.strategy(Strategy::Winch);

    // Enable parallel compilation.
    config.parallel_compilation(true);

    // Build an `Engine` with our `Config` that is tuned for fast Wasm
    // compilation.
    let engine = Engine::new(&config)?;

    // Now we can use `engine` to compile and/or run some Wasm programs...

    let _ = engine;
    Ok(())
}
