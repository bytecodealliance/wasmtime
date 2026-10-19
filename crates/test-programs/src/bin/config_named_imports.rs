mod a {
    wit_bindgen::generate!({
        inline: r#"
            package a:b;

            world a {
                import a: wasi:config/store@0.2.0-rc.1;
            }
        "#,
        path: "../wasi-config/wit",
        generate_all,
    });
}

mod b {
    wit_bindgen::generate!({
        inline: r#"
            package a:b;

            world b {
                import b: wasi:config/store@0.2.0-rc.1;
            }
        "#,
        path: "../wasi-config/wit",
        generate_all,
    });
}

use test_programs::wasi::config::store;

fn main() {
    // Each labelled store is backed by a different context on the host.
    assert_eq!(a::a::get("name").unwrap().as_deref(), Some("a"));
    assert_eq!(b::b::get("name").unwrap().as_deref(), Some("b"));
    assert_eq!(store::get("name").unwrap().as_deref(), Some("default"));
}
