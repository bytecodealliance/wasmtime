mod a {
    wit_bindgen::generate!({
        inline: r#"
            package a:b;

            world foo {
                import a: wasi:cli/environment@0.2.12;
            }
        "#,
        path: "../wasi/src/p2/wit",
        generate_all,
    });
}

mod b {
    wit_bindgen::generate!({
        inline: r#"
            package a:b;

            world bar {
                import b: wasi:cli/environment@0.2.12;
            }
        "#,
        path: "../wasi/src/p2/wit",
        generate_all,
    });
}

fn main() {
    let a = a::a::get_environment();
    let b = b::b::get_environment();
    let c = wasip2::cli::environment::get_environment();
    assert_ne!(a, b);
    assert_ne!(a, c);
    assert_ne!(b, c);
}
