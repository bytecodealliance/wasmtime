//! Module comprising of core wasm events
use super::*;
#[expect(unused_imports, reason = "used for doc-links")]
use wasmtime_environ::{WasmFuncType, WasmValType};

/// Note: Switch [`CoreFuncArgTypes`] to use [`Vec<WasmValType>`] for better efficiency
type CoreFuncArgTypes = WasmFuncType;

/// A call event from a Core Wasm module into the host
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostFuncEntryEvent {
    /// Raw values passed across the call/return boundary
    args: RRFuncArgVals,
    /// Param/return types (required to support replay validation)
    types: CoreFuncArgTypes,
}
impl HostFuncEntryEvent {
    // Record
    pub fn new(args: &[MaybeUninit<ValRaw>], types: WasmFuncType) -> Self {
        Self {
            args: func_argvals_from_raw_slice(args),
            types: types,
        }
    }
}
impl Validate<CoreFuncArgTypes> for HostFuncEntryEvent {
    fn validate(&self, expect_types: &CoreFuncArgTypes) -> Result<(), ReplayError> {
        self.log();
        if &self.types == expect_types {
            Ok(())
        } else {
            Err(ReplayError::FailedValidation)
        }
    }
}

/// A return event after a host call for a Core Wasm
///
/// Matches 1:1 with [`HostFuncEntryEvent`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostFuncReturnEvent {
    /// Raw values passed across the call/return boundary
    args: RRFuncArgVals,
}
impl HostFuncReturnEvent {
    // Record
    pub fn new(args: &[MaybeUninit<ValRaw>]) -> Self {
        Self {
            args: func_argvals_from_raw_slice(args),
        }
    }
    // Replay
    /// Consume the caller event and encode it back into the slice with an optional
    /// typechecking validation of the event.
    pub fn move_into_slice(self, args: &mut [MaybeUninit<ValRaw>]) {
        func_argvals_into_raw_slice(self.args, args);
    }
}
