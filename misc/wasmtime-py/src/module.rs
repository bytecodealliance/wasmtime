//! WebAssembly Module API object.

use pyo3::prelude::*;

use std::rc::Rc;

#[pyclass]
pub struct Module {
    pub module: Rc<wasmtime_environ::Module>,
}
