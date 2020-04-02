//! Adds support for profiling jitted code using VTune Amplifier
//!
//! Build:
//!     cargo build --features=vtune
//! Profile:
//!     amplxe-cl -run-pass-thru=--no-altstack -v -collect hotspots target/debug/wasmtime --vtune test.wasm
//!
//! Note: amplxe-cl is a command-line tool for Vtune which should be installed.

use crate::ProfilingAgent;
use anyhow::Result;
use core::ptr;
use ittapi_rs::*;
use std::collections::HashMap;
use std::ffi::CString;
use std::sync::Mutex;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::wasm::DefinedFuncIndex;
use wasmtime_environ::Module;
use wasmtime_runtime::VMFunctionBody;

/// Interface for driving the ittapi for VTune support
pub struct VTuneAgent {
    // Note that we use a mutex internally to serialize state updates
    // since multiple threads may be sharing this agent.
    state: Mutex<State>,
}

/// Interface for driving vtune
#[derive(Clone, Debug, Default)]
struct State {
    /// Unique identifier for the jitted function
    method_id: HashMap<(usize, DefinedFuncIndex), u32>,
}

impl VTuneAgent {
    /// Intialize a VTuneAgent and write out the header
    pub fn new() -> Result<Self> {
        let state = State {
            method_id: HashMap::new(),
        };
        Ok(VTuneAgent {
            state: Mutex::new(state),
        })
    }
}

impl Drop for VTuneAgent {
    fn drop(&mut self) {
        self.state.lock().unwrap().event_shutdown();
    }
}

impl State {
    /// Return the unique method ID for use with the ittapi
    pub fn get_method_id(&mut self, module_id: usize, func_idx: DefinedFuncIndex) -> u32 {
        let method_id: u32;
        unsafe {
            method_id = iJIT_GetNewMethodID();
        }
        assert_eq!(
            self.method_id.insert((module_id, func_idx), method_id),
            None
        );
        method_id
    }

    /// Load module
    pub fn event_load(
        &mut self,
        method_id: u32,
        filename: &str,
        module_name: &str,
        method_name: &str,
        addr: *const u8,
        len: usize,
    ) -> () {
        let mut jmethod = _iJIT_Method_Load {
            method_id: method_id,
            method_name: CString::new(method_name)
                .expect("CString::new failed")
                .into_raw(),
            method_load_address: addr as *mut ::std::os::raw::c_void,
            method_size: len as u32,
            line_number_size: 0,
            line_number_table: ptr::null_mut(),
            class_id: 0,
            class_file_name: CString::new(module_name)
                .expect("CString::new failed")
                .into_raw(),
            source_file_name: CString::new(filename)
                .expect("CString::new failed")
                .into_raw(),
        };
        let jmethod_ptr = &mut jmethod as *mut _ as *mut _;
        unsafe {
            println!("EventLoad: NotifyEvent Called {}", method_id);
            let _ret = iJIT_NotifyEvent(
                iJIT_jvm_event_iJVM_EVENT_TYPE_METHOD_LOAD_FINISHED,
                jmethod_ptr as *mut ::std::os::raw::c_void,
            );
        }
    }

    /// Shutdown module
    fn event_shutdown(&mut self) -> () {
        unsafe {
            println!("Drop was called!!!!!!\n");
            let _ret = iJIT_NotifyEvent(iJIT_jvm_event_iJVM_EVENT_TYPE_SHUTDOWN, ptr::null_mut());
        }
    }
}

impl ProfilingAgent for VTuneAgent {
    fn module_load(
        &self,
        module: &Module,
        functions: &PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
        dbg_image: Option<&[u8]>,
    ) {
        self.state
            .lock()
            .unwrap()
            .module_load(module, functions, dbg_image);
    }
}

impl State {
    fn module_load(
        &mut self,
        module: &Module,
        functions: &PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
        _dbg_image: Option<&[u8]>,
    ) -> () {
        for (idx, func) in functions.iter() {
            let (addr, len) = unsafe { ((**func).as_ptr() as *const u8, (**func).len()) };
            let default_filename = "wasm_file";
            let default_module_name = String::from("wasm_module");
            let module_name = module.name.as_ref().unwrap_or(&default_module_name);
            let method_name = super::debug_name(module, idx);
            let method_id = self.get_method_id(module.id, idx);
            println!(
                "Event Load: ({}) {:?}::{:?} Addr:{:?}\n",
                method_id, module_name, method_name, addr
            );
            self.event_load(
                method_id,
                default_filename,
                module_name,
                &method_name,
                addr,
                len,
            );
        }
    }
}
