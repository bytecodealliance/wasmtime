
// ====------------------------------------------------------------------------------------==== //
//
// Cretonne code generation library.
//
// ====------------------------------------------------------------------------------------==== //

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub mod ir;
pub mod write;
pub mod cfg;

pub mod entity_map;

#[cfg(test)]pub mod test_utils;
