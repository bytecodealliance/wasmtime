use test_programs::wasi::cli::environment;

fn main() {
    assert_eq!(environment::initial_cwd().as_deref(), Some("/sandbox"));
}
