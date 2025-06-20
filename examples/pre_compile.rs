//! Pre-compiling a Wasm program.

use wasmtime::{Config, Engine, Result, Strategy};

fn main() -> Result<()> {
    // Configure Wasmtime for compiling Wasm programs to x86_64 with the
    // Cranelift compiler.
    let mut config = Config::new();
    config.strategy(Strategy::Cranelift);
    if let Err(error) = config.target("x86_64") {
        eprintln!(
            "this Wasmtime was not built with the x86_64 Cranelift backend \
             enabled: {error:?}",
        );
        return Ok(());
    }

    // Create an `Engine` with that configuration.
    let engine = match Engine::new(&config) {
        Ok(engine) => engine,
        Err(error) => {
            println!("Wasmtime build is incompatible with config: {error:?}");
            return Ok(());
        }
    };

    // Pre-compile a Wasm program.
    //
    // Note that passing the Wasm text format, like we are doing here, is only
    // supported when the `wat` cargo feature is enabled.
    let precompiled = engine.precompile_module(
        r#"
            (module
              (func (export "add") (param i32 i32) (result i32)
                (i32.add (local.get 0) (local.get 1))
              )
            )
        "#
        .as_bytes(),
    )?;

    // Write the pre-compiled program to a file.
    //
    // Note that the `.cwasm` extension is conventional for these files, and is
    // what the Wasmtime CLI will use by default, for example.
    std::fs::write("add.cwasm", &precompiled)?;

    // And we are done -- now a different Wasmtime embedding can load and run
    // the pre-compiled Wasm program from that `add.cwasm` file!
    Ok(())
}
