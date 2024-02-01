// This just tests that things compile when `tracing: false` is set,
// which isn't the default.

wiggle::from_witx!({
    witx: ["$CARGO_MANIFEST_DIR/tests/atoms.witx"],
    async: {
        atoms::double_int_return_float,
    },
    tracing: false,
});

impl wiggle::GuestErrorType for types::Errno {
    fn success() -> Self {
        types::Errno::Ok
    }
}
