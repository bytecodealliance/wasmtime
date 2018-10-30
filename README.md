# Lightbeam

This is an early-stage experimental project to build a single-pass
wasm-function-to-machine-code translator.

It's currently built with [dynasm](https://crates.io/crates/dynasm) and
targets x86-64, however the function\_body.rs/backend.rs split is likely
to evolve towards a configuration point allowing other targets or even
other assemblers to be supported.

It's a very early stage project, and a good one for learning how
WebAssembly works at a low level, for learning assembly programming, or
both! And we're happy to mentor. So welcome, and check out the
[issue tracker] to see what's happening and how to get involved!

[issue tracker]: https://github.com/CraneStation/lightbeam/issues
