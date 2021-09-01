# Markdown Parser

The following steps describe an implementation of a WASI markdown parser, in Rust, using [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark).

First, we will generate a new executable with cargo:

```bash
cargo new --bin rust_wasi_markdown_parser
cd rust_wasi_markdown_parser
```

Then, we will open the `src/main.rs` and enter the following contents. Please see the comments to understand what our program will be doing.

## `src/main.rs`

```rust,should_panic
{{#include ./rust_wasi_markdown_parser/src/main.rs}}
```

Next, we will want to add WASI as a target that we can compile to. We will ask the rustup tool to install support for WASI. Then, we will compile our program to WASI. To do this we will run:

```bash
rustup target add wasm32-wasi
cargo build --target wasm32-wasi
```

Our wasm file should be compiled to `target/wasm32-wasi/debug/rust_wasi_markdown_parser.wasm`. It is worth noting that even though the WASI APIs are not being used directly, when we compile our program to target WASI, the rust APIs and standard library will be using these WASI APIs under the hood for us! Now that we have our program compiled to target WASI, let's run our program!

To do this, we can use the Wasmtime CLI. However, there is one thing to note about Wasmtime, WASI, and the capability based security model. We need to give our program explicit access to read files on our device. Wasm modules that implement WASI will not have this capability unless we give them the capability.

To grant the capability to read in a directory using the Wasmtime CLI, we need to use the --dir flag. --dir will instruct wasmtime to make the passed directory available to access files from. (You can also `--mapdir GUEST_DIRECTORY::HOST_DIRECTORY` to make it available under a different path inside the content.) For example:

```bash
wasmtime --dir . my-wasi-program.wasm
```

For this example, we will be passing a markdown file to our program called: `example-markdown.md`, that will exist in whatever our current directory (`./`) is. Our markdown file, `example-markdown.md`, will contain:

```md
# Hello!

I am example markdown for this demo!
```

So, **to run our compiled WASI program, we will run**:

```bash
wasmtime --dir . target/wasm32-wasi/debug/rust_wasi_markdown_parser.wasm -- ./example_markdown.md
```

Which should look like the following:

```html 
<h1>Hello!</h1>
<p>I am example markdown for this demo!</p>
```

Hooray! We were able to write a Wasm Module, that uses WASI to read a markdown file, parse the markdown, and write the output to stdout! Continue reading to see more examples of using Wasmtime to execute Wasm Modules, from the CLI or even embedded in your application!

