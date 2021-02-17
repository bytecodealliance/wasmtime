//! Contains the macro-generated implementation of wasi-nn from the its witx definition file.
use crate::ctx::WasiNnCtx;
use crate::ctx::WasiNnError;

// Generate the traits and types of wasi-nn in several Rust modules (e.g. `types`).
wiggle::from_witx!({
    witx: ["$WASI_ROOT/phases/ephemeral/witx/wasi_ephemeral_nn.witx"],
    ctx: WasiNnCtx,
    errors: { nn_errno => WasiNnError }
});

use types::NnErrno;

/// Wiggle generates code that performs some input validation on the arguments passed in by users of
/// wasi-nn. Here we convert the validation error into one (or more, eventually) of the error
/// variants defined in the witx.
impl types::GuestErrorConversion for WasiNnCtx {
    fn into_nn_errno(&self, e: wiggle::GuestError) -> NnErrno {
        eprintln!("Guest error: {:?}", e);
        NnErrno::InvalidArgument
    }
}

impl<'a> types::UserErrorConversion for WasiNnCtx {
    fn nn_errno_from_wasi_nn_error(&self, e: WasiNnError) -> Result<NnErrno, wiggle::Trap> {
        eprintln!("Host error: {:?}", e);
        match e {
            WasiNnError::OpenvinoError(_) => unimplemented!(),
            WasiNnError::GuestError(_) => unimplemented!(),
            WasiNnError::UsageError(_) => unimplemented!(),
        }
    }
}

/// Additionally, we must let Wiggle know which of our error codes represents a successful operation.
impl wiggle::GuestErrorType for NnErrno {
    fn success() -> Self {
        Self::Success
    }
}
