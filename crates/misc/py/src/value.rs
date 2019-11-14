//! Utility functions to handle values conversion between abstractions/targets.

use pyo3::exceptions::Exception;
use pyo3::prelude::*;
use pyo3::types::PyAny;
use wasmtime_interface_types::Value;

pub fn pyobj_to_value(_: Python, p: &PyAny) -> PyResult<Value> {
    if let Ok(n) = p.extract() {
        Ok(Value::I32(n))
    } else if let Ok(n) = p.extract() {
        Ok(Value::U32(n))
    } else if let Ok(n) = p.extract() {
        Ok(Value::I64(n))
    } else if let Ok(n) = p.extract() {
        Ok(Value::U64(n))
    } else if let Ok(n) = p.extract() {
        Ok(Value::F64(n))
    } else if let Ok(n) = p.extract() {
        Ok(Value::F32(n))
    } else if let Ok(s) = p.extract() {
        Ok(Value::String(s))
    } else {
        Err(PyErr::new::<Exception, _>("unsupported value type"))
    }
}

pub fn value_to_pyobj(py: Python, value: Value) -> PyResult<PyObject> {
    Ok(match value {
        Value::I32(i) => i.into_py(py),
        Value::U32(i) => i.into_py(py),
        Value::I64(i) => i.into_py(py),
        Value::U64(i) => i.into_py(py),
        Value::F32(i) => i.into_py(py),
        Value::F64(i) => i.into_py(py),
        Value::String(i) => i.into_py(py),
    })
}
