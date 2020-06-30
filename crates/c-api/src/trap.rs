use crate::{wasm_frame_vec_t, wasm_instance_t, wasm_name_t, wasm_store_t};
use once_cell::unsync::OnceCell;
use wasmtime::Trap;

#[repr(C)]
#[derive(Clone)]
pub struct wasm_trap_t {
    pub(crate) trap: Trap,
}

wasmtime_c_api_macros::declare_ref!(wasm_trap_t);

impl wasm_trap_t {
    pub(crate) fn new(trap: Trap) -> wasm_trap_t {
        wasm_trap_t { trap: trap }
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct wasm_frame_t {
    trap: Trap,
    idx: usize,
    func_name: OnceCell<Option<wasm_name_t>>,
    module_name: OnceCell<Option<wasm_name_t>>,
}

wasmtime_c_api_macros::declare_own!(wasm_frame_t);

pub type wasm_message_t = wasm_name_t;

#[no_mangle]
pub extern "C" fn wasm_trap_new(
    _store: &wasm_store_t,
    message: &wasm_message_t,
) -> Box<wasm_trap_t> {
    let message = message.as_slice();
    if message[message.len() - 1] != 0 {
        panic!("wasm_trap_new message stringz expected");
    }
    let message = String::from_utf8_lossy(&message[..message.len() - 1]);
    Box::new(wasm_trap_t {
        trap: Trap::new(message),
    })
}

#[no_mangle]
pub extern "C" fn wasm_trap_message(trap: &wasm_trap_t, out: &mut wasm_message_t) {
    let mut buffer = Vec::new();
    buffer.extend_from_slice(trap.trap.to_string().as_bytes());
    buffer.reserve_exact(1);
    buffer.push(0);
    out.set_buffer(buffer);
}

#[no_mangle]
pub extern "C" fn wasm_trap_origin(raw: &wasm_trap_t) -> Option<Box<wasm_frame_t>> {
    if raw.trap.trace().len() > 0 {
        Some(Box::new(wasm_frame_t {
            trap: raw.trap.clone(),
            idx: 0,
            func_name: OnceCell::new(),
            module_name: OnceCell::new(),
        }))
    } else {
        None
    }
}

#[no_mangle]
pub extern "C" fn wasm_trap_trace(raw: &wasm_trap_t, out: &mut wasm_frame_vec_t) {
    let vec = (0..raw.trap.trace().len())
        .map(|idx| {
            Some(Box::new(wasm_frame_t {
                trap: raw.trap.clone(),
                idx,
                func_name: OnceCell::new(),
                module_name: OnceCell::new(),
            }))
        })
        .collect();
    out.set_buffer(vec);
}

#[no_mangle]
pub extern "C" fn wasmtime_trap_exit_status(raw: &wasm_trap_t, status: &mut i32) -> bool {
    match raw.trap.i32_exit_status() {
        Some(i) => {
            *status = i;
            true
        }
        None => false,
    }
}

#[no_mangle]
pub extern "C" fn wasm_frame_func_index(frame: &wasm_frame_t) -> u32 {
    frame.trap.trace()[frame.idx].func_index()
}

#[no_mangle]
pub extern "C" fn wasmtime_frame_func_name(frame: &wasm_frame_t) -> Option<&wasm_name_t> {
    frame
        .func_name
        .get_or_init(|| {
            frame.trap.trace()[frame.idx]
                .func_name()
                .map(|s| wasm_name_t::from(s.to_string().into_bytes()))
        })
        .as_ref()
}

#[no_mangle]
pub extern "C" fn wasmtime_frame_module_name(frame: &wasm_frame_t) -> Option<&wasm_name_t> {
    frame
        .module_name
        .get_or_init(|| {
            frame.trap.trace()[frame.idx]
                .module_name()
                .map(|s| wasm_name_t::from(s.to_string().into_bytes()))
        })
        .as_ref()
}

#[no_mangle]
pub extern "C" fn wasm_frame_func_offset(frame: &wasm_frame_t) -> usize {
    frame.trap.trace()[frame.idx].func_offset()
}

#[no_mangle]
pub extern "C" fn wasm_frame_instance(_arg1: *const wasm_frame_t) -> *mut wasm_instance_t {
    unimplemented!("wasm_frame_instance")
}

#[no_mangle]
pub extern "C" fn wasm_frame_module_offset(frame: &wasm_frame_t) -> usize {
    frame.trap.trace()[frame.idx].module_offset()
}
