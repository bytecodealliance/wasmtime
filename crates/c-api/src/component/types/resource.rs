use wasmtime::component::ResourceType;

type_wrapper! {
    #[derive(PartialEq)]
    pub struct wasmtime_component_resource_type_t {
        pub(crate) ty: ResourceType,
    }

    clone: wasmtime_component_resource_type_clone,
    delete: wasmtime_component_resource_type_delete,
    equal: wasmtime_component_resource_type_equal,
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_component_resource_type_new_host(
    ty: u32,
) -> Box<wasmtime_component_resource_type_t> {
    Box::new(wasmtime_component_resource_type_t {
        ty: ResourceType::host_dynamic(ty),
    })
}
