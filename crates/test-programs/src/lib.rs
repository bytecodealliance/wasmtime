pub mod http;
pub mod nn;
pub mod preview1;
pub mod sockets;

wit_bindgen::generate!({
    world: "test-command",
    path: "../wasi/wit",
    generate_all,
});

pub mod proxy {
    wit_bindgen::generate!({
        path: "../wasi-http/wit",
        world: "wasi:http/proxy",
        default_bindings_module: "test_programs::proxy",
        pub_export_macro: true,
        with: {
            "wasi:http/types@0.2.0": crate::wasi::http::types,
            "wasi:http/outgoing-handler@0.2.0": crate::wasi::http::outgoing_handler,
            "wasi:random/random@0.2.0": crate::wasi::random::random,
            "wasi:io/error@0.2.0": crate::wasi::io::error,
            "wasi:io/poll@0.2.0": crate::wasi::io::poll,
            "wasi:io/streams@0.2.0": crate::wasi::io::streams,
            "wasi:cli/stdout@0.2.0": crate::wasi::cli::stdout,
            "wasi:cli/stderr@0.2.0": crate::wasi::cli::stderr,
            "wasi:cli/stdin@0.2.0": crate::wasi::cli::stdin,
            "wasi:clocks/monotonic-clock@0.2.0": crate::wasi::clocks::monotonic_clock,
            "wasi:clocks/wall-clock@0.2.0": crate::wasi::clocks::wall_clock,
        },
    });
}

pub mod config {
    wit_bindgen::generate!({
        path: "../wasi-runtime-config/wit",
        world: "wasi:config/imports",
    });
}

pub mod keyvalue {
    wit_bindgen::generate!({
        path: "../wasi-keyvalue/wit",
        world: "wasi:keyvalue/imports",
        type_section_suffix: "keyvalue",
    });
}
