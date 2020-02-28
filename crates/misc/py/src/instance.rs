//! WebAssembly Instance API object.

use crate::function::Function;
use crate::memory::Memory;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::rc::Rc;
use wasmtime_interface_types::ModuleData;

#[pyclass]
pub struct Instance {
    pub instance: wasmtime::Instance,
    pub data: Rc<ModuleData>,
}

#[pymethods]
impl Instance {
    #[getter(exports)]
    fn get_exports(&mut self) -> PyResult<PyObject> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let exports = PyDict::new(py);
        let module = self.instance.module().clone();
        for (i, e) in module.exports().iter().enumerate() {
            match e.ty() {
                wasmtime::ExternType::Func(ft) => {
                    let mut args_types = Vec::new();
                    for ty in ft.params().iter() {
                        args_types.push(ty.clone());
                    }
                    let f = Py::new(
                        py,
                        Function {
                            instance: self.instance.clone(),
                            data: self.data.clone(),
                            export_name: e.name().to_string(),
                            args_types,
                        },
                    )?;
                    exports.set_item(e.name().to_string(), f)?;
                }
                wasmtime::ExternType::Memory(_) => {
                    let f = Py::new(
                        py,
                        Memory {
                            memory: self.instance.exports()[i].memory().unwrap().clone(),
                        },
                    )?;
                    exports.set_item(e.name().to_string(), f)?;
                }
                _ => {
                    // Skip unknown export type.
                    continue;
                }
            }
        }

        Ok(exports.to_object(py))
    }
}
