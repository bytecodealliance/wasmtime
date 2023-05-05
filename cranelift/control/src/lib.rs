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
//! There are two ways to acquire a [ControlPlane]:
//! - [arbitrary] for the real deal
//! - [default] for an "empty" control plane which always returns default
//!   values
//!
//! ## Fuel Limit
//!
//! Controls the number of mutations or optimizations that the compiler will
//! perform before stopping.
//!
//! When a perturbation introduced by chaos mode triggers a bug, it may not be
//! immediately clear which of the introduced perturbations was the trigger. The
//! fuel limit can then be used to binary-search for the trigger. It limits the
//! number of perturbations introduced by the control plane. The fuel limit will
//! typically be set with a command line argument passed to a fuzz target. For
//! example:
//! ```sh
//! cargo fuzz run --features chaos $TARGET -- --fuel=16
//! ```
//!
//! [arbitrary]: ControlPlane#method.arbitrary
//! [default]: ControlPlane#method.default

#[cfg(not(feature = "chaos"))]
mod zero_sized;
#[cfg(not(feature = "chaos"))]
pub use zero_sized::*;

#[cfg(feature = "chaos")]
mod chaos;
#[cfg(feature = "chaos")]
pub use chaos::*;
