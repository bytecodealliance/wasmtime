use super::types as guest_types;
use super::WasiCryptoCtx;

pub use anyhow::Error;

use std::num::TryFromIntError;

#[derive(thiserror::Error, Debug)]
pub enum CryptoError {
    #[error("Success")]
    Success,
    #[error("Guest error")]
    GuestError(#[from] wiggle::GuestError),
    #[error("Not implemented")]
    NotImplemented,
    #[error("Unsupported feature")]
    UnsupportedFeature,
    #[error("Prohibited by local policy")]
    ProhibitedOperation,
    #[error("Unsupported encoding")]
    UnsupportedEncoding,
    #[error("Unsupported algorithm")]
    UnsupportedAlgorithm,
    #[error("Unsupported option")]
    UnsupportedOption,
    #[error("Invalid key")]
    InvalidKey,
    #[error("Verification failed")]
    InvalidLength,
    #[error("Invalid length")]
    VerificationFailed,
    #[error("RNG error")]
    RNGError,
    #[error("Operation failed")]
    AlgorithmFailure,
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Handle already closed")]
    Closed,
    #[error("Invalid handle")]
    InvalidHandle,
    #[error("Overflow")]
    Overflow,
    #[error("Internal error")]
    InternalError,
    #[error("Too many open handles")]
    TooManyHandles,
    #[error("Selected algorithm doesn't support a key")]
    KeyNotSupported,
    #[error("Selected algorithm requires a key")]
    KeyRequired,
    #[error("Authentication tag did not verify")]
    InvalidTag,
    #[error("Operation invalid for the selected algorithm")]
    InvalidOperation,
    #[error("Nonce required")]
    NonceRequired,
    #[error("Nonce doesn't have a correct size")]
    InvalidNonce,
    #[error("Option not set")]
    OptionNotSet,
    #[error("Key not found")]
    NotFound,
    #[error("Parameters missing")]
    ParametersMissing,
    #[error("Incompatible keys")]
    IncompatibleKeys,
    #[error("Expired secret")]
    Expired,
}

impl From<CryptoError> for guest_types::CryptoErrno {
    fn from(e: CryptoError) -> Self {
        match e {
            CryptoError::Success => guest_types::CryptoErrno::Success,
            CryptoError::GuestError(_wiggle_error) => guest_types::CryptoErrno::GuestError,
            CryptoError::NotImplemented => guest_types::CryptoErrno::NotImplemented,
            CryptoError::UnsupportedFeature => guest_types::CryptoErrno::UnsupportedFeature,
            CryptoError::ProhibitedOperation => guest_types::CryptoErrno::ProhibitedOperation,
            CryptoError::UnsupportedEncoding => guest_types::CryptoErrno::UnsupportedEncoding,
            CryptoError::UnsupportedAlgorithm => guest_types::CryptoErrno::UnsupportedAlgorithm,
            CryptoError::UnsupportedOption => guest_types::CryptoErrno::UnsupportedOption,
            CryptoError::InvalidKey => guest_types::CryptoErrno::InvalidKey,
            CryptoError::InvalidLength => guest_types::CryptoErrno::InvalidLength,
            CryptoError::VerificationFailed => guest_types::CryptoErrno::VerificationFailed,
            CryptoError::RNGError => guest_types::CryptoErrno::RngError,
            CryptoError::AlgorithmFailure => guest_types::CryptoErrno::AlgorithmFailure,
            CryptoError::InvalidSignature => guest_types::CryptoErrno::InvalidSignature,
            CryptoError::Closed => guest_types::CryptoErrno::Closed,
            CryptoError::InvalidHandle => guest_types::CryptoErrno::InvalidHandle,
            CryptoError::Overflow => guest_types::CryptoErrno::Overflow,
            CryptoError::InternalError => guest_types::CryptoErrno::InternalError,
            CryptoError::TooManyHandles => guest_types::CryptoErrno::TooManyHandles,
            CryptoError::KeyNotSupported => guest_types::CryptoErrno::KeyNotSupported,
            CryptoError::KeyRequired => guest_types::CryptoErrno::KeyRequired,
            CryptoError::InvalidTag => guest_types::CryptoErrno::InvalidTag,
            CryptoError::InvalidOperation => guest_types::CryptoErrno::InvalidOperation,
            CryptoError::NonceRequired => guest_types::CryptoErrno::NonceRequired,
            CryptoError::InvalidNonce => guest_types::CryptoErrno::InvalidNonce,
            CryptoError::OptionNotSet => guest_types::CryptoErrno::OptionNotSet,
            CryptoError::NotFound => guest_types::CryptoErrno::NotFound,
            CryptoError::ParametersMissing => guest_types::CryptoErrno::ParametersMissing,
            CryptoError::IncompatibleKeys => guest_types::CryptoErrno::IncompatibleKeys,
            CryptoError::Expired => guest_types::CryptoErrno::Expired,
        }
    }
}

impl From<TryFromIntError> for CryptoError {
    fn from(_: TryFromIntError) -> Self {
        CryptoError::Overflow
    }
}

impl From<TryFromIntError> for guest_types::CryptoErrno {
    fn from(_: TryFromIntError) -> Self {
        CryptoError::Overflow.into()
    }
}

#[macro_export]
macro_rules! ensure {
    ($cond:expr, $err:expr $(,)?) => {
        if !$cond {
            return Err($err);
        }
    };
    ($cond:expr, $fmt:expr, $($arg:tt)*) => {
        if !$cond {
            return Err($fmt, $($arg)*);
        }
    };
}

#[macro_export]
macro_rules! bail {
    ($err:expr $(,)?) => {
        return Err($err);
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err($fmt, $($arg)*);
    };
}

pub use {bail, ensure};

impl From<CryptoError> for i32 {
    fn from(e: CryptoError) -> Self {
        guest_types::CryptoErrno::from(e).into()
    }
}

impl<'a> wiggle::GuestErrorType for guest_types::CryptoErrno {
    fn success() -> Self {
        guest_types::CryptoErrno::Success
    }
}

impl guest_types::GuestErrorConversion for WasiCryptoCtx {
    fn into_crypto_errno(&self, e: wiggle::GuestError) -> guest_types::CryptoErrno {
        eprintln!("GuestError (witx) {:?}", e);
        guest_types::CryptoErrno::GuestError
    }
}

impl From<wiggle::GuestError> for guest_types::CryptoErrno {
    fn from(e: wiggle::GuestError) -> Self {
        eprintln!("GuestError (impl) {:?}", e);
        guest_types::CryptoErrno::GuestError
    }
}
