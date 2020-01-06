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

```
$ cargo new --bin wasmtime_hello
$ cd wasmtime_hello
$ cp $WASM_FILES/hello.wasm .
```

We will be using the wasmtime engine/API to run the wasm file, so we will add the dependency to `Cargo.toml`:

```
[dependencies]
wasmtime = "<current version>"
```

where "<current version>" is the current version number of the `wasmtime` crate.

It is time to add code to the `src/main.rs`. First, the engine and storage need to be activated:

```rust
use wasmtime::*;

let engine = Engine::default();
let store = HostRef::new(Store::new(&engine));
```

The `HostRef` will be used a lot -- it is a "convenience" object to store and refer an object between the host and
the embedded environments.

The `hello.wasm` can be read from the file system and provided to the `Module` object constructor as `&[u8]`:

```rust
use std::fs::read;

let hello_wasm = read("hello.wasm").expect("wasm file");

let module = HostRef::new(Module::new(&store, &hello_wasm).expect("wasm module"));
```

The module instance can now be created. Normally, you would provide exports, but in this case, there are none required:

```rust
let instance = Instance::new(&store, &module, &[]).expect("wasm instance");
```

Everything is set. If a WebAssembly module has a start function -- it was run.
The instance's exports can be used at this point. wasmtime provides functions
to look up an export by name, and ensure that it's a function:

```rust
let answer = instance.find_export_by_name("answer").expect("answer").func().expect("function");
```

The exported function can be called using the `call` method. Remember that in most of the cases,
a `HostRef<_>` object will be returned, so `borrow()` or `borrow_mut()` method has to be used to refer the
specific object. The exported "answer" function accepts no parameters and returns a single `i32` value.

```rust
let result = answer.borrow().call(&[]).expect("success");
println!("Answer: {}", result[0].i32());
```

The names of the WebAssembly module's imports and exports can be discovered by means of module's corresponding methods.

# src/main.rs

```rust
use std::fs::read;
use wasmtime::*;

fn main() {
    let engine = Engine::default();
    let store = HostRef::new(Store::new(&engine));

    let wasm = read("hello.wasm").expect("wasm file");

    let module = HostRef::new(Module::new(&store, &wasm).expect("wasm module"));
    let instance = Instance::new(&store, &module, &[]).expect("wasm instance");

    let answer = instance.find_export_by_name("answer").expect("answer").func().expect("function");
    let result = answer.borrow().call(&[]).expect("success");
    println!("Answer: {}", result[0].i32());
}
```
