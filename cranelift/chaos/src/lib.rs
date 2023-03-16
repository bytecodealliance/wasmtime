//! # Cranelift Chaos Mode
//!
//! Chaos mode is a compilation feature intended to be turned on for certain
//! fuzz targets. When the feature is turned off - as is normally the case -
//! [ChaosEngine] will be a zero-sized type and optimized away.
//!
//! While the feature is turned on, the struct [ChaosEngine]
//! provides functionality to tap into pseudo-randomness at specific locations
//! in the code. It may be used for targeted fuzzing of compiler internals,
//! e.g. manipulate heuristic optimizations, clobber undefined register bits
//! etc.
//!
//! There are three ways to acquire a [ChaosEngine]:
//! - [arbitrary] for the real deal
//! - [noop] for the zero-sized type when `chaos` is disabled
//! - [todo] for stubbing out code paths during development
//!
//! The reason both [noop] and [todo] exist is so that [todo] can easily
//! be searched for and removed later.
//!
//! [arbitrary]: ChaosEngine#method.arbitrary
//! [noop]: ChaosEngine::noop
//! [todo]: ChaosEngine::todo

#[cfg(not(any(feature = "chaos", doc)))]
mod disabled;
#[cfg(not(any(feature = "chaos", doc)))]
pub use disabled::*;

#[cfg(any(feature = "chaos", doc))]
mod enabled;
#[cfg(any(feature = "chaos", doc))]
pub use enabled::*;
