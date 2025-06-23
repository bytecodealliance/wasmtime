#[repr(transparent)]
pub struct wasmtime_wasip2_config_t {
    pub(crate) builder: wasmtime_wasi::p2::WasiCtxBuilder,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_wasip2_config_new() -> Box<wasmtime_wasip2_config_t> {
    Box::new(wasmtime_wasip2_config_t {
        builder: wasmtime_wasi::p2::WasiCtxBuilder::new(),
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_wasip2_config_inherit_stdout(
    config: &mut wasmtime_wasip2_config_t,
) {
    config.builder.inherit_stdout();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_wasip2_config_delete(_: Box<wasmtime_wasip2_config_t>) {}
