pub mod http;
pub mod sockets;

wit_bindgen::generate!({
    inline: "
        package wasmtime:test;

        world testp3 {
            include wasi:cli/imports@0.3.0-rc-2026-02-09;
            include wasi:tls/imports@0.3.0-draft;
            import wasi:http/types@0.3.0-rc-2026-02-09;
            import wasi:http/client@0.3.0-rc-2026-02-09;
            import wasi:http/handler@0.3.0-rc-2026-02-09;

            export wasi:cli/run@0.3.0-rc-2026-02-09;
        }
    ",
    path: [
        "../wasi-http/src/p3/wit",
        "../wasi-tls/src/p3/wit",
    ],
    world: "wasmtime:test/testp3",
    default_bindings_module: "test_programs::p3",
    pub_export_macro: true,
    generate_all,
});

pub mod service {
    wit_bindgen::generate!({
        path: "../wasi-http/src/p3/wit",
        world: "wasi:http/service",
        default_bindings_module: "test_programs::p3::service",
        pub_export_macro: true,
        with: {
            "wasi:http/handler@0.3.0-rc-2026-02-09": crate::p3::wasi::http::handler,
            "wasi:http/types@0.3.0-rc-2026-02-09": crate::p3::wasi::http::types,
            "wasi:http/client@0.3.0-rc-2026-02-09": crate::p3::wasi::http::client,
            "wasi:random/random@0.3.0-rc-2026-02-09": crate::p3::wasi::random::random,
            "wasi:random/insecure@0.3.0-rc-2026-02-09": crate::p3::wasi::random::insecure,
            "wasi:random/insecure-seed@0.3.0-rc-2026-02-09": crate::p3::wasi::random::insecure_seed,
            "wasi:cli/stdout@0.3.0-rc-2026-02-09": crate::p3::wasi::cli::stdout,
            "wasi:cli/stderr@0.3.0-rc-2026-02-09": crate::p3::wasi::cli::stderr,
            "wasi:cli/stdin@0.3.0-rc-2026-02-09": crate::p3::wasi::cli::stdin,
            "wasi:cli/types@0.3.0-rc-2026-02-09": crate::p3::wasi::cli::types,
            "wasi:clocks/monotonic-clock@0.3.0-rc-2026-02-09": crate::p3::wasi::clocks::monotonic_clock,
            "wasi:clocks/system-clock@0.3.0-rc-2026-02-09": crate::p3::wasi::clocks::system_clock,
            "wasi:clocks/types@0.3.0-rc-2026-02-09": crate::p3::wasi::clocks::types,
        },
    });
}
