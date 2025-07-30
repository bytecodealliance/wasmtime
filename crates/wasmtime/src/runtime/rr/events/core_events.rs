//! Module comprising of core wasm events
use super::*;
#[expect(unused_imports, reason = "used for doc-links")]
use wasmtime_environ::{WasmFuncType, WasmValType};

/// Note: Switch [`CoreFuncArgTypes`] to use [`Vec<WasmValType>`] for better efficiency
#[cfg(feature = "rr-type-validation")]
type CoreFuncArgTypes = WasmFuncType;

/// A call event from a Core Wasm module into the host
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostFuncEntryEvent {
    /// Raw values passed across the call/return boundary
    args: RRFuncArgVals,
    /// Optional param/return types (required to support replay validation)
    #[cfg(feature = "rr-type-validation")]
    types: Option<CoreFuncArgTypes>,
}
impl HostFuncEntryEvent {
    // Record
    pub fn new(
        args: &[MaybeUninit<ValRaw>],
        #[cfg(feature = "rr-type-validation")] types: Option<WasmFuncType>,
    ) -> Self {
        Self {
            args: func_argvals_from_raw_slice(args),
            #[cfg(feature = "rr-type-validation")]
            types: types,
        }
    }
    // Replay
    #[cfg(feature = "rr-type-validation")]
    pub fn validate(&self, expect_types: &CoreFuncArgTypes) -> Result<(), ReplayError> {
        replay_args_typecheck(self.types.as_ref(), expect_types)
    }
}

/// A return event after a host call for a Core Wasm
///
/// Matches 1:1 with [`HostFuncEntryEvent`]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostFuncReturnEvent {
    /// Raw values passed across the call/return boundary
    args: RRFuncArgVals,
    /// Optional param/return types (required to support replay validation)
    #[cfg(feature = "rr-type-validation")]
    types: Option<CoreFuncArgTypes>,
}
impl HostFuncReturnEvent {
    // Record
    pub fn new(
        args: &[MaybeUninit<ValRaw>],
        #[cfg(feature = "rr-type-validation")] types: Option<WasmFuncType>,
    ) -> Self {
        Self {
            args: func_argvals_from_raw_slice(args),
            #[cfg(feature = "rr-type-validation")]
            types: types,
        }
    }
    // Replay
    /// Consume the caller event and encode it back into the slice with an optional
    /// typechecking validation of the event.
    pub fn move_into_slice(
        self,
        args: &mut [MaybeUninit<ValRaw>],
        #[cfg(feature = "rr-type-validation")] expect_types: Option<&WasmFuncType>,
    ) -> Result<(), ReplayError> {
        #[cfg(feature = "rr-type-validation")]
        if let Some(e) = expect_types {
            replay_args_typecheck(self.types.as_ref(), e)?;
        }
        func_argvals_into_raw_slice(self.args, args);
        Ok(())
    }
}
