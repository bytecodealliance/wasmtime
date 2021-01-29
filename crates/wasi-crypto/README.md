# wasmtime-wasi-crypto

This crate enables support for the [wasi-crypto] APIs in Wasmtime.

The sole purpose of the implementation is to allow bindings and
application developers to test the proposed APIs. This implementation
is not meant to be used in production. Like the specification, it is
currently experimental and its functionality can quickly change.

Since the [wasi-crypto] API is expected to be an optional feature of
WASI, this crate is currently separate from the [wasi-common] crate.

* [documentation]
* [interfaces reference]
* [interfaces reference (compact)]

[wasi-crypto]: https://github.com/WebAssembly/wasi-crypto
[wasi-common]: ../../wasi-common
[documentation]: ../spec/docs/wasi-crypto.md
[interfaces reference]: ../spec/witx/wasi_ephemeral_crypto.md
[interfaces reference (compact)]: ../spec/witx/wasi_ephemeral_crypto.txt

## Wasmtime integration

Use the Wasmtime APIs to instantiate a Wasm module and link the
`wasi-crypto` modules as follows:

```rust
use wasmtime_wasi_crypto::{
    WasiCryptoAsymmetricCommon, WasiCryptoCommon, WasiCryptoCtx, WasiCryptoSignatures,
    WasiCryptoSymmetric,
};

let cx_crypto = WasiCryptoCtx::new();
WasiCryptoCommon::new(linker.store(), cx_crypto.clone()).add_to_linker(linker)?;
WasiCryptoAsymmetricCommon::new(linker.store(), cx_crypto.clone()).add_to_linker(linker)?;
WasiCryptoSignatures::new(linker.store(), cx_crypto.clone()).add_to_linker(linker)?;
WasiCryptoSymmetric::new(linker.store(), cx_crypto.clone()).add_to_linker(linker)?;

let wasi = wasmtime_wasi::old::snapshot_0::Wasi::new(linker.store(), mk_cx()?);
wasi.add_to_linker(linker)?;
```

## Building Wasmtime

Wasmtime must be compiled with the `wasi-crypto` feature flag
(disabled by default) in order to include the crypto APIs.

## Examples

Example [rust bindings] and [assemblyscript bindings] are provided to
demonstrate how these APIs can be used and exposed to applications in
an idiomatic way.

[rust bindings]: ../spec/implementations/bindings/rust
[assemblyscript bindings]: ../spec/implementations/bindings/assemblyscript
