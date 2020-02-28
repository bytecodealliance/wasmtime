//! WebAssembly Module API object.

use pyo3::prelude::*;

#[pyclass]
pub struct Module {
    pub module: wasmtime::Module,
}
