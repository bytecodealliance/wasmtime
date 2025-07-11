wit_bindgen::generate!({
    inline: "
        package wasmtime:test;

        world testp3 {
            include wasi:cli/imports@0.3.0;

            export wasi:cli/run@0.3.0;
        }
    ",
    path: "../wasi/src/p3/wit",
    world: "wasmtime:test/testp3",
    default_bindings_module: "test_programs::p3",
    pub_export_macro: true,
    async: [
        "wasi:cli/run@0.3.0#run",
    ],
    generate_all
});
