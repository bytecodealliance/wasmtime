use std::env;
use std::process::{Command, Stdio};

fn run_example(name: &'static str) {
    let cargo = env::var("CARGO").unwrap_or("cargo".to_string());
    let pkg_dir = env!("CARGO_MANIFEST_DIR");
    assert!(
        Command::new(cargo)
            .current_dir(pkg_dir)
            .stdout(Stdio::null())
            .args(&["run", "-q", "--example", name])
            .status()
            .expect("success")
            .success(),
        "failed to execute the example '{}'",
        name,
    );
}

#[test]
fn test_run_hello_example() {
    run_example("hello");
}

#[test]
fn test_run_gcd_example() {
    run_example("gcd");
}

#[test]
fn test_run_memory_example() {
    run_example("memory");
}

#[test]
fn test_run_multi_example() {
    run_example("multi");
}
