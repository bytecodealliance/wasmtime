# `wasmtime-rust` - Using WebAssembly from Rust

This crate is intended to be an example of how to load WebAssembly files from a
native Rust application. You can always use `wasmtime` and its family of crates
directly, but the purpose of this crate is to provide an ergonomic macro:

```rust
#[wasmtime_rust::wasmtime]
trait WasmMarkdown {
    fn render(&mut self, input: &str) -> String;
}

fn main() -> anyhow::Result<()> {
    let mut markdown = WasmMarkdown::load_file("markdown.wasm")?;
    println!("{}", markdown.render("# Hello, Rust!"));

    Ok(())
}
```

The `wasmtime` macro defined in the `wasmtime-rust` crate is placed on a `trait`
which includes the set of functionality which a wasm module should export. In
this case we're expecting one `render` function which takes and returns a
string.

The macro expands to a `struct` with all of the methods on the trait (they must
all be `&mut self`) and one function called `load_file` to actually instantiate
the module.

Note that this macro is still in early stages of development, so error messages
aren't great yet and all functionality isn't supported yet.

## Missing features

Currently if the wasm module imports any symbols outside of the WASI namespace
the module will not load. It's intended that support for this will be added soon
though!
