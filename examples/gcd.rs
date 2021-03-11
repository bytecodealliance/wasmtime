//! Example of instantiating of the WebAssembly module and invoking its exported
//! function.

// You can execute this example with `cargo run --example gcd`

use anyhow::Result;
use wasmtime::*;

fn main() -> Result<()> {
    // Load our WebAssembly (parsed WAT in our case), and then load it into a
    // `Module` which is attached to a `Store` cache. After we've got that we
    // can instantiate it.
    let store = Store::default();
    let module = Module::from_file(store.engine(), "examples/gcd.wat")?;
    let instance = Instance::new(&store, &module, &[])?;

    // Invoke `gcd` export
    let gcd = instance.get_typed_func::<(i32, i32), i32>("gcd")?;

    println!("gcd(6, 27) = {}", gcd.call((6, 27))?);
    Ok(())
}
