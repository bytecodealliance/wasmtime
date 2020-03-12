# Rust

The [Rust Programming Language](https://www.rust-lang.org) supports WebAssembly
as a compilation target. If you're not familiar with Rust it's recommended to
start [with its introductory documentation](https://www.rust-lang.org/learn).
Compiling to WebAssembly will involve specifying the desired target via the
`--target` flag, and to do this there are a number of "target triples" for
WebAssembly compilation in Rust:

* `wasm32-wasi` - when using `wasmtime` this is likely what you'll be using. The
  WASI target is integrated into the standard library and is intended on
  producing standalone binaries.
* `wasm32-unknown-unknown` - this target, like the WASI one, is focused on
  producing single `*.wasm` binaries. The standard library, however, is largely
  stubbed out since the "unknown" part of the target means libstd can't assume
  anything. This means that while binaries will likely work in `wasmtime`,
  common conveniences like `println!` or `panic!` won't work.
* `wasm32-unknown-emscripten` - this target is intended to work in a web browser
  and produces a `*.wasm` file coupled with a `*.js` file, and it is not
  compatible with `wasmtime`.

For the rest of this documentation we'll assume that you're using the
`wasm32-wasi` target for compiling Rust code and executing inside of `wasmtime`.

## Hello, World!

Cross-compiling to WebAssembly involves a number of knobs that need
configuration, but you can often gloss over these internal details by using
build tooling intended for the WASI target. For example we can start out writing
a WebAssembly binary with [`cargo
wasi`](https://github.com/bytecodealliance/cargo-wasi).

First up we'll [install `cargo
wasi`](https://bytecodealliance.github.io/cargo-wasi/install.html):

```sh
$ cargo install cargo-wasi
```

Next we'll make a new Cargo project:

```sh
$ cargo new hello-world
$ cd hello-world
```

Inside of `src/main.rs` you'll see the canonical Rust "Hello, World!" using
`println!`. We'll be executing this for the `wasm32-wasi` target, so you'll want
to make sure you're previously [built `wasmtime` and inserted it into
`PATH`](./cli-install.md);

```sh
$ cargo wasi run
info: downloading component 'rust-std' for 'wasm32-wasi'
info: installing component 'rust-std' for 'wasm32-wasi'
   Compiling hello-world v0.1.0 (/hello-world)
    Finished dev [unoptimized + debuginfo] target(s) in 0.16s
     Running `/.cargo/bin/cargo-wasi target/wasm32-wasi/debug/hello-world.wasm`
     Running `target/wasm32-wasi/debug/hello-world.wasm`
Hello, world!
```

And we're already running our first WebAssembly code inside of `wasmtime`!

While it's automatically happening for you as part of `cargo wasi`, you can also
run `wasmtime` yourself:

```sh
$ wasmtime target/wasm32-wasi/debug/hello-world.wasm
Hello, world!
```

You can check out the [introductory documentation of
`cargo-wasi`](https://bytecodealliance.github.io/cargo-wasi/hello-world.html) as
well for some more information.

## Writing Libraries

Previously for "Hello, World!" we created a *binary* project which used
`src/main.rs`. Not all `*.wasm` binaries are intended to be executed like
commands, though. Some are intended to be loaded into applications and called
through various APIs, acting more like libraries. For this use case you'll want
to add this to `Cargo.toml`:

```toml
# in Cargo.toml ...

[lib]
crate-type = ['cdylib']
```

and afterwards you'll want to write your code in `src/lib.rs` like so:

```rust
#[no_mangle]
pub extern "C" fn print_hello() {
    println!("Hello, world!");
}
```

When you execute `cargo wasi build` that'll generate a `*.wasm` file which has
one exported function, `print_hello`. We can then run it via the CLI like so:

```sh
$ cargo wasi build
   Compiling hello-world v0.1.0 (/home/alex/code/hello-world)
    Finished dev [unoptimized + debuginfo] target(s) in 0.08s
$ wasmtime --invoke print_hello target/wasm32-wasi/debug/hello_world.wasm
Hello, world!
```

As a library crate one of your primary consumers may be other languages as well.
You'll want to consult the [section of this book for using `wasmtime` from
Python`](./lang-python.md) and after running through the basics there you can
execute our file in Python:

```sh
$ cp target/wasm32-wasi/debug/hello_world.wasm .
$ python3
>>> import wasmtime
>>> import hello_world
>>> hello_world.print_hello()
Hello, world!
()
>>>
```

Note that this form of using `#[no_mangle]` Rust functions is pretty primitive.
You're only able to work with primitive datatypes like integers and floats.
While this works for some applications if you need to work with richer types
like strings or structs, then you'll want to use the support in `wasmtime` for
interface types.

## WebAssembly Interface Types

> **Note**: support for interface types has temporarily removed from Wasmtime.
> This documentation is somewhat up to date but will no longer work with recent
> versions of Wasmtime. For more information see
> https://github.com/bytecodealliance/wasmtime/issues/677

Working with WebAssembly modules at the bare-bones level means that you're only
dealing with integers and floats. Many APIs, however, want to work with things
like byte arrays, strings, structures, etc. To facilitate these interactions the
[WebAssembly Interface Types
Proposal](https://github.com/webassembly/interface-types) comes into play. The
`wasmtime` runtime has support for interface types, and the Rust toolchain has
library support in a crate called
[`wasm-bindgen`](https://crates.io/crates/wasm-bindgen).

> **Note**: WebAssembly Interface Types is still a WebAssembly proposal and is
> under active development. The toolchain may not match the exact specification,
> and during development you'll generally need to make sure tool versions are
> all kept up to date to ensure everything aligns right. This'll all smooth over
> as the proposal stabilizes!

To get started with WebAssembly interface types let's write a library
module which will generate a greeting for us. The module itself won't do any
printing, we'll simply be working with some strings.

To get starts let's add this to our `Cargo.toml`:

```toml
[lib]
crate-type = ['cdylib']

[dependencies]
wasm-bindgen = "0.2.54"
```

Using this crate, we can then update our `src/lib.rs` with the following:

```rust,ignore
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
```

Then we can build this with:

```sh
$ cargo wasi build --release
    Updating crates.io index
...
    Finished dev [unoptimized + debuginfo] target(s) in 9.57s
 Downloading precompiled wasm-bindgen v0.2.54
```

and we have our new wasm binary!

> **Note**: for now when using `wasm-bindgen` you must use `--release` mode to
> build wasi binaries with interface types.

We can then test out support for this with the CLI:

```sh
$ wasmtime --invoke greet ./target/wasm32-wasi/release/hello_world.wasm "Wasmtime CLI"
warning: using `--invoke` with a function that takes arguments is experimental and may break in the future
warning: using `--invoke` with a function that returns values is experimental and may break in the future
Hello, Wasmtime CLI!
```

Here we can see some experimental warnings, but we got our error message printed
out! The first CLI parameter, `"Wasmtime CLI"`, was passed as the first argument
of the `greet` function. The resulting string was then printed out to the
console.

Like before, we can also execute this with Python:

```sh
$ cp target/wasm32-wasi/release/hello_world.wasm .
$ python3
>>> import wasmtime
>>> import hello_world
>>> hello_world.greet('python interpreter')
'Hello, python interpreter!'
>>>
```

Note that `wasm-bindgen` was originally developed for JS and usage in a browser,
but a subset of its implementation (such as arguments which are strings) are
supported for WebAssembly interface types. You can also check out the [reference
documentation for `wasm-bindgen`](https://rustwasm.github.io/wasm-bindgen/) for
more information about how it works. Note that the `wasm-bindgen` support for
wasm interface type is still in its nascent phase and is likely to be greatly
improved in the future.

## Exporting Rust functionality

Currently only Rust functions can be exported from a wasm module. Rust functions
must be `#[no_mangle]` to show up in the final binary, but if you're using
`#[wasm_bindgen]` that will happen automatically for you.

Memory is by default exported from Rust modules under the name `memory`. This
can be tweaked with the `-Clink-arg` flag to rustc to pass flags to LLD, the
WebAssembly code linker.

Tables cannot be imported at this time. When using `rustc` directly there is no
support for `anyref` and only one function table is supported. When using
`wasm-bindgen` it may inject an `anyref` table if necessary, but this table is
an internal detail and is not exported. The function table can be exported by
passing the `--export-table` argument to LLD (via `-C link-arg`) or can be
imported with the `--import-table`.

Rust currently does not have support for exporting or importing custom `global`
values.

## Importing host functionality

Only functions can be imported in Rust at this time, and they can be imported
via raw interfaces like:

```rust
# struct MyStruct;
#[link(wasm_import_module = "the-wasm-import-module")]
extern "C" {
    // imports the name `foo` from `the-wasm-import-module`
    fn foo();

    // functions can have integer/float arguments/return values
    fn translate(a: i32) -> f32;

    // Note that the ABI of Rust and wasm is somewhat in flux, so while this
    // works, it's recommended to rely on raw integer/float values where
    // possible.
    fn translate_fancy(my_struct: MyStruct) -> u32;

    // you can also explicitly specify the name to import, this imports `bar`
    // instead of `baz` from `the-wasm-import-module`.
    #[link_name = "bar"]
    fn baz();
}
```

When you're using `wasm-bindgen` you would instead use:

```rust,ignore
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "the-wasm-import-module")]
extern "C" {
    fn foo();
    fn baz();
    // ...
}
```

Note that unless you're using interface types you likely don't need
`wasm-bindgen`.
