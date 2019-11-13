#![allow(improper_ctypes)]

use crate::import::into_instance_from_obj;
use crate::instance::Instance;
use crate::memory::Memory;
use crate::module::Module;
use core::cell::RefCell;
use pyo3::exceptions::Exception;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PySet};
use pyo3::wrap_pyfunction;
use std::rc::Rc;
use wasmtime_interface_types::ModuleData;
use wasmtime_jit::Features;

mod function;
mod import;
mod instance;
mod memory;
mod module;
mod value;

fn err2py(err: anyhow::Error) -> PyErr {
    PyErr::new::<Exception, _>(format!("{:?}", err))
}

#[pyclass]
pub struct InstantiateResultObject {
    instance: Py<Instance>,
    module: Py<Module>,
}

#[pymethods]
impl InstantiateResultObject {
    #[getter(instance)]
    fn get_instance(&self) -> PyResult<Py<Instance>> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        Ok(self.instance.clone_ref(py))
    }

    #[getter(module)]
    fn get_module(&self) -> PyResult<Py<Module>> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        Ok(self.module.clone_ref(py))
    }
}

/// WebAssembly instantiate API method.
#[pyfunction]
pub fn instantiate(
    py: Python,
    buffer_source: &PyBytes,
    import_obj: &PyDict,
) -> PyResult<Py<InstantiateResultObject>> {
    let wasm_data = buffer_source.as_bytes();

    let generate_debug_info = false;

    let isa = {
        let isa_builder = cranelift_native::builder().map_err(|s| PyErr::new::<Exception, _>(s))?;
        let flag_builder = cranelift_codegen::settings::builder();
        isa_builder.finish(cranelift_codegen::settings::Flags::new(flag_builder))
    };

    let mut context = wasmtime_jit::Context::with_isa(isa, wasmtime_jit::CompilationStrategy::Auto)
        .with_features(Features {
            multi_value: true,
            ..Features::default()
        });
    context.set_debug_info(generate_debug_info);
    let global_exports = context.get_global_exports();

    for (name, obj) in import_obj.iter() {
        context.name_instance(
            name.to_string(),
            into_instance_from_obj(py, global_exports.clone(), obj)?,
        )
    }

    let data = Rc::new(ModuleData::new(wasm_data).map_err(err2py)?);

    // If this module expects to be able to use wasi then go ahead and hook
    // that up into the imported crates.
    if let Some(module_name) = data.find_wasi_module_name() {
        let wasi_handle =
            wasmtime_wasi::instantiate_wasi("", context.get_global_exports(), &[], &[], &[])
                .map_err(|e| err2py(e.into()))?;
        context.name_instance(module_name, wasi_handle);
    }
    let instance = context
        .instantiate_module(None, wasm_data)
        .map_err(|e| err2py(e.into()))?;

    let module = Py::new(
        py,
        Module {
            module: instance.module(),
        },
    )?;

    let instance = Py::new(
        py,
        Instance {
            context: Rc::new(RefCell::new(context)),
            instance,
            data,
        },
    )?;

    Py::new(py, InstantiateResultObject { instance, module })
}

#[pyfunction]
pub fn imported_modules<'p>(py: Python<'p>, buffer_source: &PyBytes) -> PyResult<&'p PyDict> {
    let wasm_data = buffer_source.as_bytes();
    let dict = PyDict::new(py);
    // TODO: error handling
    let mut parser = wasmparser::ModuleReader::new(wasm_data).unwrap();
    while !parser.eof() {
        let section = parser.read().unwrap();
        match section.code {
            wasmparser::SectionCode::Import => {}
            _ => continue,
        };
        let reader = section.get_import_section_reader().unwrap();
        for import in reader {
            let import = import.unwrap();
            // Skip over wasi-looking imports since those aren't imported from
            // Python but rather they're implemented natively.
            if wasmtime_wasi::is_wasi_module(import.module) {
                continue;
            }
            let set = match dict.get_item(import.module) {
                Some(set) => set.downcast_ref::<PySet>().unwrap(),
                None => {
                    let set = PySet::new::<PyObject>(py, &[])?;
                    dict.set_item(import.module, set)?;
                    set
                }
            };
            set.add(import.field)?;
        }
    }
    Ok(dict)
}

#[pymodule]
fn lib_wasmtime(_: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Instance>()?;
    m.add_class::<Memory>()?;
    m.add_class::<Module>()?;
    m.add_class::<InstantiateResultObject>()?;
    m.add_wrapped(wrap_pyfunction!(instantiate))?;
    m.add_wrapped(wrap_pyfunction!(imported_modules))?;
    Ok(())
}
