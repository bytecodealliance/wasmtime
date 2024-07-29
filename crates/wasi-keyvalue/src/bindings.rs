wasmtime::component::bindgen!({
    path: "wit",
    world: "wasi:keyvalue/imports",
    trappable_imports: true,
    async: true,
    with: {
        "wasi:keyvalue/store/bucket": crate::Bucket,
    },
    trappable_error_type: {
        "wasi:keyvalue/store/error" => crate::Error,
    },
});

pub(crate) mod sync {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "wasi:keyvalue/imports",
        trappable_imports: true,
        with: {
            "wasi:keyvalue/store/bucket": crate::Bucket,
        },
        trappable_error_type: {
            "wasi:keyvalue/store/error" => crate::Error,
        },
    });
}
