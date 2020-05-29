use wiggle_test::{impl_errno, WasiCtx};

#[derive(Debug, thiserror::Error)]
pub enum RichError {
    #[error("Invalid argument: {0}")]
    InvalidArg(String),
    #[error("Won't cross picket line: {0}")]
    PicketLine(String),
}

wiggle::from_witx!({
    witx: ["tests/arrays.witx"],
    ctx: WasiCtx,
    errors: { errno => RichError },
});

impl_errno!(types::Errno, types::GuestErrorConversion);

impl<'a> types::UserErrorConversion for WasiCtx<'a> {
    fn errno_from_rich_error(&self, _e: RichError) -> types::Errno {
        unimplemented!();
    }
}

impl<'a> arrays::Arrays for WasiCtx<'a> {
    fn reduce_excuses(
        &self,
        _excuses: &types::ConstExcuseArray,
    ) -> Result<types::Excuse, RichError> {
        unimplemented!()
    }
    fn populate_excuses(&self, _excuses: &types::ExcuseArray) -> Result<(), RichError> {
        unimplemented!()
    }
}
