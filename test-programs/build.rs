use heck::ToSnakeCase;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use wit_component::ComponentEncoder;

fn build_adapter(name: &str, features: &[&str]) -> Vec<u8> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed=../src");
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--release")
        .current_dir("../")
        .arg("--target=wasm32-unknown-unknown")
        .env("CARGO_TARGET_DIR", &out_dir)
        .env_remove("CARGO_ENCODED_RUSTFLAGS");
    for f in features {
        cmd.arg(f);
    }
    let status = cmd.status().unwrap();
    assert!(status.success());

    let adapter = out_dir.join(format!("wasi_snapshot_preview1.{name}.wasm"));
    std::fs::copy(
        out_dir.join("wasm32-unknown-unknown/release/wasi_snapshot_preview1.wasm"),
        &adapter,
    )
    .unwrap();
    println!("wasi {name} adapter: {:?}", &adapter);
    fs::read(&adapter).unwrap()
}

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let reactor_adapter = build_adapter("reactor", &[]);
    let command_adapter =
        build_adapter("command", &["--no-default-features", "--features=command"]);

    println!("cargo:rerun-if-changed=./wasi-tests");
    println!("cargo:rerun-if-changed=./command-tests");
    println!("cargo:rerun-if-changed=./reactor-tests");

    // wasi-tests and command-tests need require nightly for a feature in the `io-extras` crate:
    let mut cmd = Command::new("rustup");
    cmd.arg("run")
        .arg("nightly-2023-03-14")
        .arg("cargo")
        .arg("build")
        .arg("--target=wasm32-wasi")
        .arg("--package=wasi-tests")
        .arg("--package=command-tests")
        .env("CARGO_TARGET_DIR", &out_dir)
        .env("CARGO_PROFILE_DEV_DEBUG", "1")
        .env_remove("CARGO_ENCODED_RUSTFLAGS");
    let status = cmd.status().unwrap();
    assert!(status.success());

    // reactor-tests can build on stable:
    let mut cmd = Command::new("rustup");
    cmd.arg("run")
        .arg("stable")
        .arg("cargo")
        .arg("build")
        .arg("--target=wasm32-wasi")
        .arg("--package=reactor-tests")
        .env("CARGO_TARGET_DIR", &out_dir)
        .env("CARGO_PROFILE_DEV_DEBUG", "1")
        .env_remove("CARGO_ENCODED_RUSTFLAGS");
    let status = cmd.status().unwrap();
    assert!(status.success());

    let meta = cargo_metadata::MetadataCommand::new().exec().unwrap();

    let command_tests = targets_in_package(&meta, "command-tests", "bin")
        .into_iter()
        .map(|stem| compile_component(stem, &out_dir, &command_adapter))
        .collect::<Vec<_>>();

    let wasi_tests_modules = targets_in_package(&meta, "wasi-tests", "bin")
        .into_iter()
        .map(|stem| {
            (
                stem.clone(),
                out_dir
                    .join("wasm32-wasi/debug")
                    .join(format!("{stem}.wasm"))
                    .as_os_str()
                    .to_str()
                    .unwrap()
                    .to_string(),
            )
        })
        .collect::<Vec<_>>();

    let wasi_tests_components = targets_in_package(&meta, "wasi-tests", "bin")
        .into_iter()
        .map(|stem| compile_component(stem, &out_dir, &command_adapter))
        .collect::<Vec<_>>();

    let reactor_tests = targets_in_package(&meta, "reactor-tests", "cdylib")
        .into_iter()
        .map(|stem| compile_component(stem, &out_dir, &reactor_adapter))
        .collect::<Vec<_>>();

    let src = format!(
        "const COMMAND_TESTS_COMPONENTS: &[(&str, &str)] = &{command_tests:?};
         const WASI_TESTS_MODULES: &[(&str, &str)] = &{wasi_tests_modules:?};
         const WASI_TESTS_COMPONENTS: &[(&str, &str)] = &{wasi_tests_components:?};
         const REACTOR_TESTS_COMPONENTS: &[(&str, &str)] = &{reactor_tests:?};
        ",
    );
    std::fs::write(out_dir.join("components.rs"), src).unwrap();
}

fn compile_component(stem: String, out_dir: &PathBuf, adapter: &[u8]) -> (String, String) {
    let stem = stem.to_snake_case();
    let file = out_dir
        .join("wasm32-wasi/debug")
        .join(format!("{stem}.wasm"));
    let module = fs::read(&file).expect("read wasm module");
    let component = ComponentEncoder::default()
        .module(module.as_slice())
        .unwrap()
        .validate(true)
        .adapter("wasi_snapshot_preview1", adapter)
        .unwrap()
        .encode()
        .expect(&format!(
            "module {:?} can be translated to a component",
            file
        ));
    let component_path = out_dir.join(format!("{}.component.wasm", &stem));
    fs::write(&component_path, component).expect("write component to disk");
    (
        stem,
        component_path.as_os_str().to_str().unwrap().to_string(),
    )
}

fn targets_in_package<'a>(
    meta: &'a cargo_metadata::Metadata,
    package: &'a str,
    kind: &'a str,
) -> Vec<String> {
    let targets = meta
        .packages
        .iter()
        .find(|p| p.name == package)
        .unwrap()
        .targets
        .iter()
        .filter(move |t| t.kind == &[kind])
        .map(|t| t.name.to_snake_case())
        .collect::<Vec<_>>();
    if targets.is_empty() {
        panic!("no targets for package {package:?} of kind {kind:?}")
    }
    targets
}
