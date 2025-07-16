pub mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "transmit-callee",
        concurrent_exports: true,
        async: true
    });
}
