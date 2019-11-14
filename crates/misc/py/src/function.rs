//! Support for a calling of a bounds (exported) function.

extern crate alloc;

use crate::value::{pyobj_to_value, value_to_pyobj};
use alloc::rc::Rc;
use pyo3::exceptions::Exception;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyTuple};
use wasmtime_api as api;
use wasmtime_interface_types::ModuleData;

// TODO support non-export functions
#[pyclass]
pub struct Function {
    pub instance: api::HostRef<api::Instance>,
    pub export_name: String,
    pub args_types: Vec<api::ValType>,
    pub data: Rc<ModuleData>,
}

impl Function {
    pub fn func(&self) -> api::HostRef<api::Func> {
        let e = self
            .instance
            .borrow()
            .find_export_by_name(&self.export_name)
            .expect("named export")
            .clone();
        e.func().expect("function export").clone()
    }
}

#[pymethods]
impl Function {
    #[__call__]
    #[args(args = "*")]
    fn call(&self, py: Python, args: &PyTuple) -> PyResult<PyObject> {
        let mut runtime_args = Vec::new();
        for item in args.iter() {
            runtime_args.push(pyobj_to_value(py, item)?);
        }
        let results = self
            .data
            .invoke_export(&self.instance, self.export_name.as_str(), &runtime_args)
            .map_err(crate::err2py)?;
        let mut py_results = Vec::new();
        for result in results {
            py_results.push(value_to_pyobj(py, result)?);
        }
        if py_results.len() == 1 {
            Ok(py_results[0].clone_ref(py))
        } else {
            Ok(PyTuple::new(py, py_results).to_object(py))
        }
    }
}

fn parse_annotation_type(s: &str) -> api::ValType {
    match s {
        "I32" | "i32" => api::ValType::I32,
        "I64" | "i64" => api::ValType::I64,
        "F32" | "f32" => api::ValType::F32,
        "F64" | "f64" => api::ValType::F64,
        _ => panic!("unknown type in annotations"),
    }
}

struct WrappedFn {
    func: PyObject,
    returns_types: Vec<api::ValType>,
}

impl WrappedFn {
    pub fn new(func: PyObject, returns_types: Vec<api::ValType>) -> Self {
        WrappedFn {
            func,
            returns_types,
        }
    }
}

impl api::Callable for WrappedFn {
    fn call(
        &self,
        params: &[api::Val],
        returns: &mut [api::Val],
    ) -> Result<(), api::HostRef<api::Trap>> {
        let gil = Python::acquire_gil();
        let py = gil.python();

        let params = params
            .iter()
            .map(|p| match p {
                api::Val::I32(i) => i.clone().into_py(py),
                api::Val::I64(i) => i.clone().into_py(py),
                _ => {
                    panic!();
                }
            })
            .collect::<Vec<PyObject>>();

        let result = self
            .func
            .call(py, PyTuple::new(py, params), None)
            .expect("TODO: convert result to trap");

        let result = if let Ok(t) = result.cast_as::<PyTuple>(py) {
            t
        } else {
            if result.is_none() {
                PyTuple::empty(py)
            } else {
                PyTuple::new(py, &[result])
            }
        };
        for (i, ty) in self.returns_types.iter().enumerate() {
            let result_item = result.get_item(i);
            returns[i] = match ty {
                api::ValType::I32 => api::Val::I32(result_item.extract::<i32>().unwrap()),
                api::ValType::I64 => api::Val::I64(result_item.extract::<i64>().unwrap()),
                _ => {
                    panic!();
                }
            };
        }
        Ok(())
    }
}

pub fn wrap_into_pyfunction(
    store: &api::HostRef<api::Store>,
    callable: &PyAny,
) -> PyResult<api::HostRef<api::Func>> {
    if !callable.hasattr("__annotations__")? {
        // TODO support calls without annotations?
        return Err(PyErr::new::<Exception, _>(
            "import is not a function".to_string(),
        ));
    }

    let annot = callable.getattr("__annotations__")?.cast_as::<PyDict>()?;
    let mut params = Vec::new();
    let mut returns = Vec::new();
    for (name, value) in annot.iter() {
        let ty = parse_annotation_type(&value.to_string());
        match name.to_string().as_str() {
            "return" => returns.push(ty),
            _ => params.push(ty),
        }
    }

    let ft = api::FuncType::new(
        params.into_boxed_slice(),
        returns.clone().into_boxed_slice(),
    );

    let gil = Python::acquire_gil();
    let wrapped = WrappedFn::new(callable.to_object(gil.python()), returns);
    let f = api::Func::new(store, ft, Rc::new(wrapped));
    Ok(api::HostRef::new(f))
}
