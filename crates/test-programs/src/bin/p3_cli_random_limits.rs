use test_programs::p3::wasi::random;

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let args = std::env::args().collect::<Vec<_>>();
        let args = args.iter().map(|s| s.as_str()).collect::<Vec<_>>();
        match &args[1..] {
            ["random", n] => {
                random::random::get_random_bytes(n.parse().unwrap());
            }
            ["insecure", n] => {
                random::insecure::get_insecure_random_bytes(n.parse().unwrap());
            }
            other => {
                panic!("unexpected args: {other:?}");
            }
        }
        Ok(())
    }
}

fn main() {}
