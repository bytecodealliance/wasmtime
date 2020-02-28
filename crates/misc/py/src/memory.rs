//! WebAssembly Memory API object.

use pyo3::class::PyBufferProtocol;
use pyo3::exceptions::BufferError;
use pyo3::ffi;
use pyo3::prelude::*;
use std::ffi::CStr;
use std::os::raw::{c_int, c_void};
use std::ptr;

#[pyclass]
pub struct Memory {
    pub memory: wasmtime::Memory,
}

#[pymethods]
impl Memory {
    #[getter(current)]
    pub fn current(&self) -> u32 {
        self.memory.size()
    }

    pub fn grow(&self, _number: u32) -> u32 {
        (-1i32) as u32
    }
}

#[pyproto]
impl PyBufferProtocol for Memory {
    fn bf_getbuffer(&self, view: *mut ffi::Py_buffer, flags: c_int) -> PyResult<()> {
        if view.is_null() {
            return Err(BufferError::py_err("View is null"));
        }

        unsafe {
            /*
            As a special case, for temporary buffers that are wrapped by
            PyMemoryView_FromBuffer() or PyBuffer_FillInfo() this field is NULL.
            In general, exporting objects MUST NOT use this scheme.
                        */
            (*view).obj = ptr::null_mut();
        }

        let readonly = if (flags & ffi::PyBUF_WRITABLE) == ffi::PyBUF_WRITABLE {
            0
        } else {
            1
        };

        unsafe {
            let base = self.memory.data_ptr();
            let current_length = self.memory.data_size();

            (*view).buf = base as *mut c_void;
            (*view).len = current_length as isize;
            (*view).readonly = readonly;
            (*view).itemsize = 1;

            (*view).format = ptr::null_mut();
            if (flags & ffi::PyBUF_FORMAT) == ffi::PyBUF_FORMAT {
                let msg = CStr::from_bytes_with_nul(b"B\0").unwrap();
                (*view).format = msg.as_ptr() as *mut _;
            }

            (*view).ndim = 1;
            (*view).shape = ptr::null_mut();
            if (flags & ffi::PyBUF_ND) == ffi::PyBUF_ND {
                (*view).shape = (&((*view).len)) as *const _ as *mut _;
            }

            (*view).strides = ptr::null_mut();
            if (flags & ffi::PyBUF_STRIDES) == ffi::PyBUF_STRIDES {
                (*view).strides = &((*view).itemsize) as *const _ as *mut _;
            }

            (*view).suboffsets = ptr::null_mut();
            (*view).internal = ptr::null_mut();
        }

        Ok(())
    }
}
