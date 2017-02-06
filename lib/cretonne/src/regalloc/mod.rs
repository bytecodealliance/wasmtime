//! Register allocation.
//!
//! This module contains data structures and algorithms used for register allocation.

pub mod liverange;
pub mod liveness;
pub mod allocatable_set;
pub mod live_value_tracker;

mod affinity;
