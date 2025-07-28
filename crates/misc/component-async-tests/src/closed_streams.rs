pub mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "closed-streams",
    });
}
