pub mod async_;
pub mod http;
pub mod nn;
pub mod p3;
pub mod preview1;
pub mod sockets;
pub mod tls;

wit_bindgen::generate!({
    inline: "
        package wasmtime:test;

        world test {
            include wasi:cli/imports@0.2.6;
            include wasi:http/imports@0.2.6;
            include wasi:config/imports@0.2.0-draft;
            include wasi:keyvalue/imports@0.2.0-draft;
            include wasi:tls/imports@0.2.0-draft;
        }
    ",
    path: [
        "../wasi-http/wit",
        "../wasi-config/wit",
        "../wasi-keyvalue/wit",
        "../wasi-tls/wit/deps/tls",
    ],
    world: "wasmtime:test/test",
    features: ["cli-exit-with-code", "tls"],
    generate_all,
});

pub mod proxy {
    wit_bindgen::generate!({
        path: "../wasi-http/wit",
        world: "wasi:http/proxy",
        default_bindings_module: "test_programs::proxy",
        pub_export_macro: true,
        with: {
            "wasi:http/types@0.2.6": crate::wasi::http::types,
            "wasi:http/outgoing-handler@0.2.6": crate::wasi::http::outgoing_handler,
            "wasi:random/random@0.2.6": crate::wasi::random::random,
            "wasi:io/error@0.2.6": crate::wasi::io::error,
            "wasi:io/poll@0.2.6": crate::wasi::io::poll,
            "wasi:io/streams@0.2.6": crate::wasi::io::streams,
            "wasi:cli/stdout@0.2.6": crate::wasi::cli::stdout,
            "wasi:cli/stderr@0.2.6": crate::wasi::cli::stderr,
            "wasi:cli/stdin@0.2.6": crate::wasi::cli::stdin,
            "wasi:clocks/monotonic-clock@0.2.6": crate::wasi::clocks::monotonic_clock,
            "wasi:clocks/wall-clock@0.2.6": crate::wasi::clocks::wall_clock,
        },
    });
}

impl std::fmt::Display for wasi::io::error::Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_debug_string())
    }
}

impl std::error::Error for wasi::io::error::Error {}
