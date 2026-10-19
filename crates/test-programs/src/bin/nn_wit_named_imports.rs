mod a {
    wit_bindgen::generate!({
        inline: r#"
            package a:b;

            world a {
                import a: wasi:nn/graph@0.2.0-rc-2024-10-28;
            }
        "#,
        path: "../wasi-nn/wit",
        generate_all,
    });
}

mod b {
    wit_bindgen::generate!({
        inline: r#"
            package a:b;

            world b {
                import b: wasi:nn/graph@0.2.0-rc-2024-10-28;
            }
        "#,
        path: "../wasi-nn/wit",
        generate_all,
    });
}

fn main() {
    // Each labelled `graph` is backed by a different registry on the host,
    // which only has a graph with the same name as the label.
    a::a::load_by_name("a").unwrap();
    assert!(a::a::load_by_name("b").is_err());
    b::b::load_by_name("b").unwrap();
}
