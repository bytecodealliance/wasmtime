extern crate alloc;

mod instantiate;
mod syscalls;
mod r#trait;
mod trait_impl;

#[macro_use]
extern crate wasmtime_bindings_macro;
pub use instantiate::{instantiate_wasi, instantiate_wasi_with_context};
pub use r#trait::{wasi_mod, Wasi, WasiMem};
pub use trait_impl::instantiate_wasi2;
