#![cfg_attr(not(feature = "test_programs"), allow(dead_code))]

use heck::ToSnakeCase;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use wit_component::ComponentEncoder;

// NB: this is set to `false` when a breaking change to WIT is made since the
// wasi-http WIT is currently a submodule and can't be updated atomically with
// the rest of Wasmtime.
const BUILD_WASI_HTTP_TESTS: bool = true;

fn main() {
    #[cfg(feature = "test_programs")]
    build_and_generate_tests();
}

fn build_and_generate_tests() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let reactor_adapter = build_adapter(&out_dir, "reactor", &[]);
    let command_adapter = build_adapter(
        &out_dir,
        "command",
        &["--no-default-features", "--features=command"],
    );

    println!("cargo:rerun-if-changed=./wasi-tests");
    println!("cargo:rerun-if-changed=./command-tests");
    println!("cargo:rerun-if-changed=./reactor-tests");
    if BUILD_WASI_HTTP_TESTS {
        println!("cargo:rerun-if-changed=./wasi-http-tests");
    } else {
        println!("cargo:rustc-cfg=skip_wasi_http_tests");
    }

    // Build the test programs:
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--target=wasm32-wasi")
        .arg("--package=wasi-tests")
        .arg("--package=command-tests")
        .arg("--package=reactor-tests")
        .env("CARGO_TARGET_DIR", &out_dir)
        .env("CARGO_PROFILE_DEV_DEBUG", "1")
        .env_remove("CARGO_ENCODED_RUSTFLAGS");
    if BUILD_WASI_HTTP_TESTS {
        cmd.arg("--package=wasi-http-tests");
    }
    let status = cmd.status().unwrap();
    assert!(status.success());

    let meta = cargo_metadata::MetadataCommand::new().exec().unwrap();

    modules_rs(&meta, "wasi-tests", "bin", &out_dir);
    components_rs(&meta, "wasi-tests", "bin", &command_adapter, &out_dir);

    if BUILD_WASI_HTTP_TESTS {
        modules_rs(&meta, "wasi-http-tests", "bin", &out_dir);
        // FIXME this is broken at the moment because guest bindgen is embedding the proxy world type,
        // so wit-component expects the module to contain the proxy's exports. we need a different
        // world to pass guest bindgen that is just "a command that also can do outbound http"
        //components_rs(&meta, "wasi-http-tests", "bin", &command_adapter, &out_dir);
    }

    components_rs(&meta, "command-tests", "bin", &command_adapter, &out_dir);
    components_rs(&meta, "reactor-tests", "cdylib", &reactor_adapter, &out_dir);
}

// Creates an `${out_dir}/${package}_modules.rs` file that exposes a `get_module(&str) -> Module`,
// and a contains a `use self::{module} as _;` for each module that ensures that the user defines
// a symbol (ideally a #[test]) corresponding to each module.
fn modules_rs(meta: &cargo_metadata::Metadata, package: &str, kind: &str, out_dir: &PathBuf) {
    let modules = targets_in_package(meta, package, kind)
        .into_iter()
        .map(|stem| {
            (
                stem.clone(),
                out_dir
                    .join("wasm32-wasi")
                    .join("debug")
                    .join(format!("{stem}.wasm"))
                    .as_os_str()
                    .to_str()
                    .unwrap()
                    .to_string(),
            )
        })
        .collect::<Vec<_>>();

    let mut decls = String::new();
    let mut cases = String::new();
    let mut uses = String::new();
    for (stem, file) in modules {
        let global = format!("{}_MODULE", stem.to_uppercase());
        // Load the module from disk only once, in case it is used many times:
        decls += &format!(
            "
            lazy_static::lazy_static!{{
                static ref {global}: wasmtime::Module = {{
                    wasmtime::Module::from_file(&ENGINE, {file:?}).unwrap()
                }};
            }}
        "
        );
        // Match the stem str literal to the module. Cloning is just a ref count incr.
        cases += &format!("{stem:?} => {global}.clone(),\n");
        // Statically ensure that the user defines a function (ideally a #[test]) for each stem.
        uses += &format!("#[allow(unused_imports)] use self::{stem} as _;\n");
    }

    std::fs::write(
        out_dir.join(&format!("{}_modules.rs", package.to_snake_case())),
        format!(
            "
        {decls}
        pub fn get_module(s: &str) -> wasmtime::Module {{
            match s {{
                {cases}
                _ => panic!(\"no such module: {{}}\", s),
            }}
        }}
        {uses}
        "
        ),
    )
    .unwrap();
}

// Build the WASI Preview 1 adapter, and get the binary:
fn build_adapter(out_dir: &PathBuf, name: &str, features: &[&str]) -> Vec<u8> {
    println!("cargo:rerun-if-changed=../wasi-preview1-component-adapter");
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--release")
        .arg("--package=wasi-preview1-component-adapter")
        .arg("--target=wasm32-unknown-unknown")
        .env("CARGO_TARGET_DIR", out_dir)
        .env_remove("CARGO_ENCODED_RUSTFLAGS");
    for f in features {
        cmd.arg(f);
    }
    let status = cmd.status().unwrap();
    assert!(status.success());

    let adapter = out_dir.join(format!("wasi_snapshot_preview1.{name}.wasm"));
    std::fs::copy(
        out_dir
            .join("wasm32-unknown-unknown")
            .join("release")
            .join("wasi_snapshot_preview1.wasm"),
        &adapter,
    )
    .unwrap();
    println!("wasi {name} adapter: {:?}", &adapter);
    fs::read(&adapter).unwrap()
}

// Builds components out of modules, and creates an `${out_dir}/${package}_component.rs` file that
// exposes a `get_component(&str) -> Component`
// and a contains a `use self::{component} as _;` for each module that ensures that the user defines
// a symbol (ideally a #[test]) corresponding to each component.
fn components_rs(
    meta: &cargo_metadata::Metadata,
    package: &str,
    kind: &str,
    adapter: &[u8],
    out_dir: &PathBuf,
) {
    let mut decls = String::new();
    let mut cases = String::new();
    let mut uses = String::new();
    for target_name in targets_in_package(&meta, package, kind) {
        let stem = target_name.to_snake_case();
        let file = compile_component(&stem, out_dir, adapter);

        let global = format!("{}_COMPONENT", stem.to_uppercase());
        decls += &format!(
            "
            lazy_static::lazy_static!{{
                static ref {global}: wasmtime::component::Component = {{
                    wasmtime::component::Component::from_file(&ENGINE, {file:?}).unwrap()
                }};
            }}
        "
        );
        cases += &format!("{stem:?} => {global}.clone(),\n");
        uses += &format!("#[allow(unused_imports)] use self::{stem} as _;\n");
    }

    std::fs::write(
        out_dir.join(&format!("{}_components.rs", package.to_snake_case())),
        format!(
            "
        {decls}
        pub fn get_component(s: &str) -> wasmtime::component::Component {{
            match s {{
                {cases}
                _ => panic!(\"no such component: {{}}\", s),
            }}
        }}
        {uses}
        "
        ),
    )
    .unwrap();
}

// Compile a component, return the path of the binary:
fn compile_component(stem: &str, out_dir: &PathBuf, adapter: &[u8]) -> PathBuf {
    let file = out_dir
        .join("wasm32-wasi")
        .join("debug")
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
    component_path
}

// Get all targets in a given package with a given kind
// kind is "bin" for test program crates that expose a `fn main`, and
// "cdylib" for crates that implement a reactor.
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
