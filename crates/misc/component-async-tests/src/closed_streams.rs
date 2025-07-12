pub mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "closed-streams",
        concurrent_imports: true,
        concurrent_exports: true,
        async: {
            only_imports: [
                "local:local/closed#read-stream",
                "local:local/closed#read-future",
            ]
        },
    });
}
