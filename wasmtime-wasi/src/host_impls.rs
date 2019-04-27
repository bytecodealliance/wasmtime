use super::wasm32;

pub fn wasmtime_ssp_proc_exit(rval: wasm32::__wasi_exitcode_t) {
    ::std::process::exit(rval as i32)
}
