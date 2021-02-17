mod ctx;
mod r#impl;
mod witx;

pub use ctx::WasiNnCtx;

// Defines a `struct WasiNn` with member fields and appropriate APIs for dealing with all the
// various WASI exports.
wasmtime_wiggle::wasmtime_integration!({
    // The wiggle code to integrate with lives here:
    target: witx,
    // This must be the same witx document as used above:
    witx: ["$WASI_ROOT/phases/ephemeral/witx/wasi_ephemeral_nn.witx"],
    // This must be the same ctx type as used for the target:
    ctx: WasiNnCtx,
    // This macro will emit a struct to represent the instance, with this name and docs:
    modules: {
        wasi_ephemeral_nn => {
          name: WasiNn,
          docs: "An instantiated instance of the wasi-nn exports.",
        }
    },
});
