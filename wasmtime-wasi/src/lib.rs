extern crate alloc;

mod instantiate;
mod syscalls;

pub use instantiate::{instantiate_wasi, instantiate_wasi_with_context};
