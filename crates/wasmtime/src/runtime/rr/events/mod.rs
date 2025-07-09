use super::ReplayError;
use crate::ValRaw;
use crate::prelude::*;
use core::fmt;
use core::mem::{self, MaybeUninit};
use serde::{Deserialize, Serialize};

const VAL_RAW_SIZE: usize = mem::size_of::<ValRaw>();

/// A serde compatible representation of errors produced by specific events
///
/// We need this since the [anyhow::Error] trait object cannot be used. This
/// type just encapsulates the corresponding display messages during recording
/// so that it can be validated and/or re-thrown during replay
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventError {
    ReallocError(String),
    LowerError(String),
    LowerStoreError(String),
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReallocError(s) | Self::LowerError(s) | Self::LowerStoreError(s) => {
                write!(f, "{}", s)
            }
        }
    }
}

impl std::error::Error for EventError {}

/// Transmutable byte array used to serialize [`ValRaw`] union
///
/// Maintaining the exact layout is crucial for zero-copy transmutations
/// between [`ValRawSer`] and [`ValRaw`] as currently assumed. However,
/// in the future, this type could be represented with LEB128s
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

impl From<ValRawSer> for MaybeUninit<ValRaw> {
    fn from(value: ValRawSer) -> Self {
        MaybeUninit::new(value.into())
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

/// Construct [`RRFuncArgVals`] from raw value buffer
fn func_argvals_from_raw_slice<T>(args: &[T]) -> RRFuncArgVals
where
    ValRawSer: From<T>,
    T: Copy,
{
    args.iter().map(|x| ValRawSer::from(*x)).collect::<Vec<_>>()
}

/// Encode [`RRFuncArgVals`] back into raw value buffer
fn func_argvals_into_raw_slice<T>(rr_args: RRFuncArgVals, raw_args: &mut [T])
where
    ValRawSer: Into<T>,
{
    for (src, dst) in rr_args.into_iter().zip(raw_args.iter_mut()) {
        *dst = src.into();
    }
}

/// Typechecking validation for replay, if `src_types` exist
///
/// Returns [`ReplayError::FailedFuncValidation`] if typechecking fails
fn replay_args_typecheck<T>(src_types: Option<T>, expect_types: T) -> Result<(), ReplayError>
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

pub mod component_wasm;
pub mod core_wasm;
