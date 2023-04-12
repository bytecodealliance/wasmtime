pub mod command;
pub mod proxy;

wasmtime::component::bindgen!({
    path: "../wit",
    world: "command",
    only_interfaces: true,
    tracing: true,
    async: true,
    trappable_error_type: {
        "filesystem"::"error-code": Error,
        "streams"::"stream-error": Error,
    }
});
