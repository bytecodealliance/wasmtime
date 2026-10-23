//! GC ops generator: split from the previous monolithic file.

pub mod limits;
pub mod mutator;
pub mod ops;
pub mod stack;
pub mod types;

#[cfg(test)]
mod tests;
