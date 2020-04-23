use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Once;

fn run_c_example(name: &'static str, expected_out: &str) {
    // Windows requires different `cc` flags and I'm not sure what they
    // are. Also we need a way to shepherd the current host target to the `cc`
    // invocation but `cargo` only defines the `TARGET` environment variable for
    // build scripts, not tests. Therefore, we just make these tests specific to
    // bog standard x64 linux. This should run in CI, at least!
    if cfg!(not(all(
        target_arch = "x86_64",
        target_os = "linux",
        target_env = "gnu"
    ))) {
        eprintln!("This test is only enabled for the `x86_64-unknown-linux-gnu` target");
        return;
    }

    let pkg_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    // Make sure we've built `libwasmtime.a` with the `wat` feature enabled
    // so that we have the `wasmtime_wat2wasm` function.
    static BUILD_LIBWASMTIME: Once = Once::new();
    BUILD_LIBWASMTIME.call_once(|| {
        let status = Command::new("cargo")
            .args(&["build", "-p", "wasmtime-c-api", "--features", "wat"])
            .current_dir(pkg_dir)
            .status()
            .expect("should run `cargo build` OK");
        assert!(status.success());
    });

    let examples_dir = pkg_dir
        // Pop `c-api`.
        .join("..")
        // Pop `crates`.
        .join("..")
        .join("examples");
    let include_dir = pkg_dir.join("include");
    let wasm_c_api_include_dir = pkg_dir.join("wasm-c-api").join("include");
    let out_dir = pkg_dir.join("..").join("..").join("target").join("debug");
    let c_examples_dir = out_dir.join("c-examples");
    fs::create_dir_all(&c_examples_dir).unwrap();
    let libwasmtime = out_dir.join("libwasmtime.a");
    assert!(libwasmtime.exists());

    let status = Command::new(env::var("CC").unwrap_or("gcc".into()))
        .arg(examples_dir.join(name).with_extension("c"))
        .arg(libwasmtime)
        .arg(format!("-I{}", include_dir.display()))
        .arg(format!("-I{}", wasm_c_api_include_dir.display()))
        .arg("-lpthread")
        .arg("-ldl")
        .arg("-lm")
        .arg("-lrt")
        .current_dir(&examples_dir)
        .arg("-o")
        .arg(c_examples_dir.join(name))
        .status()
        .expect("should spawn CC ok");
    assert!(status.success());
    assert!(c_examples_dir.join(name).exists());

    let output = Command::new(c_examples_dir.join(name))
        .current_dir(pkg_dir.join("..").join(".."))
        .output()
        .expect("should spawn C example OK");

    assert!(
        output.status.success(),
        "failed to execute the C example '{}': {}",
        name,
        String::from_utf8_lossy(&output.stderr),
    );

    let actual_stdout =
        String::from_utf8(output.stdout).expect("C example's output should be utf-8");
    assert_eq!(
        actual_stdout, expected_out,
        "unexpected stdout from example",
    );
}

#[test]
fn test_run_hello_example() {
    run_c_example(
        "hello",
        "Initializing...\n\
         Compiling module...\n\
         Creating callback...\n\
         Instantiating module...\n\
         Extracting export...\n\
         Calling export...\n\
         Calling back...\n\
         > Hello World!\n\
         All finished!\n",
    );
}

#[test]
fn test_run_memory_example() {
    run_c_example(
        "memory",
        "Initializing...\n\
         Compiling module...\n\
         Instantiating module...\n\
         Extracting exports...\n\
         Checking memory...\n\
         Mutating memory...\n\
         Growing memory...\n\
         Creating stand-alone memory...\n\
         Shutting down...\n\
         Done.\n",
    );
}

#[test]
fn test_run_linking_example() {
    run_c_example("linking", "Hello, world!\n");
}

#[test]
fn test_run_multi_example() {
    run_c_example(
        "multi",
        "Initializing...\n\
         Compiling module...\n\
         Creating callback...\n\
         Instantiating module...\n\
         Extracting export...\n\
         Calling export...\n\
         Calling back...\n\
         > 1 2\n\
         \n\
         Printing result...\n\
         > 2 1\n\
         Shutting down...\n\
         Done.\n",
    );
}

#[test]
fn test_run_gcd_example() {
    run_c_example("gcd", "gcd(6, 27) = 3\n");
}
