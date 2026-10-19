mod a {
    wit_bindgen::generate!({
        inline: r#"
            package a:b;

            world a {
                import a: wasi:keyvalue/store@0.2.0-draft;
            }
        "#,
        path: "../wasi-keyvalue/wit",
        generate_all,
    });
}

mod b {
    wit_bindgen::generate!({
        inline: r#"
            package a:b;

            world b {
                import b: wasi:keyvalue/store@0.2.0-draft;
            }
        "#,
        path: "../wasi-keyvalue/wit",
        generate_all,
    });
}

use test_programs::wasi::keyvalue::store;

fn main() {
    // Each labelled store is backed by a different context on the host.
    let a = a::a::open("").unwrap();
    let b = b::b::open("").unwrap();
    let default = store::open("").unwrap();
    assert_eq!(a.get("name").unwrap().as_deref(), Some(&b"a"[..]));
    assert_eq!(b.get("name").unwrap().as_deref(), Some(&b"b"[..]));
    assert_eq!(
        default.get("name").unwrap().as_deref(),
        Some(&b"default"[..])
    );
}
