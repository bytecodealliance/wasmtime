//! The actual implementation of garbage collection, for when the `gc` Cargo
//! feature is enabled.

mod anyref;
mod externref;
mod i31;
mod rooting;
mod structref;

pub use anyref::*;
pub use externref::*;
pub use i31::*;
pub use rooting::*;
pub use structref::*;
