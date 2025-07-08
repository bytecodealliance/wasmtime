//! Module comprising of component model wasm events
use super::*;
use wasmtime_environ::component::TypeTuple;

/// A call event from a Wasm component into the host
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentHostFuncEntryEvent {
    /// Raw values passed across the call entry boundary
    args: RRFuncArgVals,

    /// Optional param/return types (required to support replay validation).
    ///
    /// Note: This relies on the invariant that [InterfaceType] will always be
    /// deterministic. Currently, the type indices into various [ComponentTypes]
    /// maintain this, allowing for quick type-checking.
    types: Option<TypeTuple>,
}
impl ComponentHostFuncEntryEvent {
    // Record
    pub fn new(args: &[MaybeUninit<ValRaw>], types: Option<&TypeTuple>) -> Self {
        Self {
            args: func_argvals_from_raw_slice(args),
            types: types.cloned(),
        }
    }
    // Replay
    pub fn validate(&self, expect_types: &TypeTuple) -> Result<(), ReplayError> {
        replay_args_typecheck(self.types.as_ref(), expect_types)
    }
}

/// A return event after a host call for a Wasm component
///
/// Matches 1:1 with [`ComponentHostFuncEntryEvent`]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentHostFuncReturnEvent {
    /// Lowered values passed across the call return boundary
    args: RRFuncArgVals,
    /// Optional param/return types (required to support replay validation).
    ///
    /// Note: This relies on the invariant that [InterfaceType] will always be
    /// deterministic. Currently, the type indices into various [ComponentTypes]
    /// maintain this, allowing for quick type-checking.
    types: Option<TypeTuple>,
}
impl ComponentHostFuncReturnEvent {
    // Record
    pub fn new(args: &[ValRaw], types: Option<&TypeTuple>) -> Self {
        Self {
            args: func_argvals_from_raw_slice(args),
            types: types.cloned(),
        }
    }
    // Replay
    pub fn validate(&self, expect_types: &TypeTuple) -> Result<(), ReplayError> {
        replay_args_typecheck(self.types.as_ref(), expect_types)
    }

    /// Consume the caller event and encode it back into the slice with an optional
    /// typechecking validation of the event.
    pub fn move_into_slice(
        self,
        args: &mut [ValRaw],
        expect_types: Option<&TypeTuple>,
    ) -> Result<(), ReplayError> {
        if let Some(e) = expect_types {
            self.validate(e)?;
        }
        func_argvals_into_raw_slice(self.args, args);
        Ok(())
    }
}
