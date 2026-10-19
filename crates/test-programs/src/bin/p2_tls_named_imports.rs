mod a {
    wit_bindgen::generate!({
        inline: r#"
            package a:b;

            world a {
                import a: wasi:tls/types@0.2.0-draft;
            }
        "#,
        path: "../wasi-tls/wit",
        features: ["tls"],
        with: {
            "wasi:io/error@0.2.12": test_programs::wasi::io::error,
            "wasi:io/poll@0.2.12": test_programs::wasi::io::poll,
            "wasi:io/streams@0.2.12": test_programs::wasi::io::streams,
        },
        generate_all,
    });
}

mod b {
    wit_bindgen::generate!({
        inline: r#"
            package a:b;

            world b {
                import b: wasi:tls/types@0.2.0-draft;
            }
        "#,
        path: "../wasi-tls/wit",
        features: ["tls"],
        with: {
            "wasi:io/error@0.2.12": test_programs::wasi::io::error,
            "wasi:io/poll@0.2.12": test_programs::wasi::io::poll,
            "wasi:io/streams@0.2.12": test_programs::wasi::io::streams,
        },
        generate_all,
    });
}

use test_programs::wasi::cli::{stdin, stdout};

fn main() {
    // Each labelled import is backed by a different TLS provider on the host
    // which fails handshakes with its own name.
    let handshake =
        a::a::ClientHandshake::new("example.com", stdin::get_stdin(), stdout::get_stdout());
    let future = a::a::ClientHandshake::finish(handshake);
    future.subscribe().block();
    let err = future.get().unwrap().unwrap().err().unwrap();
    assert_eq!(err.to_debug_string(), "provider a");

    let handshake =
        b::b::ClientHandshake::new("example.com", stdin::get_stdin(), stdout::get_stdout());
    let future = b::b::ClientHandshake::finish(handshake);
    future.subscribe().block();
    let err = future.get().unwrap().unwrap().err().unwrap();
    assert_eq!(err.to_debug_string(), "provider b");
}
