//! Error type
use crate::Result;
use std::{ffi, io, num};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error {0}")]
    Io(#[from] io::Error),
    #[error("a nul byte was not found in the expected position")]
    Nul(#[from] ffi::NulError),
    #[error("integral type conversion failed")]
    IntConversion(#[from] num::TryFromIntError),
}

impl Error {
    pub fn from_success_code<T: IsZero>(t: T) -> Result<()> {
        if t.is_zero() {
            Ok(())
        } else {
            Err(Self::from(io::Error::last_os_error()))
        }
    }

    pub fn from_result<T: IsMinusOne>(t: T) -> Result<T> {
        if t.is_minus_one() {
            Err(Self::from(io::Error::last_os_error()))
        } else {
            Ok(t)
        }
    }
}

#[doc(hidden)]
pub trait IsZero {
    fn is_zero(&self) -> bool;
}

macro_rules! impl_is_zero {
    ($($t:ident)*) => ($(impl IsZero for $t {
        fn is_zero(&self) -> bool {
            *self == 0
        }
    })*)
}

impl_is_zero! { i32 i64 isize }

#[doc(hidden)]
pub trait IsMinusOne {
    fn is_minus_one(&self) -> bool;
}

macro_rules! impl_is_minus_one {
    ($($t:ident)*) => ($(impl IsMinusOne for $t {
        fn is_minus_one(&self) -> bool {
            *self == -1
        }
    })*)
}

impl_is_minus_one! { i32 i64 isize }
