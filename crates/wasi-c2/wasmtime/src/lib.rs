pub use wasi_c2::WasiCtx;

// Defines a `struct Wasi` with member fields and appropriate APIs for dealing
// with all the various WASI exports.
wasmtime_wiggle::wasmtime_integration!({
    // The wiggle code to integrate with lives here:
    target: wasi_c2::snapshots::preview_1,
    // This must be the same witx document as used above. This should be ensured by
    // the `WASI_ROOT` env variable, which is set in wasi-common's `build.rs`.
    witx: ["$WASI_ROOT/phases/snapshot/witx/wasi_snapshot_preview1.witx"],
    // This must be the same ctx type as used for the target:
    ctx: WasiCtx,
    // This macro will emit a struct to represent the instance,
    // with this name and docs:
    modules: { wasi_snapshot_preview1 =>
        { name: Wasi,
          docs: "An instantiated instance of the wasi exports.

This represents a wasi module which can be used to instantiate other wasm
modules. This structure exports all that various fields of the wasi instance
as fields which can be used to implement your own instantiation logic, if
necessary. Additionally [`Wasi::get_export`] can be used to do name-based
resolution.",
        // Don't use the wiggle generated code to implement proc_exit, we need
        // to hook directly into the runtime there:
          function_override: {
            proc_exit => wasi_proc_exit
          }
        },
    },
    // Error to return when caller module is missing memory export:
    missing_memory: { wasi_c2::snapshots::preview_1::types::Errno::Inval },
});

fn wasi_proc_exit(code: i32) {
    panic!("stubbed out: wasi proc exit with code {}", code)
}
