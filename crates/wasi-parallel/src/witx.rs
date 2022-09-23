//! Contains the macro-generated implementation of wasi-nn from its WITX
//! definition file.
use crate::{WasiParallel, WasiParallelError};

// Generate the traits and types of wasi-parallel to several Rust modules (e.g.
// `types`). TODO: eventually re-add Git submodule for auto-retrieval of the
// specification.
wiggle::from_witx!({
    witx: ["$WASI_ROOT/wasi-parallel.witx"],
    errors: { par_errno => WasiParallelError },
    skip: ["parallel_exec"],
});

use types::ParErrno;

// Additionally, we must let Wiggle know which of our error codes represents a
// successful operation.
impl wiggle::GuestErrorType for ParErrno {
    fn success() -> Self {
        Self::Success
    }
}

// Provide a way to map errors from the `WasiEphemeralTrait` (see `impl.rs`) to
// the WITX-defined error type.
impl wasi_ephemeral_parallel::UserErrorConversion for WasiParallel {
    fn par_errno_from_wasi_parallel_error(
        &mut self,
        e: anyhow::Error,
    ) -> Result<crate::witx::types::ParErrno, wiggle::Trap> {
        todo!("must handle error: {}", e)
    }
}
