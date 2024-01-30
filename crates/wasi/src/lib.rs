//! `wasmtime-wasi` now supports using multiple snapshots to interface to the
//! same `WasiCtx`!
//!
//! `wasmtime_wasi::Wasi::new(&Store, WasiCtx)` is a struct which owns your
//! `WasiCtx` and provides linkage to every available snapshot.
//!
//! Individual snapshots are available through
//! `wasmtime_wasi::snapshots::preview_{0, 1}::Wasi::new(&Store, Rc<RefCell<WasiCtx>>)`.

#![warn(clippy::cast_sign_loss)]

#[cfg(feature = "preview2")]
pub mod preview2;

/// Exit the process with a conventional OS error code as long as Wasmtime
/// understands the error. If the error is not an `I32Exit` or `Trap`, return
/// the error back to the caller for it to decide what to do.
///
/// Note: this function is designed for usage where it is acceptable for
/// Wasmtime failures to terminate the parent process, such as in the Wasmtime
/// CLI; this would not be suitable for use in multi-tenant embeddings.
#[cfg(feature = "exit")]
pub fn maybe_exit_on_error(e: anyhow::Error) -> anyhow::Error {
    use std::process;
    use wasmtime::Trap;

    // If a specific WASI error code was requested then that's
    // forwarded through to the process here without printing any
    // extra error information.
    let code = e.downcast_ref::<preview2::I32Exit>().map(|e| e.0);
    if let Some(exit) = code {
        // Print the error message in the usual way.
        // On Windows, exit status 3 indicates an abort (see below),
        // so return 1 indicating a non-zero status to avoid ambiguity.
        if cfg!(windows) && exit >= 3 {
            process::exit(1);
        }
        process::exit(exit);
    }

    // If the program exited because of a trap, return an error code
    // to the outside environment indicating a more severe problem
    // than a simple failure.
    if e.is::<Trap>() {
        eprintln!("Error: {:?}", e);

        if cfg!(unix) {
            // On Unix, return the error code of an abort.
            process::exit(128 + libc::SIGABRT);
        } else if cfg!(windows) {
            // On Windows, return 3.
            // https://docs.microsoft.com/en-us/cpp/c-runtime-library/reference/abort?view=vs-2019
            process::exit(3);
        }
    }

    e
}
