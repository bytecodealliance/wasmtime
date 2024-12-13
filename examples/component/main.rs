use anyhow::Context;
use std::{fs, path::Path};

use wasmtime::{
    component::{bindgen, Component, Linker},
    Config, Engine, Result, Store,
};

// Generate bindings of the guest and host components.
bindgen!("convert" in "./examples/component/convert.wit");

struct HostComponent;

// Implementation of the host interface defined in the wit file.
impl host::Host for HostComponent {
    fn multiply(&mut self, a: f32, b: f32) -> f32 {
        a * b
    }
    fn apply(&mut self, a: f32, b: f32, op: host::BinaryOperation) -> f32 {
        match op {
            host::BinaryOperation::Add => a + b,
            host::BinaryOperation::Multiply => a * b,
        }
    }
}

struct MyState {
    host: HostComponent,
}

/// This function is only needed until rust can natively output a component.
///
/// Generally embeddings should not be expected to do this programmatically, but instead
/// language specific tooling should be used, for example in Rust `cargo component`
/// is a good way of doing that: https://github.com/bytecodealliance/cargo-component
///
/// In this example we convert the code here to simplify the testing process and build system.
fn convert_to_component(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let bytes = fs::read(&path).context("failed to read input file")?;
    // allow direct use of a component : look at the layer field of the preamble, see
    // https://github.com/WebAssembly/component-model/blob/main/design/mvp/Binary.md#component-definitions
    if bytes.len() > 6 && bytes[6] == 1 {
        Ok(bytes)
    } else {
        wit_component::ComponentEncoder::default()
            .module(&bytes)?
            .encode()
    }
}

fn main() -> Result<()> {
    // NOTE: The wasm32-unknown-unknown target is used here for simplicity, real world use cases
    // should probably use the wasm32-wasip1 target, and enable wasi preview2 within the component
    // model.
    let module_path = "target/wasm32-unknown-unknown/debug/guest.wasm";
    let component = convert_to_component(module_path)?;

    if std::env::args().len() > 1 {
        // if called with an argument, just write the component binary
        // this is useful for the c-api test, which doesn't expose wit_component::ComponentEncoder
        let component_path = module_path.replace("guest.wasm", "guest-component.wasm");
        println!("{} bytes written to {}", component.len(), component_path);
        fs::write(&component_path, &component)?;
        return Ok(());
    }

    // Create an engine with the component model enabled (disabled by default).
    let engine = Engine::new(Config::new().wasm_component_model(true))?;

    // Create our component and call our generated host functions.
    let component = Component::from_binary(&engine, &component)?;
    let mut linker = Linker::new(&engine);
    host::add_to_linker(&mut linker, |state: &mut MyState| &mut state.host)?;

    let mut store = Store::new(
        &engine,
        MyState {
            host: HostComponent {},
        },
    );
    let convert = Convert::instantiate(&mut store, &component, &linker)?;
    let result = convert.call_convert_celsius_to_fahrenheit(&mut store, 23.4)?;
    println!("Converted to: {result:?}");
    let result = convert.call_convert(&mut store, Temperature::Fahrenheit(66.2))?;
    println!("Converted to: {result:?}");
    Ok(())
}
