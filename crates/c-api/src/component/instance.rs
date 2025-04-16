use wasmtime::component::Instance;

#[repr(transparent)]
pub struct wasmtime_component_instance_t {
    pub(crate) instance: Instance,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_component_instance_delete(
    _instance: Box<wasmtime_component_instance_t>,
) {
}
