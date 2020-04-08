//! Support for FDE data generation.
use thiserror::Error;

/// Enumerate the errors possible in mapping Cranelift registers to their DWARF equivalent.
#[allow(missing_docs)]
#[derive(Error, Debug)]
pub enum RegisterMappingError {
    #[error("unable to find bank for register info")]
    MissingBank,
    #[error("register mapping is currently only implemented for x86_64")]
    UnsupportedArchitecture,
    #[error("unsupported register bank: {0}")]
    UnsupportedRegisterBank(&'static str),
}
