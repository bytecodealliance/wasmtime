//! Support for a calling of a bounds (exported) function.

extern crate alloc;

use pyo3::prelude::*;
use pyo3::types::PyTuple;

use crate::value::{pyobj_to_value, value_to_pyobj};
use alloc::rc::Rc;
use core::cell::RefCell;

use cranelift_codegen::ir;
use wasmtime_interface_types::ModuleData;
use wasmtime_jit::{Context, InstanceHandle};
use wasmtime_runtime::Export;

// TODO support non-export functions
#[pyclass]
pub struct Function {
    pub context: Rc<RefCell<Context>>,
    pub instance: InstanceHandle,
    pub export_name: String,
    pub args_types: Vec<ir::Type>,
    pub data: Rc<ModuleData>,
}

impl Function {
    pub fn get_signature(&self) -> ir::Signature {
        let mut instance = self.instance.clone();
        if let Some(Export::Function { signature, .. }) = instance.lookup(&self.export_name) {
            signature
        } else {
            panic!()
        }
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
        let mut instance = self.instance.clone();
        let mut cx = self.context.borrow_mut();
        let results = self
            .data
            .invoke(
                &mut cx,
                &mut instance,
                self.export_name.as_str(),
                &runtime_args,
            )
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
