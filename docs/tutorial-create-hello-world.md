# Creating `hello-world.wasm`

There are a number of ways to create `.wasm` files but for the purposes of this
tutorial, we'll be using the Rust toolchain. You can find more information on
creating `.wasm` files from other languages in the
[Writing WebAssembly section](./wasm.md).

To build WebAssembly binaries with Rust, you'll need the standard Rust toolchain.

[Follow these instructions to install `rustc`, `rustup` and `cargo`](https://www.rust-lang.org/tools/install)

Next, you should add WebAssembly as a build target for cargo like so:

```sh
$ rustup target add wasm32-wasi
```

Finally, create a new Rust project called 'hello-world'. You can do this by running:

```sh
$ cargo new hello-world
```

After that, the hello-world folder should look like this.

```text
hello-world/
├── Cargo.lock
├── Cargo.toml
└── src
   └── main.rs
```

And the `main.rs` file inside the `src` folder should contain the following rust code.

```rust
fn main() {
    println!("Hello, world!");
}

```

Now, we can tell `cargo` to build a WebAssembly file:

```sh
$ cargo build --target wasm32-wasi
```

Now, in the `target` folder, there's a `hello-world.wasm` file. You can find it here:

```text
hello-world/
├── Cargo.lock
├── Cargo.toml
├── src
└── target
   └── ...
   └── wasm32-wasi
      └── debug
         └── ...
         └── hello-world.wasm

```
