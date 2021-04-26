//! Contains the macro-generated implementation of wasi-nn from the its witx definition file.
use crate::ctx::WasiNnCtx;
use crate::ctx::WasiNnError;

// Generate the traits and types of wasi-nn in several Rust modules (e.g. `types`).
wiggle::from_witx!({
    witx: ["$WASI_ROOT/phases/ephemeral/witx/wasi_ephemeral_nn.witx"],
    errors: { nn_errno => WasiNnError }
});

use types::NnErrno;

impl<'a> types::UserErrorConversion for WasiNnCtx {
    fn nn_errno_from_wasi_nn_error(&self, e: WasiNnError) -> Result<NnErrno, wiggle::Trap> {
        eprintln!("Host error: {:?}", e);
        match e {
            WasiNnError::OpenvinoSetupError(_) => unimplemented!(),
            WasiNnError::OpenvinoInferenceError(_) => unimplemented!(),
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
