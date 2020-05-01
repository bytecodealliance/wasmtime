use crate::frame_info::FRAME_INFO;
use crate::{Exit, Trap};
use anyhow::Error;
use wasmtime_environ::ir::TrapCode;

/// Convert from an internal unwinding code into an `Error`.
pub(crate) fn from_runtime(jit: wasmtime_runtime::Trap) -> Error {
    let info = FRAME_INFO.read().unwrap();
    match jit {
        wasmtime_runtime::Trap::User(error) => error,
        wasmtime_runtime::Trap::Jit {
            pc,
            backtrace,
            maybe_interrupted,
        } => {
            let mut code = info
                .lookup_trap_info(pc)
                .map(|info| info.trap_code)
                .unwrap_or(TrapCode::StackOverflow);
            if maybe_interrupted && code == TrapCode::StackOverflow {
                code = TrapCode::Interrupt;
            }
            Error::new(Trap::new_wasm(&info, Some(pc), code, backtrace))
        }
        wasmtime_runtime::Trap::Wasm {
            trap_code,
            backtrace,
        } => Error::new(Trap::new_wasm(&info, None, trap_code, backtrace)),
        wasmtime_runtime::Trap::OOM { backtrace } => Error::new(Trap::new_with_trace(
            &info,
            None,
            "out of memory".to_string(),
            backtrace,
        )),
        wasmtime_runtime::Trap::Exit { status } => Error::new(Exit::new(status)),
    }
}
