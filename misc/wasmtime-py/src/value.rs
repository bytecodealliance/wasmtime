//! Utility functions to handle values conversion between abstractions/targets.

use pyo3::exceptions::Exception;
use pyo3::prelude::*;
use pyo3::types::PyAny;

use core::ptr;
use cranelift_codegen::ir;
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

pub unsafe fn read_value_from(py: Python, ptr: *mut i64, ty: ir::Type) -> PyObject {
    match ty {
        ir::types::I32 => ptr::read(ptr as *const i32).into_py(py),
        ir::types::I64 => ptr::read(ptr as *const i64).into_py(py),
        ir::types::F32 => ptr::read(ptr as *const f32).into_py(py),
        ir::types::F64 => ptr::read(ptr as *const f64).into_py(py),
        _ => panic!("TODO add PyResult to read_value_from"),
    }
}

pub unsafe fn write_value_to(py: Python, ptr: *mut i64, ty: ir::Type, val: PyObject) {
    match ty {
        ir::types::I32 => ptr::write(ptr as *mut i32, val.extract::<i32>(py).expect("i32")),
        ir::types::I64 => ptr::write(ptr as *mut i64, val.extract::<i64>(py).expect("i64")),
        ir::types::F32 => ptr::write(ptr as *mut f32, val.extract::<f32>(py).expect("f32")),
        ir::types::F64 => ptr::write(ptr as *mut f64, val.extract::<f64>(py).expect("f64")),
        _ => panic!("TODO add PyResult to write_value_to"),
    }
}
