//! Adds support for profiling JIT-ed code using VTune.
//!
//! ### Build
//!
//! ```ignore
//! cargo build --features=vtune
//! ```
//!
//! ### Profile
//!
//! ```ignore
//! vtune -run-pass-thru=--no-altstack -v -collect hotspots target/debug/wasmtime --vtune test.wasm
//! ```
//!
//! Note: `vtune` is a command-line tool for VTune which must [be
//! installed](https://www.intel.com/content/www/us/en/developer/tools/oneapi/vtune-profiler.html#standalone)
//! for this to work.

use crate::{CompiledModule, ProfilingAgent};
use anyhow::Result;
use core::ptr;
use ittapi_rs::*;
use object::Object as _;
use std::collections::HashMap;
use std::ffi::CString;
use std::sync::{atomic, Mutex};
use wasmtime_environ::EntityRef;

/// Interface for driving the ittapi for VTune support
pub struct VTuneAgent {
    // Note that we use a mutex internally to serialize state updates
    // since multiple threads may be sharing this agent.
    state: Mutex<State>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
enum MethodKind {
    Func { module_id: usize, func_id: usize },
    ExportedFuncTrampoline { module_id: usize, func_id: usize },
}

/// Interface for driving vtune
#[derive(Clone, Debug, Default)]
struct State {
    /// Unique identifier for the jitted code region.
    method_id: HashMap<MethodKind, u32>,
}

impl VTuneAgent {
    /// Initialize a VTuneAgent.
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
    /// Return the unique method ID for use with the ittapi.
    fn get_method_id(&mut self, kind: MethodKind) -> u32 {
        let method_id = Self::get_raw_method_id();
        assert_eq!(self.method_id.insert(kind, method_id), None);
        method_id
    }

    /// Return a method ID for use with the ittapi, for code that has no unique identifier.
    fn get_raw_method_id() -> u32 {
        unsafe { iJIT_GetNewMethodID() }
    }

    /// Notify vtune about a newly tracked code region.
    fn event_load(
        &mut self,
        method_id: u32,
        filename: &str,
        module_name: &str,
        method_name: &str,
        addr: *const u8,
        len: usize,
    ) -> () {
        let mut jmethod = _iJIT_Method_Load {
            method_id,
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
            log::trace!(
                "NotifyEvent: method load (single method with id {})",
                method_id
            );
            let _ret = iJIT_NotifyEvent(
                iJIT_jvm_event_iJVM_EVENT_TYPE_METHOD_LOAD_FINISHED,
                jmethod_ptr as *mut ::std::os::raw::c_void,
            );
        }
    }

    /// Shutdown module
    fn event_shutdown(&mut self) -> () {
        unsafe {
            log::trace!("NotifyEvent shutdown (whole module)");
            let _ret = iJIT_NotifyEvent(iJIT_jvm_event_iJVM_EVENT_TYPE_SHUTDOWN, ptr::null_mut());
        }
    }
}

impl ProfilingAgent for VTuneAgent {
    fn module_load(&self, module: &CompiledModule, dbg_image: Option<&[u8]>) {
        self.state.lock().unwrap().module_load(module, dbg_image);
    }
    fn trampoline_load(&self, image: &object::File<'_>) {
        self.state.lock().unwrap().trampoline_load(image);
    }
}

impl State {
    fn module_load(&mut self, module: &CompiledModule, _dbg_image: Option<&[u8]>) -> () {
        // Global counter for module ids.
        static MODULE_ID: atomic::AtomicUsize = atomic::AtomicUsize::new(0);
        let global_module_id = MODULE_ID.fetch_add(1, atomic::Ordering::SeqCst);

        let default_filename = "wasm_file";
        let module_name = module
            .module()
            .name
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("wasm_module");

        for (idx, func) in module.finished_functions() {
            let (addr, len) = unsafe { ((*func).as_ptr() as *const u8, (*func).len()) };
            let method_name = super::debug_name(module.module(), idx);
            let method_id = self.get_method_id(MethodKind::Func {
                module_id: global_module_id,
                func_id: idx.index(),
            });
            log::trace!(
                "new function ({}) {:?}::{:?} @ {:?}\n",
                method_id,
                module_name,
                method_name,
                addr
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

        // Note: these are the trampolines into exported functions.
        for (idx, func, len) in module.trampolines() {
            let idx = idx.index();
            let (addr, len) = (func as usize as *const u8, len);
            let method_name = format!("wasm::trampoline[{}]", idx,);
            let method_id = self.get_method_id(MethodKind::ExportedFuncTrampoline {
                module_id: global_module_id,
                func_id: idx,
            });
            log::trace!(
                "new export trampoline ({}) for function {} @ {:?}\n",
                method_id,
                idx,
                addr
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

    fn trampoline_load(&mut self, image: &object::File<'_>) {
        use object::{ObjectSection, ObjectSymbol, SectionKind, SymbolKind};

        let text_base = match image.sections().find(|s| s.kind() == SectionKind::Text) {
            Some(section) => match section.data() {
                Ok(data) => data.as_ptr() as usize,
                Err(_) => return,
            },
            None => return,
        };

        for sym in image.symbols() {
            if !sym.is_definition() {
                continue;
            }
            if sym.kind() != SymbolKind::Text {
                continue;
            }
            let address = sym.address();
            let size = sym.size();
            if address == 0 || size == 0 {
                continue;
            }
            if let Ok(name) = sym.name() {
                let addr = text_base + address as usize;
                let method_id = Self::get_raw_method_id();
                self.event_load(
                    method_id,
                    "wasm_file",
                    "wasm_module",
                    &name,
                    addr as *const u8,
                    size as usize,
                );
            }
        }
    }
}
