use crate::{wasm_frame_vec_t, wasm_instance_t, wasm_name_t, wasm_store_t};
use once_cell::unsync::OnceCell;
use wasmtime::{Trap, TrapCode};

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
pub unsafe extern "C" fn wasmtime_trap_new(message: *const u8, len: usize) -> Box<wasm_trap_t> {
    let bytes = crate::slice_from_raw_parts(message, len);
    let message = String::from_utf8_lossy(&bytes);
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
    if raw.trap.trace().unwrap_or(&[]).len() > 0 {
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
    let vec = (0..raw.trap.trace().unwrap_or(&[]).len())
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
pub extern "C" fn wasmtime_trap_code(raw: &wasm_trap_t, code: &mut i32) -> bool {
    match raw.trap.trap_code() {
        Some(c) => {
            *code = match c {
                TrapCode::StackOverflow => 0,
                TrapCode::MemoryOutOfBounds => 1,
                TrapCode::HeapMisaligned => 2,
                TrapCode::TableOutOfBounds => 3,
                TrapCode::IndirectCallToNull => 4,
                TrapCode::BadSignature => 5,
                TrapCode::IntegerOverflow => 6,
                TrapCode::IntegerDivisionByZero => 7,
                TrapCode::BadConversionToInteger => 8,
                TrapCode::UnreachableCodeReached => 9,
                TrapCode::Interrupt => 10,
                _ => unreachable!(),
            };
            true
        }
        None => false,
    }
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
    frame.trap.trace().expect("backtraces are always enabled")[frame.idx].func_index()
}

#[no_mangle]
pub extern "C" fn wasmtime_frame_func_name(frame: &wasm_frame_t) -> Option<&wasm_name_t> {
    frame
        .func_name
        .get_or_init(|| {
            frame.trap.trace().expect("backtraces are always enabled")[frame.idx]
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
            frame.trap.trace().expect("backtraces are always enabled")[frame.idx]
                .module_name()
                .map(|s| wasm_name_t::from(s.to_string().into_bytes()))
        })
        .as_ref()
}

#[no_mangle]
pub extern "C" fn wasm_frame_func_offset(frame: &wasm_frame_t) -> usize {
    frame.trap.trace().expect("backtraces are always enabled")[frame.idx]
        .func_offset()
        .unwrap_or(usize::MAX)
}

#[no_mangle]
pub extern "C" fn wasm_frame_instance(_arg1: *const wasm_frame_t) -> *mut wasm_instance_t {
    unimplemented!("wasm_frame_instance")
}

#[no_mangle]
pub extern "C" fn wasm_frame_module_offset(frame: &wasm_frame_t) -> usize {
    frame.trap.trace().expect("backtraces are always enabled")[frame.idx]
        .module_offset()
        .unwrap_or(usize::MAX)
}

#[no_mangle]
pub extern "C" fn wasm_frame_copy(frame: &wasm_frame_t) -> Box<wasm_frame_t> {
    Box::new(frame.clone())
}
