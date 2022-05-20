//! Support for the component model in Wasmtime.
//!
//! This module contains all of the internal type definitions used by Wasmtime
//! to process the component model. Despite everything being `pub` here this is
//! not the public interface of Wasmtime to the component model. Instead this is
//! the internal support to mirror the core wasm support that Wasmtime already
//! implements.
//!
//! Some main items contained within here are:
//!
//! * Type hierarchy information for the component model
//! * Translation of a component into Wasmtime's representation
//! * Type information about a component used at runtime
//!
//! This module also contains a lot of Serialize/Deserialize types which are
//! encoded in the final compiled image for a component.
//!
//! Note that this entire module is gated behind the `component-model` Cargo
//! feature.
//!
//! ## Warning: In-progress
//!
//! As-of the time of this writing this module is incomplete and under
//! development. It will be added to incrementally over time as more features
//! are implemented. Current design decisions are also susceptible to change at
//! any time. Some comments may reflect historical rather than current state as
//! well (sorry).

mod info;
mod translate;
mod types;
pub use self::info::*;
pub use self::translate::*;
pub use self::types::*;
