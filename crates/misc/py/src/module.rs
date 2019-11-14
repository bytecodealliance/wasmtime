//! WebAssembly Module API object.

extern crate alloc;

use pyo3::prelude::*;
use wasmtime_api as api;

#[pyclass]
pub struct Module {
    pub module: api::HostRef<api::Module>,
}
