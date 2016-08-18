
// ====------------------------------------------------------------------------------------==== //
//
// Cretonne code generation library.
//
// ====------------------------------------------------------------------------------------==== //

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub mod ir;
pub mod isa;
pub mod write;
pub mod cfg;
pub mod dominator_tree;
pub mod entity_map;
pub mod settings;

mod constant_hash;

#[cfg(test)]pub mod test_utils;
