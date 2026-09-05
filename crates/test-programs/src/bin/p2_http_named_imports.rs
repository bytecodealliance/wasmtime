mod a {
    wit_bindgen::generate!({
        inline: r#"
            package a:b;

            world foo {
                import a: wasi:http/types@0.2.12;
            }
        "#,
        path: "../wasi-http/wit",
        generate_all,
    });
}

mod b {
    wit_bindgen::generate!({
        inline: r#"
            package a:b;

            world bar {
                import b: wasi:http/types@0.2.12;
            }
        "#,
        path: "../wasi-http/wit",
        generate_all,
    });
}

fn main() {
    let a = a::a::Fields::new();
    assert!(a.append("a", b"0").is_err());
    assert!(a.append("b", b"0").is_ok());

    let b = b::b::Fields::new();
    assert!(b.append("a", b"0").is_ok());
    assert!(b.append("b", b"0").is_err());

    let c = wasip2::http::types::Fields::new();
    assert!(c.append("a", b"0").is_ok());
    assert!(c.append("b", b"0").is_ok());
}
