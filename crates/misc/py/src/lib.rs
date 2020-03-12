use crate::function::{wrap_into_pyfunction, Function};
use crate::instance::Instance;
use crate::memory::Memory;
use crate::module::Module;
use pyo3::exceptions::Exception;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes, PyDict, PySet};
use pyo3::wrap_pyfunction;

mod function;
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

fn find_export_in(obj: &PyAny, store: &wasmtime::Store, name: &str) -> PyResult<wasmtime::Extern> {
    let obj = obj.cast_as::<PyDict>()?;

    Ok(if let Some(item) = obj.get_item(name) {
        if item.is_callable() {
            if item.get_type().is_subclass::<Function>()? {
                let wasm_fn = item.cast_as::<Function>()?;
                wasm_fn.func().into()
            } else {
                wrap_into_pyfunction(store, item)?.into()
            }
        } else if item.get_type().is_subclass::<Memory>()? {
            let wasm_mem = item.cast_as::<Memory>()?;
            wasm_mem.memory.clone().into()
        } else {
            return Err(PyErr::new::<Exception, _>(format!(
                "unsupported import type {}",
                name
            )));
        }
    } else {
        return Err(PyErr::new::<Exception, _>(format!(
            "import {} is not found",
            name
        )));
    })
}

/// WebAssembly instantiate API method.
#[pyfunction]
pub fn instantiate(
    py: Python,
    buffer_source: &PyBytes,
    import_obj: &PyDict,
) -> PyResult<Py<InstantiateResultObject>> {
    let wasm_data = buffer_source.as_bytes();

    let engine = wasmtime::Engine::new(&wasmtime::Config::new().wasm_multi_value(true));
    let store = wasmtime::Store::new(&engine);

    let module = wasmtime::Module::new(&store, wasm_data).map_err(err2py)?;

    // If this module expects to be able to use wasi then go ahead and hook
    // that up into the imported crates.
    let cx = wasmtime_wasi::WasiCtxBuilder::new()
        .build()
        .map_err(|e| err2py(e.into()))?;
    let wasi_snapshot_preview1 = wasmtime_wasi::Wasi::new(&store, cx);
    let cx = wasmtime_wasi::old::snapshot_0::WasiCtxBuilder::new()
        .build()
        .map_err(|e| err2py(e.into()))?;
    let wasi_snapshot = wasmtime_wasi::old::snapshot_0::Wasi::new(&store, cx);

    let mut imports: Vec<wasmtime::Extern> = Vec::new();
    for i in module.imports() {
        if i.module() == "wasi_snapshot" {
            if let Some(func) = wasi_snapshot.get_export(i.name()) {
                imports.push(func.clone().into());
                continue;
            }
        }
        if i.module() == "wasi_snapshot_preview1" {
            if let Some(func) = wasi_snapshot_preview1.get_export(i.name()) {
                imports.push(func.clone().into());
                continue;
            }
        }
        let module_name = i.module();
        if let Some(m) = import_obj.get_item(module_name) {
            let e = find_export_in(m, &store, i.name())?;
            imports.push(e);
        } else {
            return Err(PyErr::new::<Exception, _>(format!(
                "imported module {} is not found",
                module_name
            )));
        }
    }

    let instance = wasmtime::Instance::new(&module, &imports)
        .map_err(|t| PyErr::new::<Exception, _>(format!("instantiated with trap {:?}", t)))?;

    let module = Py::new(py, Module { module })?;

    let instance = Py::new(py, Instance { instance })?;

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
