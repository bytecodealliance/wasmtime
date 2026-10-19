mod a {
    wit_bindgen::generate!({
        inline: r#"
            package a:b;

            world a {
                import a: wasi:tls/client@0.3.0-draft;
            }
        "#,
        path: "../wasi-tls/src/p3/wit",
        generate_all,
    });
}

mod b {
    wit_bindgen::generate!({
        inline: r#"
            package a:b;

            world b {
                import b: wasi:tls/client@0.3.0-draft;
            }
        "#,
        path: "../wasi-tls/src/p3/wit",
        generate_all,
    });
}

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        // Each labelled import is backed by a different TLS provider on the
        // host which fails handshakes with its own name.
        let conn = a::a::Connector::new();
        let (_cleartext_tx, cleartext_rx) = a::wit_stream::new();
        let (_ciphertext_rx, _send_result) = conn.send(cleartext_rx);
        let (_ciphertext_tx, ciphertext_rx) = a::wit_stream::new();
        let (_cleartext_rx, _receive_result) = conn.receive(ciphertext_rx);
        let err = a::a::Connector::connect(conn, "example.com".into())
            .await
            .unwrap_err();
        assert_eq!(err.to_debug_string(), "provider a");

        let conn = b::b::Connector::new();
        let (_cleartext_tx, cleartext_rx) = b::wit_stream::new();
        let (_ciphertext_rx, _send_result) = conn.send(cleartext_rx);
        let (_ciphertext_tx, ciphertext_rx) = b::wit_stream::new();
        let (_cleartext_rx, _receive_result) = conn.receive(ciphertext_rx);
        let err = b::b::Connector::connect(conn, "example.com".into())
            .await
            .unwrap_err();
        assert_eq!(err.to_debug_string(), "provider b");
        Ok(())
    }
}

fn main() {}
