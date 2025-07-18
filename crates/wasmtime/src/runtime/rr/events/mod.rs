use super::ReplayError;
use crate::ValRaw;
use crate::prelude::*;
use core::fmt;
use core::mem::{self, MaybeUninit};
use serde::{Deserialize, Serialize};

/// A serde compatible representation of errors produced by actions during
/// initial recording for specific events
///
/// We need this since the [anyhow::Error] trait object cannot be used. This
/// type just encapsulates the corresponding display messages during recording
/// so that it can be validated and/or re-thrown during replay
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventActionError {
    ReallocError(String),
    LowerError(String),
    LowerStoreError(String),
}

impl fmt::Display for EventActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReallocError(s) | Self::LowerError(s) | Self::LowerStoreError(s) => {
                write!(f, "{}", s)
            }
        }
    }
}

impl std::error::Error for EventActionError {}

type ValRawBytes = [u8; mem::size_of::<ValRaw>()];

/// Types that can be converted zero-copy to [`ValRawBytes`] for
/// serialization/deserialization in record/replay (since
/// unions are non serializable by `serde`)
///
/// Essentially [`From`] and [`Into`] but local to the crate
/// to bypass orphan rule for externally defined types
trait ValRawBytesConvertable {
    fn to_valraw_bytes(self) -> ValRawBytes;
    fn from_valraw_bytes(value: ValRawBytes) -> Self;
}

impl ValRawBytesConvertable for ValRaw {
    #[inline]
    fn to_valraw_bytes(self) -> ValRawBytes {
        self.as_bytes()
    }
    #[inline]
    fn from_valraw_bytes(value: ValRawBytes) -> Self {
        ValRaw::from_bytes(value)
    }
}

impl ValRawBytesConvertable for MaybeUninit<ValRaw> {
    #[inline]
    fn to_valraw_bytes(self) -> ValRawBytes {
        // Uninitialized data is assumed and serialized, so hence
        // may contain some undefined values
        unsafe { self.assume_init() }.to_valraw_bytes()
    }
    #[inline]
    fn from_valraw_bytes(value: ValRawBytes) -> Self {
        MaybeUninit::new(ValRaw::from_valraw_bytes(value))
    }
}

type RRFuncArgVals = Vec<ValRawBytes>;

/// Construct [`RRFuncArgVals`] from raw value buffer
fn func_argvals_from_raw_slice<T>(args: &[T]) -> RRFuncArgVals
where
    T: ValRawBytesConvertable + Copy,
{
    args.iter().map(|x| x.to_valraw_bytes()).collect()
}

/// Encode [`RRFuncArgVals`] back into raw value buffer
fn func_argvals_into_raw_slice<T>(rr_args: RRFuncArgVals, raw_args: &mut [T])
where
    T: ValRawBytesConvertable,
{
    for (src, dst) in rr_args.into_iter().zip(raw_args.iter_mut()) {
        *dst = T::from_valraw_bytes(src);
    }
}

/// Typechecking validation for replay, if `src_types` exist
///
/// Returns [`ReplayError::FailedFuncValidation`] if typechecking fails
#[cfg(feature = "rr-type-validation")]
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
            "Warning: Replay typechecking cannot be performed since recorded trace is missing validation data"
        );
        Ok(())
    }
}

pub mod component_wasm;
pub mod core_wasm;
