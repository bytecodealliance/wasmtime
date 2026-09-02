mod a {
    wit_bindgen::generate!({
        inline: r#"
            package a:b;

            world foo {
                import a: wasi:cli/environment@0.3.0;
            }
        "#,
        path: "../wasi/src/p3/wit",
        generate_all,
    });
}

mod b {
    wit_bindgen::generate!({
        inline: r#"
            package a:b;

            world bar {
                import b: wasi:cli/environment@0.3.0;
            }
        "#,
        path: "../wasi/src/p3/wit",
        generate_all,
    });
}

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let a = a::a::get_environment();
        let b = b::b::get_environment();
        let c = test_programs::p3::wasi::cli::environment::get_environment();
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(b, c);
        Ok(())
    }
}

fn main() {}
