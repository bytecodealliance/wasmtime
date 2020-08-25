//! A very experimental module modeled providing a high-level and safe
//! filesystem interface, modeled after `std::fs`, implemented on top of
//! WASI functions.
//!
//! Most functions in this API are not yet implemented!
//!
//! This corresponds to [`std::fs`].
//!
//! Instead of [`std::fs`'s free functions] which operate on paths, this
//! crate has methods on [`Dir`] which operate on paths which must be
//! relative to and within the directory.
//!
//! Since all functions which expose raw file descriptors are `unsafe`,
//! I/O handles in this API are unforgeable (unsafe code notwithstanding).
//! This combined with WASI's lack of absolute paths provides a natural
//! capability-oriented interface.
//!
//! [`std::fs`]: https://doc.rust-lang.org/std/fs/index.html
//! [`std::fs`'s free functions]: https://doc.rust-lang.org/std/fs/index.html#functions
//! [`DIR`]: struct.Dir.html

// TODO: When more things are implemented, remove these.
#![allow(
    unused_imports,
    unreachable_code,
    unused_variables,
    unused_mut,
    unused_unsafe,
    dead_code
)]

mod dir;
mod dir_builder;
mod dir_entry;
mod file;
mod file_type;
mod metadata;
mod open_options;
mod permissions;
mod readdir;

pub use crate::wasi::types::Fd;

pub use dir::*;
pub use dir_builder::*;
pub use dir_entry::*;
pub use file::*;
pub use file_type::*;
pub use metadata::*;
pub use open_options::*;
pub use permissions::*;
pub use readdir::*;
