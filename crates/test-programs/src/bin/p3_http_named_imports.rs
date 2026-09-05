mod a {
    wit_bindgen::generate!({
        inline: r#"
            package a:b;

            world foo {
                import a: wasi:http/types@0.3.0;
            }
        "#,
        path: "../wasi-http/src/p3/wit",
        generate_all,
    });
}

mod b {
    wit_bindgen::generate!({
        inline: r#"
            package a:b;

            world bar {
                import b: wasi:http/types@0.3.0;
            }
        "#,
        path: "../wasi-http/src/p3/wit",
        generate_all,
    });
}

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let a = a::a::Fields::new();
        assert!(a.append("a", b"0").is_err());
        assert!(a.append("b", b"0").is_ok());

        let b = b::b::Fields::new();
        assert!(b.append("a", b"0").is_ok());
        assert!(b.append("b", b"0").is_err());

        let c = test_programs::p3::wasi::http::types::Fields::new();
        assert!(c.append("a", b"0").is_ok());
        assert!(c.append("b", b"0").is_ok());
        Ok(())
    }
}

fn main() {}
