pub mod http;
pub mod nn;
pub mod preview1;
pub mod sockets;

wit_bindgen::generate!("test-command" in "../wasi/wit");

pub mod proxy {
    wit_bindgen::generate!({
        path: "../wasi-http/wit",
        world: "wasi:http/proxy",
        default_bindings_module: "test_programs::proxy",
        pub_export_macro: true,
        with: {
            "wasi:http/types@0.2.0": crate::wasi::http::types,
            "wasi:http/outgoing-handler@0.2.0": crate::wasi::http::outgoing_handler,
        },
    });
}

pub mod rpc_hello {
    wit_bindgen::generate!({
        path: "../rpc/tests/wit/hello",
        world: "client",
        default_bindings_module: "test_programs::rpc_hello",
        pub_export_macro: true,
    });
}
