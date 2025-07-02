use crate::ValRaw;
use crate::prelude::*;
use core::fmt;
use core::mem::{self, MaybeUninit};
use serde::{Deserialize, Serialize};
use wasmtime_environ::component::InterfaceType;
use wasmtime_environ::{WasmFuncType, WasmValType};

const VAL_RAW_SIZE: usize = mem::size_of::<ValRaw>();

#[derive(Debug)]
pub enum ReplayError {
    EmptyBuffer,
    FailedFuncValidation,
    IncorrectEventVariant,
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBuffer => {
                write!(f, "replay buffer is empty!")
            }
            Self::FailedFuncValidation => {
                write!(f, "func replay event typecheck validation failed")
            }
            Self::IncorrectEventVariant => {
                write!(f, "event methods invoked on incorrect variant")
            }
        }
    }
}

impl std::error::Error for ReplayError {}

/// Transmutable byte array used to serialize [`ValRaw`] union
///
/// Maintaining the exact layout is crucial for zero-copy transmutations
/// between [`ValRawSer`] and [`ValRaw`]
#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
#[repr(C)]
pub(super) struct ValRawSer([u8; VAL_RAW_SIZE]);

impl From<ValRaw> for ValRawSer {
    fn from(value: ValRaw) -> Self {
        unsafe { Self(mem::transmute(value)) }
    }
}

impl From<ValRawSer> for ValRaw {
    fn from(value: ValRawSer) -> Self {
        unsafe { mem::transmute(value.0) }
    }
}

impl From<MaybeUninit<ValRaw>> for ValRawSer {
    /// Uninitialized data is assumed, and serialized
    fn from(value: MaybeUninit<ValRaw>) -> Self {
        unsafe { Self::from(value.assume_init()) }
    }
}

impl fmt::Debug for ValRawSer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hex_digits_per_byte = 2;
        let _ = write!(f, "0x..");
        for b in self.0.iter().rev() {
            let _ = write!(f, "{:0width$x}", b, width = hex_digits_per_byte);
        }
        Ok(())
    }
}

type RRFuncArgVals = Vec<ValRawSer>;

/// Note: Switch [`CoreFuncArgTypes`] to use [`Vec<WasmValType>`] for better efficiency
type CoreFuncArgTypes = WasmFuncType;

/// Construct [`RRFuncArgVals`] from raw value buffer
fn func_argvals_from_raw_slice<T>(args: &[T]) -> RRFuncArgVals
where
    ValRawSer: From<T>,
    T: Copy,
{
    args.iter().map(|x| ValRawSer::from(*x)).collect::<Vec<_>>()
}

/// Encode [`RRFuncArgVals`] back into raw value buffer
fn func_argvals_into_raw_slice(rr_args: RRFuncArgVals, raw_args: &mut [MaybeUninit<ValRaw>]) {
    for (src, dst) in rr_args.into_iter().zip(raw_args.iter_mut()) {
        *dst = MaybeUninit::new(src.into());
    }
}

/// Typechecking validation for replay, if `src_types` exist
///
/// Returns [`ReplayError::FailedFuncValidation`] if typechecking fails
fn replay_args_typecheck<T>(src_types: Option<&T>, expect_types: &T) -> Result<(), ReplayError>
where
    T: PartialEq,
{
    if let Some(types) = src_types {
        if types == expect_types {
            Ok(())
        } else {
            Err(ReplayError::FailedFuncValidation)
        }
    } else {
        println!(
            "Warning: Replay typechecking cannot be performed 
                            since recorded trace is missing validation data"
        );
        Ok(())
    }
}

/// A call event from a Wasm component into the host
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentHostFuncEntryEvent {
    /// Raw values passed across the call entry boundary
    args: RRFuncArgVals,

    /// Optional param/return types (required to support replay validation).
    ///
    /// Note: This relies on the invariant that [InterfaceType] will always be
    /// deterministic. Currently, the type indices into various [ComponentTypes]
    /// do maintain this allowing for quick type-checking.
    types: Option<InterfaceType>,
}
impl ComponentHostFuncEntryEvent {
    // Record
    pub fn new(args: &[MaybeUninit<ValRaw>], types: Option<InterfaceType>) -> Self {
        Self {
            args: func_argvals_from_raw_slice(args),
            types: types,
        }
    }
    // Replay
    pub fn validate(&self, expect_types: &InterfaceType) -> Result<(), ReplayError> {
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
    /// Optional param/return types (required to support replay validation)
    types: Option<InterfaceType>,
}
impl ComponentHostFuncReturnEvent {
    // Record
    pub fn new(args: &[ValRaw], types: Option<InterfaceType>) -> Self {
        Self {
            args: func_argvals_from_raw_slice(args),
            types: types,
        }
    }
    // Replay
    /// Consume the caller event and encode it back into the slice with an optional
    /// typechecking validation of the event.
    pub fn move_into_slice(
        self,
        args: &mut [MaybeUninit<ValRaw>],
        expect_types: Option<&InterfaceType>,
    ) -> Result<(), ReplayError> {
        if let Some(e) = expect_types {
            replay_args_typecheck(self.types.as_ref(), e)?;
        }
        func_argvals_into_raw_slice(self.args, args);
        Ok(())
    }
}

/// A call event from a Core Wasm module into the host
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreHostFuncEntryEvent {
    /// Raw values passed across the call/return boundary
    args: RRFuncArgVals,
    /// Optional param/return types (required to support replay validation)
    types: Option<CoreFuncArgTypes>,
}
impl CoreHostFuncEntryEvent {
    // Record
    pub fn new(args: &[MaybeUninit<ValRaw>], types: Option<WasmFuncType>) -> Self {
        Self {
            args: func_argvals_from_raw_slice(args),
            types: types,
        }
    }
    // Replay
    pub fn validate(&self, expect_types: &CoreFuncArgTypes) -> Result<(), ReplayError> {
        replay_args_typecheck(self.types.as_ref(), expect_types)
    }
}

/// A return event after a host call for a Core Wasm
///
/// Matches 1:1 with [`CoreHostFuncEntryEvent`]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreHostFuncReturnEvent {
    /// Raw values passed across the call/return boundary
    args: RRFuncArgVals,
    /// Optional param/return types (required to support replay validation)
    types: Option<CoreFuncArgTypes>,
}
impl CoreHostFuncReturnEvent {
    // Record
    pub fn new(args: &[MaybeUninit<ValRaw>], types: Option<WasmFuncType>) -> Self {
        Self {
            args: func_argvals_from_raw_slice(args),
            types: types,
        }
    }
    // Replay
    /// Consume the caller event and encode it back into the slice with an optional
    /// typechecking validation of the event.
    pub fn move_into_slice(
        self,
        args: &mut [MaybeUninit<ValRaw>],
        expect_types: Option<&WasmFuncType>,
    ) -> Result<(), ReplayError> {
        if let Some(e) = expect_types {
            replay_args_typecheck(self.types.as_ref(), e)?;
        }
        func_argvals_into_raw_slice(self.args, args);
        Ok(())
    }
}
