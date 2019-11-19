//! WebAssembly Module API object.

use pyo3::prelude::*;
use wasmtime as api;

#[pyclass]
pub struct Module {
    pub module: api::HostRef<api::Module>,
}
