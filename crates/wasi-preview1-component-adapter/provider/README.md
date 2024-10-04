<div align="center">
  <h1><code>wasi-preview1-component-adapter-provider</code></h1>
  <strong>A <a href="https://bytecodealliance.org/">Bytecode Alliance</a> project</strong>
  <p>
    <strong>
      A utility library containing binaries for WASI Preview1 adapters for easy use from Rust.
    </strong>
  </p>

  <p>
    <a href="https://crates.io/crates/wasi-preview1-component-adapter-provider"><img src="https://img.shields.io/crates/v/wasi-preview1-component-adapter-provider.svg?style=flat-square" alt="Crates.io version" /></a>
    <a href="https://crates.io/crates/wasi-preview1-component-adapter-provider"><img src="https://img.shields.io/crates/d/wasi-preview1-component-adapter-provider.svg?style=flat-square" alt="Download" /></a>
    <a href="https://docs.rs/wasi-preview1-component-adapter-provider/"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs docs" /></a>
  </p>
</div>

`wasi-preview1-component-adapter-provider` contains the raw bytes of the WASI Preview1 to Preview2 adapters (Reactor, Command, and Proxy).

For example, if you wanted to write the adapter bytes back into a `.wasm` binary:

```rust
use wasi_preview1_component_adapter_provider::WASI_SNAPSHOT_PREVIEW1_REACTOR_ADAPTER;

fn main() {
    std::fs::write(
        "wasi_snapshot_preview1.reactor.wasm",
        WASI_SNAPSHOT_PREVIEW1_REACTOR_ADAPTER
    ).expect("failed to write bytes to file");
}
```

A more realistic use-case is performing the *adaptation* step of preparing a WASI Preview2 component from an existing WASI Preview1 component:

```rust
use wasi_preview1_component_adapter_provider::WASI_SNAPSHOT_PREVIEW1_REACTOR_ADAPTER;
use wit_component::ComponentEncoder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wasm_p1_bytes = std::fs::read("path/to/your/your-component.p1.wasm")?;

    let wasm_p2_bytes = ComponentEncoder::default()
        .module(&wasm_p1_bytes)?
        .adapter(
            "wasi_snapshot_preview1",
            WASI_SNAPSHOT_PREVIEW1_REACTOR_ADAPTER,
        )?
        .validate(true)
        .encode()?;

    std::fs::write("your-component.p2.wasm", wasm_p2_bytes)?;

    Ok(())
}
```

## What is a component adapter?

Code compiled to WebAssembly as described by the [base WebAssembly Spec][wasm-spec] is considered a WebAssembly "module" or "core module".

To robustly support rich types, composition, and easier interoperability, the [Component Model][cm] was created and is the spec that governs the idea of a "WebAssembly component". The component model wraps any existing WebAssembly core module(s) and extends them with additional features.

To standardize underlying system interoperability (ex. reading files, system time) in code compiled to WebAssembly, the [WebAssembly System Interface ("WASI")][wasi] was created. WASI is implemented by language tool chains (ex. Rust supports `wasm32-wasi`/`wasm32-wasip1` as a target, with [support for `wasm32-wasip2` on the way][rust-target-wasm32-wasi]), and enables compiling a WebAssembly component that utilizes [the interfaces that make up WASI Preview1][wasi-p1-interfaces].

In the ongoing work of building WASI, WASI Preview2 which contains more functionality [has been released][wasi-p2-release] -- but building directly to Preview2 is not yet integrated into language toolchains. However, Preview1 components (which *can* be produced by curren toolchains) can be *adapted* to WASI Preview2.

This is where component adapters come in.

**Component Adapters are WebAssembly binaries that contain logic that can take any WebAssembly binary implemented in terms of WASI Preview1 and convert them to WASI Preview2.**

This crate contains the binary content of those the adapter WebAssembly binaries, made accessible as constant arrays of bytes (`const &[u8]`).

[wasm-spec]: https://webassembly.github.io/spec/core
[cm]: https://component-model.bytecodealliance.org
[wasi]: https://wasi.dev/
[wasi-p2-release]: https://bytecodealliance.org/articles/WASI-0.2
[rust-target-wasm32-wasi]: https://blog.rust-lang.org/2024/04/09/updates-to-rusts-wasi-targets.html
[wasi-p1-interfaces]: https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/witx/wasi_snapshot_preview1.witx
