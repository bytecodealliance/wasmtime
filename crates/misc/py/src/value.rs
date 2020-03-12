//! Utility functions to handle values conversion between abstractions/targets.

use pyo3::exceptions::Exception;
use pyo3::prelude::*;
use pyo3::types::PyAny;
use wasmtime::Val;

pub fn pyobj_to_value(_: Python, p: &PyAny) -> PyResult<Val> {
    if let Ok(n) = p.extract() {
        Ok(Val::I32(n))
    } else if let Ok(n) = p.extract() {
        Ok(Val::I64(n))
    } else if let Ok(n) = p.extract() {
        Ok(Val::F64(n))
    } else if let Ok(n) = p.extract() {
        Ok(Val::F32(n))
    } else {
        Err(PyErr::new::<Exception, _>("unsupported value type"))
    }
}

pub fn value_to_pyobj(py: Python, value: Val) -> PyResult<PyObject> {
    Ok(match value {
        Val::I32(i) => i.into_py(py),
        Val::I64(i) => i.into_py(py),
        Val::F32(i) => i.into_py(py),
        Val::F64(i) => i.into_py(py),
        Val::AnyRef(_) | Val::FuncRef(_) | Val::V128(_) => {
            return Err(PyErr::new::<Exception, _>("unsupported value type"))
        }
    })
}
