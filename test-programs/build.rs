use heck::ToSnakeCase;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use wit_component::ComponentEncoder;

fn build_adapter(name: &str, features: &[&str]) -> Vec<u8> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed=../../src");
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--release")
        .current_dir("../../")
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

    // Build all test program crates
    // wasi-tests and test-programs require nightly for a feature in the `errno` crate
    println!("cargo:rerun-if-changed=..");
    let mut cmd = Command::new("rustup");
    cmd.arg("run")
        .arg("nightly-2023-03-14")
        .arg("cargo")
        .arg("build")
        .arg("--target=wasm32-wasi")
        .arg("--package=wasi-tests")
        .arg("--package=test-programs")
        .current_dir("..")
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
        .current_dir("..")
        .env("CARGO_TARGET_DIR", &out_dir)
        .env("CARGO_PROFILE_DEV_DEBUG", "1")
        .env_remove("CARGO_ENCODED_RUSTFLAGS");
    let status = cmd.status().unwrap();
    assert!(status.success());

    let meta = cargo_metadata::MetadataCommand::new().exec().unwrap();

    let mut command_components = Vec::new();

    for stem in targets_in_package(&meta, "test-programs", "bin").chain(targets_in_package(
        &meta,
        "wasi-tests",
        "bin",
    )) {
        let file = out_dir
            .join("wasm32-wasi/debug")
            .join(format!("{stem}.wasm"));

        let module = fs::read(&file).expect("read wasm module");
        let component = ComponentEncoder::default()
            .module(module.as_slice())
            .unwrap()
            .validate(true)
            .adapter("wasi_snapshot_preview1", &command_adapter)
            .unwrap()
            .encode()
            .expect(&format!(
                "module {:?} can be translated to a component",
                file
            ));
        let component_path = out_dir.join(format!("{}.component.wasm", &stem));
        fs::write(&component_path, component).expect("write component to disk");
        command_components.push((stem, component_path));
    }

    let mut reactor_components = Vec::new();

    for stem in targets_in_package(&meta, "reactor-tests", "cdylib") {
        let stem = stem.to_snake_case();
        let file = out_dir
            .join("wasm32-wasi/debug")
            .join(format!("{stem}.wasm"));

        let module = fs::read(&file).expect(&format!("read wasm module: {file:?}"));
        let component = ComponentEncoder::default()
            .module(module.as_slice())
            .unwrap()
            .validate(true)
            .adapter("wasi_snapshot_preview1", &reactor_adapter)
            .unwrap()
            .encode()
            .expect(&format!(
                "module {:?} can be translated to a component",
                file
            ));
        let component_path = out_dir.join(format!("{}.component.wasm", &stem));
        fs::write(&component_path, component).expect("write component to disk");
        reactor_components.push((stem, component_path));
    }

    let src = format!(
        "const COMMAND_COMPONENTS: &[(&str, &str)] = &{command_components:?};
         const REACTOR_COMPONENTS: &[(&str, &str)] = &{reactor_components:?};
        ",
    );
    std::fs::write(out_dir.join("components.rs"), src).unwrap();
}

fn targets_in_package<'a>(
    meta: &'a cargo_metadata::Metadata,
    package: &'a str,
    kind: &'a str,
) -> impl Iterator<Item = &'a String> + 'a {
    meta.packages
        .iter()
        .find(|p| p.name == package)
        .unwrap()
        .targets
        .iter()
        .filter(move |t| t.kind == &[kind])
        .map(|t| &t.name)
}
