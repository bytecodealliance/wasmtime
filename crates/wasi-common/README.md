<div align="center">
  <h1><code>wasi-common</code></h1>

<strong>A <a href="https://bytecodealliance.org/">Bytecode Alliance</a> project</strong>

  <p>
    <strong>A library providing a common implementation of WASI hostcalls for re-use in any WASI-enabled runtime.</strong>
  </p>

  <p>
    <a href="https://crates.io/crates/wasi-common"><img src="https://img.shields.io/crates/v/wasi-common.svg?style=flat-square" alt="Crates.io version" /></a>
    <a href="https://crates.io/crates/wasi-common"><img src="https://img.shields.io/crates/d/wasi-common.svg?style=flat-square" alt="Download" /></a>
    <a href="https://docs.rs/wasi-common/"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs docs" /></a>
  </p>
</div>

The `wasi-common` crate will ultimately serve as a library providing a common implementation of
WASI hostcalls for re-use in any WASI (and potentially non-WASI) runtimes
such as [Wasmtime] and [Lucet].

The library is an adaption of [lucet-wasi] crate from the [Lucet] project, and it is
currently based on [40ae1df][lucet-wasi-tracker] git revision.

Please note that the library requires Rust compiler version at least 1.37.0.

[Wasmtime]: https://github.com/bytecodealliance/wasmtime
[Lucet]: https://github.com/fastly/lucet
[lucet-wasi]: https://github.com/fastly/lucet/tree/master/lucet-wasi
[lucet-wasi-tracker]: https://github.com/fastly/lucet/commit/40ae1df64536250a2b6ab67e7f167d22f4aa7f94

## Supported syscalls

### *nix
In our *nix implementation, we currently support the entire [WASI API]
with the exception of socket hostcalls:
- `sock_recv`
- `sock_send`
- `sock_shutdown`

We expect these to be implemented when network access is standardised.

We also currently do not support the `proc_raise` hostcall, as it is expected to
be dropped entirely from WASI.

[WASI API]: https://github.com/WebAssembly/WASI/blob/master/phases/snapshot/docs.md

### Windows
In our Windows implementation, we currently support the minimal subset of [WASI API]
which allows for running the very basic "Hello world!" style WASM apps. More coming shortly,
so stay tuned!

## Development hints
When testing the crate, you may want to enable and run full wasm32 integration testsuite. This
requires `wasm32-wasi` target installed which can be done as follows using [rustup]

```
rustup target add wasm32-wasi
```

[rustup]: https://rustup.rs

Now, you should be able to run the integration testsuite by running `cargo test` on the
`test-programs` package with `test-programs/test_programs` feature enabled:

```
cargo test --features test-programs/test_programs --package test-programs
```

## Third-Party Code
Significant parts of our hostcall implementations are derived from the C implementations in
`cloudabi-utils`. See [LICENSE.cloudabi-utils](LICENSE.cloudabi-utils) for license information.

