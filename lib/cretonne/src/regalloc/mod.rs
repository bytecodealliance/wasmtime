//! Register allocation.
//!
//! This module contains data structures and algorithms used for register allocation.

pub mod allocatable_set;
pub mod coloring;
pub mod live_value_tracker;
pub mod liveness;
pub mod liverange;
pub mod virtregs;

mod affinity;
mod coalescing;
mod context;
mod diversion;
mod pressure;
mod reload;
mod solver;
mod spilling;

pub use self::allocatable_set::AllocatableSet;
pub use self::context::Context;
pub use self::diversion::RegDiversions;
