use crate::Region;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GuestError {
    #[error("Invalid enum value {0}")]
    InvalidEnumValue(&'static str),
    #[error("Out of bounds: {0:?}")]
    PtrOutOfBounds(Region),
    #[error("Borrowed: {0:?}")]
    PtrBorrowed(Region),
}
