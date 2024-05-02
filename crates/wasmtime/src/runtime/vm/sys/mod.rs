//! OS-related abstractions required by Wasmtime.
//!
//! This module is intended to house all logic that's specific to either Unix
//! or Windows, for example. The goal of this module is to be the "single
//! module" to edit if Wasmtime is ported to a new platform. Ideally all that's
//! needed is an extra block below and a new platform should be good to go after
//! filling out the implementation.

#![allow(clippy::cast_sign_loss)] // platforms too fiddly to worry about this

cfg_if::cfg_if! {
    if #[cfg(miri)] {
        mod miri;
        pub use miri::*;
    } else if #[cfg(windows)] {
        mod windows;
        pub use windows::*;
    } else if #[cfg(unix)] {
        mod unix;
        pub use unix::*;
    } else {
        mod custom;
        pub use custom::*;
    }
}
