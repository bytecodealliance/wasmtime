use crate::Region;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GuestError {
    #[error("Invalid enum value {0}")]
    InvalidEnumValue(&'static str),
    #[error("Pointer out of bounds: {0:?}")]
    PtrOutOfBounds(Region),
    #[error("Pointer not aligned to {1}: {0:?}")]
    PtrNotAligned(Region, u32),
    #[error("Pointer already borrowed: {0:?}")]
    PtrBorrowed(Region),
}
