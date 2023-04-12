pub mod command;
pub mod proxy;

wasmtime::component::bindgen!({
    path: "../wit",
    // The commmand-extended world happens to encompass all of the available interfaces:
    world: "command-extended",
    only_interfaces: true,
    tracing: true,
    async: true,
    trappable_error_type: {
        "filesystem"::"error-code": Error,
        "streams"::"stream-error": Error,
    }
});
