# WebAssembly Text Format (`*.wat`)

While not necessarily a full-blown language you might be curious how Wasmtime
interacts with [the `*.wat` text format][spec]! The `wasmtime` CLI and Rust
embedding API both support the `*.wat` text format by default.

"Hello, World!" is pretty nontrivial in the `*.wat` format since it's
assembly-like and not really intended to be a primary programming language. That
being said we can create a simple add function to call it!

For example if you have a file `add.wat` like so:

```wat
(module
  (func (export "add") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add))
```

Then you can execute this on the CLI with:

```sh
$ wasmtime add.wat --invoke add 1 2
warning: ...
warning: ...
3
```

And we can see that we're already adding numbers!

You can also see how this works in the Rust API like so:

```rust
# extern crate wasmtime;
# extern crate anyhow;
use wasmtime::*;

# fn main() -> anyhow::Result<()> {
let store = Store::default();
let wat = r#"
  (module
    (func (export "add") (param i32 i32) (result i32)
      local.get 0
      local.get 1
      i32.add))
"#;
let module = Module::new(&store, wat)?;
let instance = Instance::new(&module, &[])?;
let add = instance.get_export("add").and_then(|f| f.func()).unwrap();
let add = add.get2::<i32, i32, i32>()?;
println!("1 + 2 = {}", add(1, 2)?);
# Ok(())
# }
```

[spec]: https://webassembly.github.io/spec/core/text/index.html
