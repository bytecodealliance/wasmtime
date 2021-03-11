# Using WebAssembly from Rust

This document shows an example of how to embed Wasmtime using the [Rust
API][apidoc] to execute a simple wasm program. Be sure to also check out the
[full API documentation][apidoc] for a full listing of what the [`wasmtime`
crate][wasmtime] has to offer and the [book examples for
Rust](./examples-rust-embed.md) for more information.

[apidoc]: https://bytecodealliance.github.io/wasmtime/api/wasmtime/
[wasmtime]: https://crates.io/crates/wasmtime

## Creating the WebAssembly to execute

Creation of a WebAssembly file is generally covered by the [Writing
WebAssembly chapter](./wasm.md), so we'll just assume that you've already got a
wasm file on hand for the rest of this tutorial. To make things simple we'll
also just assume you've got a `hello.wat` file which looks like this:

```wat
(module
  (func (export "answer") (result i32)
     i32.const 42
  )
)
```

Here we're just exporting one function which returns an integer that we'll read
from Rust.

## Hello, World!

First up let's create a rust project

```sh
$ cargo new --bin wasmtime_hello
$ cd wasmtime_hello
```

Next you'll want to add `hello.wat` to the root of your project.

We will be using the `wasmtime` crate to run the wasm file, so next up we need a
dependency in `Cargo.toml`:

```toml
[dependencies]
wasmtime = "0.18.0"
```

Next up let's write the code that we need to execute this wasm file. The
simplest version of this looks like so:

```rust,no_run
# extern crate wasmtime;
use std::error::Error;
use wasmtime::*;

fn main() -> Result<(), Box<dyn Error>> {
    let engine = Engine::default();
    // A `Store` is a sort of "global object" in a sense, but for now it suffices
    // to say that it's generally passed to most constructors.
    let store = Store::new(&engine);

# if false {
    // We start off by creating a `Module` which represents a compiled form
    // of our input wasm module. In this case it'll be JIT-compiled after
    // we parse the text format.
    let module = Module::from_file(&engine, "hello.wat")?;
# }
# let module = Module::new(&engine, r#"(module (func (export "answer") (result i32) i32.const 42))"#)?;

    // After we have a compiled `Module` we can then instantiate it, creating
    // an `Instance` which we can actually poke at functions on.
    let instance = Instance::new(&store, &module, &[])?;

    // The `Instance` gives us access to various exported functions and items,
    // which we access here to pull out our `answer` exported function and
    // run it.
    let answer = instance.get_func("answer")
        .expect("`answer` was not an exported function");

    // There's a few ways we can call the `answer` `Func` value. The easiest
    // is to statically assert its signature with `typed` (in this case
    // asserting it takes no arguments and returns one i32) and then call it.
    let answer = answer.typed::<(), i32>()?;

    // And finally we can call our function! Note that the error propagation
    // with `?` is done to handle the case where the wasm function traps.
    let result = answer.call(())?;
    println!("Answer: {:?}", result);
    Ok(())
}
```

We can build and execute our example with `cargo run`. Note that by depending on
`wasmtime` you're depending on a JIT compiler, so it may take a moment to build
all of its dependencies:

```sh
$ cargo run
  Compiling ...
  ...
   Finished dev [unoptimized + debuginfo] target(s) in 42.32s
    Running `wasmtime_hello/target/debug/wasmtime_hello`
Answer: 42
```

and there we go! We've now executed our first WebAssembly in `wasmtime` and
gotten the result back.

## Importing Host Functionality

What we've just seen is a pretty small example of how to call a wasm function
and take a look at the result. Most interesting wasm modules, however, are going
to import some functions to do something a bit more interesting. For that you'll
need to provide imported functions from Rust for wasm to call!

Let's take a look at a wasm module which imports a logging function as well as
some simple arithmetic from the environment.

```wat
(module
  (import "" "log" (func $log (param i32)))
  (import "" "double" (func $double (param i32) (result i32)))
  (func (export "run")
    i32.const 0
    call $log
    i32.const 1
    call $log
    i32.const 2
    call $double
    call $log
  )
)
```

This wasm module will call our `"log"` import a few times and then also call the
`"double"` import. We can compile and instantiate this module with code that
looks like this:

```rust,no_run
# extern crate wasmtime;
use std::error::Error;
use wasmtime::*;

fn main() -> Result<(), Box<dyn Error>> {
    let engine = Engine::default();
    let store = Store::new(&engine);
# if false {
    let module = Module::from_file(&engine, "hello.wat")?;
# }
# let module = Module::new(&engine, r#"(module (import "" "log" (func $log (param i32))) (import "" "double" (func $double (param i32) (result i32))) (func (export "run") i32.const 0 call $log i32.const 1 call $log i32.const 2 call $double call $log))"#)?;

    // First we can create our `log` function, which will simply print out the
    // parameter it receives.
    let log = Func::wrap(&store, |param: i32| {
        println!("log: {}", param);
    });

    // Next we can create our double function which doubles the input it receives.
    let double = Func::wrap(&store, |param: i32| param * 2);

    // When instantiating the module we now need to provide the imports to the
    // instantiation process. This is the second slice argument, where each
    // entry in the slice must line up with the imports in the module.
    let instance = Instance::new(&store, &module, &[log.into(), double.into()])?;

    let run = instance.get_typed_func::<(), ()>("run")?;
    Ok(run.call(())?)
}
```

Note that there's a number of ways to define a `Func`, be sure to [consult its
documentation][`Func`] for other ways to create a host-defined function.

[`Func`]: https://bytecodealliance.github.io/wasmtime/api/wasmtime/struct.Func.html
