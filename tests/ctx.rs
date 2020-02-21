use wiggle_runtime::GuestError;

pub struct WasiCtx {
    pub guest_errors: Vec<GuestError>,
}

impl WasiCtx {
    pub fn new() -> Self {
        Self {
            guest_errors: vec![],
        }
    }
}

// Errno is used as a first return value in the functions above, therefore
// it must implement GuestErrorType with type Context = WasiCtx.
// The context type should let you do logging or debugging or whatever you need
// with these errors. We just push them to vecs.
#[macro_export]
macro_rules! impl_errno {
    ( $errno:ty ) => {
        impl wiggle_runtime::GuestErrorType for $errno {
            type Context = WasiCtx;
            fn success() -> $errno {
                <$errno>::Ok
            }
            fn from_error(e: GuestError, ctx: &mut WasiCtx) -> $errno {
                eprintln!("GUEST ERROR: {:?}", e);
                ctx.guest_errors.push(e);
                types::Errno::InvalidArg
            }
        }
    };
}
