# Embedding Wasmtime in Rust

This document shows how to embed Wasmtime using the Rust API, and run a simple
wasm program.

# Create some wasm

Let's create a simple WebAssembly file with a single exported function that returns an integer:

```wat
(;; wat2wasm hello.wat -o $WASM_FILES/hello.wasm ;;)
(module
  (func (export "answer") (result i32)
     i32.const 42
  )
)
```

# Create rust project

```sh
$ cargo new --bin wasmtime_hello
$ cd wasmtime_hello
$ cp $WASM_FILES/hello.wasm .
```

We will be using the wasmtime engine/API to run the wasm file, so we will add the dependency to `Cargo.toml`:

```toml
[dependencies]
wasmtime = "<current version>"
```

where "<current version>" is the current version number of the `wasmtime` crate.

It is time to add code to the `src/main.rs`. First, storage needs to be activated:

```rust
# extern crate wasmtime;
use wasmtime::*;

let store = Store::default();
```

The `hello.wasm` can be read from the file system and provided to the `Module` object constructor as `&[u8]`:

```rust,no_run
# extern crate wasmtime;
# use wasmtime::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# let store = Store::default();
use std::fs::read;

let hello_wasm = read("hello.wasm")?;

let module = Module::new(&store, &hello_wasm)?;
# Ok(())
# }
```

The module instance can now be created. Normally, you would provide imports, but
in this case, there are none required:

```rust
# extern crate wasmtime;
# use wasmtime::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# let store = Store::default();
# let module = Module::new(&store, "(module)")?;
let instance = Instance::new(&module, &[])?;
# Ok(())
# }
```

Everything is set. If a WebAssembly module has a start function -- it was run.
The instance's exports can be used at this point. wasmtime provides functions
to get an export by name, and ensure that it's a function:

```rust
# extern crate wasmtime;
# use wasmtime::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# let store = Store::default();
# let module = Module::new(&store, r#"(module (func (export "answer")))"#)?;
# let instance = Instance::new(&module, &[])?;
let answer = instance.get_export("answer").expect("answer").func().expect("function");
# Ok(())
# }
```

The exported function can be called using the `call` method. The exported
"answer" function accepts no parameters and returns a single `i32` value.

```rust
# extern crate wasmtime;
# use wasmtime::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# let store = Store::default();
# let module = Module::new(&store, r#"(module (func (export "answer") (result i32) i32.const 2))"#)?;
# let instance = Instance::new(&module, &[])?;
# let answer = instance.get_export("answer").expect("answer").func().expect("function");
let result = answer.call(&[])?;
println!("Answer: {:?}", result[0].i32());
# Ok(())
# }
```

Since we know the signature of the function ahead of time, we can also assert
its signature and call the function directly without doing conversions:

```rust
# extern crate wasmtime;
# use wasmtime::*;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# let store = Store::default();
# let module = Module::new(&store, r#"(module (func (export "answer") (result i32) i32.const 2))"#)?;
# let instance = Instance::new(&module, &[])?;
# let answer = instance.get_export("answer").expect("answer").func().expect("function");
let answer = answer.get0::<i32>()?;
let result: i32 = answer()?;
println!("Answer: {}", result);
# Ok(())
# }
```

The names of the WebAssembly module's imports and exports can be discovered by
means of module's corresponding methods.

# src/main.rs

```rust,no_run
# extern crate wasmtime;
use std::error::Error;
use std::fs::read;
use wasmtime::*;

fn main() -> Result<(), Box<dyn Error>> {
    let store = Store::default();

    let wasm = read("hello.wasm")?;

    let module = Module::new(&store, &wasm)?;
    let instance = Instance::new(&module, &[])?;

    let answer = instance.get_export("answer").expect("answer").func().expect("function");
    let result = answer.call(&[])?;
    println!("Answer: {:?}", result[0].i32());
    Ok(())
}
```
