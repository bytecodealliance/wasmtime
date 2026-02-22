//! > **⚠️ Warning ⚠️**: this crate is an internal-only crate for the Wasmtime
//! > project and is not intended for general use. APIs are not strictly
//! > reviewed for safety and usage outside of Wasmtime may have bugs. If
//! > you're interested in using this feel free to file an issue on the
//! > Wasmtime repository to start a discussion about doing so, but otherwise
//! > be aware that your usage of this crate is not supported.

#![no_std]
#![deny(missing_docs)]
#![cfg_attr(arc_try_new, allow(unstable_features))]
#![cfg_attr(arc_try_new, feature(allocator_api))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(test(attr(deny(warnings))))]
#![doc(test(attr(allow(dead_code, unused_variables, unused_mut))))]

extern crate alloc as std_alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod alloc;
pub mod error;
pub mod math;
pub mod non_max;
pub mod slab;
pub mod undo;
