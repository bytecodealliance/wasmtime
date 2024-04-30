//! Implementation of Wasmtime's system primitives for Unix-like operating
//! systems.
//!
//! This module handles Linux and macOS for example.

pub mod mmap;
pub mod unwind;
pub mod vm;

pub mod signals;

cfg_if::cfg_if! {
    if #[cfg(target_os = "macos")] {
        pub mod machports;

        pub mod macos_traphandlers;
        pub use macos_traphandlers as traphandlers;
    } else {
        pub use signals as traphandlers;
    }
}
