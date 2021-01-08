//! Contains the macro-generated implementation of wasi-nn from the its witx definition file.
use crate::ctx::WasiNnCtx;
use crate::ctx::WasiNnError;

// Generate the traits and types of wasi-nn in several Rust modules (e.g. `types`).
wiggle::from_witx!({
    witx: ["$WASI_ROOT/phases/ephemeral/witx/wasi_ephemeral_nn.witx"],
    ctx: WasiNnCtx,
    errors: { errno => WasiNnError }
});

use types::Errno;

/// Wiggle generates code that performs some input validation on the arguments passed in by users of
/// wasi-nn. Here we convert the validation error into one (or more, eventually) of the error
/// variants defined in the witx.
impl types::GuestErrorConversion for WasiNnCtx {
    fn into_errno(&self, e: wiggle::GuestError) -> Errno {
        eprintln!("Guest error: {:?}", e);
        Errno::InvalidArgument
    }
}

impl<'a> types::UserErrorConversion for WasiNnCtx {
    fn errno_from_wasi_nn_error(&self, e: WasiNnError) -> Result<Errno, wiggle::Trap> {
        eprintln!("Host error: {:?}", e);
        match e {
            WasiNnError::OpenvinoError(_) => unimplemented!(),
            WasiNnError::GuestError(_) => unimplemented!(),
            WasiNnError::UsageError(_) => unimplemented!(),
        }
    }
}

/// Additionally, we must let Wiggle know which of our error codes represents a successful operation.
impl wiggle::GuestErrorType for Errno {
    fn success() -> Self {
        Self::Success
    }
}
