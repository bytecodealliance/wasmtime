pub mod http;
pub mod nn;
pub mod preview1;
pub mod sockets;

wit_bindgen::generate!({
    inline: "
        package wasmtime:test;

        world test {
            include wasi:cli/imports@0.2.3;
            include wasi:http/imports@0.2.3;
            include wasi:config/imports@0.2.0-draft;
            include wasi:keyvalue/imports@0.2.0-draft;

            include wasi:clocks/imports@0.3.0;
            include wasi:random/imports@0.3.0;
        }
    ",
    path: [
        "../wasi/src/p3/wit",
        "../wasi-http/src/p2/wit",
        "../wasi-config/src/p2/wit",
        "../wasi-keyvalue/src/p2/wit",
    ],
    world: "wasmtime:test/test",
    features: ["cli-exit-with-code"],
    async: {
         imports: [
             "wasi:clocks/monotonic-clock@0.3.0#wait-for",
             "wasi:clocks/monotonic-clock@0.3.0#wait-until",
         ],
    },
    generate_all,
});

pub mod proxy {
    wit_bindgen::generate!({
        path: "../wasi-http/src/p2/wit",
        world: "wasi:http/proxy",
        default_bindings_module: "test_programs::proxy",
        pub_export_macro: true,
        with: {
            "wasi:http/types@0.2.3": crate::wasi::http::types,
            "wasi:http/outgoing-handler@0.2.3": crate::wasi::http::outgoing_handler,
            "wasi:random/random@0.2.3": crate::wasi::random0_2_3::random,
            "wasi:io/error@0.2.3": crate::wasi::io::error,
            "wasi:io/poll@0.2.3": crate::wasi::io::poll,
            "wasi:io/streams@0.2.3": crate::wasi::io::streams,
            "wasi:cli/stdout@0.2.3": crate::wasi::cli::stdout,
            "wasi:cli/stderr@0.2.3": crate::wasi::cli::stderr,
            "wasi:cli/stdin@0.2.3": crate::wasi::cli::stdin,
            "wasi:clocks/monotonic-clock@0.2.3": crate::wasi::clocks0_2_3::monotonic_clock,
            "wasi:clocks/wall-clock@0.2.3": crate::wasi::clocks0_2_3::wall_clock,
        },
    });
}
