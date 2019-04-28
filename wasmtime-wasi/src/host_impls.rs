use super::host;
use super::wasm32;

pub fn wasmtime_ssp_proc_exit(rval: wasm32::__wasi_exitcode_t) {
    ::std::process::exit(rval as i32)
}

pub fn wasmtime_ssp_sched_yield() -> wasm32::__wasi_errno_t {
    unsafe {
        if libc::sched_yield() < 0 {
            return host::convert_errno(host::errno());
        }
    }

    wasm32::__WASI_ESUCCESS
}
