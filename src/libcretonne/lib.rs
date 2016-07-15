
// ====------------------------------------------------------------------------------------==== //
//
// Cretonne code generation library.
//
// ====------------------------------------------------------------------------------------==== //

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub mod types;
pub mod condcodes;
pub mod immediates;
pub mod entities;
pub mod instructions;
pub mod repr;
pub mod write;
pub mod cfg;

pub mod entity_map;
