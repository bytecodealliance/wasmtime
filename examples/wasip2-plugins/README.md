Build with `cargo build --release`

Then, create a plugins directory (e.g. `plugins`) and copy `c-plugin/add.wasm`
and `js-plugin/subtract.wasm` into that directory.

Finally, run (for example):

`target/release/calculator --plugins plugins add 1 2`
or
`target/release/calculator --plugins plugins subtract 1 2`

For more details, see the ["Calculator with WebAssembly Plugins"](http://docs.wasmtime.dev/wasip2-plugins.html)
section of the documentation.
