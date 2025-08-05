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
/// so that it can be re-thrown during replay
///
/// Unforunately since we cannot serialize [anyhow::Error], there's no good
/// way to equate errors across record/replay boundary without creating a
/// common error format. Perhaps this is future work
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

impl core::error::Error for EventActionError {}

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
/// Returns [`ReplayError::FailedValidation`] if typechecking fails
#[inline(always)]
fn replay_args_typecheck<T>(src_types: Option<T>, expect_types: T) -> Result<(), ReplayError>
where
    T: PartialEq,
{
    #[cfg(feature = "rr-type-validation")]
    {
        if let Some(types) = src_types {
            if types == expect_types {
                Ok(())
            } else {
                Err(ReplayError::FailedValidation)
            }
        } else {
            println!(
                "Warning: Replay typechecking cannot be performed since recorded trace is missing validation data"
            );
            Ok(())
        }
    }
    #[cfg(not(feature = "rr-type-validation"))]
    Ok(())
}

/// Validation of values
#[inline(always)]
fn replay_args_valcheck<T>(src_val: T, expect_val: T) -> Result<(), ReplayError>
where
    T: PartialEq,
{
    #[cfg(feature = "rr-type-validation")]
    {
        if src_val == expect_val {
            Ok(())
        } else {
            Err(ReplayError::FailedValidation)
        }
    }
    #[cfg(not(feature = "rr-type-validation"))]
    Ok(())
}

/// Trait signifying types that can be validated on replay
///
/// All `PartialEq` and `Eq` types are directly validatable with themselves.
/// Note however that some [`Validate`] implementations are present even
/// when feature `rr-validate` is disabled, when validation is needed
/// for a faithful replay (e.g. [`component_events::InstantiationEvent`]).
pub trait Validate<T: ?Sized> {
    /// Perform a validation of the event to ensure replay consistency
    fn validate(&self, expect: &T) -> Result<(), ReplayError>;

    /// Write a log message
    fn log(&self)
    where
        Self: fmt::Debug,
    {
        log::debug!("Validating => {:?}", self);
    }
}

impl<T> Validate<T> for T
where
    T: PartialEq + fmt::Debug,
{
    /// All types that are [`PartialEq`] are directly validatable with themselves
    fn validate(&self, expect: &T) -> Result<(), ReplayError> {
        self.log();
        if self == expect {
            Ok(())
        } else {
            Err(ReplayError::FailedValidation)
        }
    }
}

pub mod component_events;
pub mod core_events;
