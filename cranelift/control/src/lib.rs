//! # Cranelift Control
//!
//! This is the home of the control plane of chaos mode, a compilation feature
//! intended to be turned on for certain fuzz targets. When the feature is
//! turned off, as is normally the case, [ControlPlane] will be a zero-sized
//! type and optimized away.
//!
//! While the feature is turned on, the struct [ControlPlane]
//! provides functionality to tap into pseudo-randomness at specific locations
//! in the code. It may be used for targeted fuzzing of compiler internals,
//! e.g. manipulate heuristic optimizations, clobber undefined register bits
//! etc.
//!
//! There are three ways to acquire a [ControlPlane]:
//! - [arbitrary] for the real deal
//! - [noop] for the zero-sized type when `chaos` is disabled
//! - [todo] for stubbing out code paths during development
//!
//! The reason both [noop] and [todo] exist is so that [todo] can easily
//! be searched for and removed later.
//!
//! [arbitrary]: ControlPlane#method.arbitrary
//! [noop]: ControlPlane::noop
//! [todo]: ControlPlane::todo

#[cfg(not(any(feature = "chaos", doc)))]
mod zero_sized;
#[cfg(not(any(feature = "chaos", doc)))]
pub use zero_sized::*;

#[cfg(any(feature = "chaos", doc))]
mod chaos;
#[cfg(any(feature = "chaos", doc))]
pub use chaos::*;
