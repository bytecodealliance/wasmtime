use heck::*;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use wit_component::ComponentEncoder;

fn main() {
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
    let proxy_adapter = build_adapter(
        &out_dir,
        "proxy",
        &["--no-default-features", "--features=proxy"],
    );

    // Build the test programs:
    let mut cmd = cargo();
    cmd.arg("build")
        .arg("--target=wasm32-wasip1")
        .arg("--package=test-programs")
        .env("CARGO_TARGET_DIR", &out_dir)
        .env("CARGO_PROFILE_DEV_DEBUG", "2")
        .env("RUSTFLAGS", rustflags())
        .env_remove("CARGO_ENCODED_RUSTFLAGS");
    eprintln!("running: {cmd:?}");
    let status = cmd.status().unwrap();
    assert!(status.success());

    let meta = cargo_metadata::MetadataCommand::new().exec().unwrap();
    let targets = meta
        .packages
        .iter()
        .find(|p| p.name == "test-programs")
        .unwrap()
        .targets
        .iter()
        .filter(move |t| t.kind == &["bin"])
        .map(|t| &t.name)
        .collect::<Vec<_>>();

    let mut generated_code = String::new();

    let mut kinds = BTreeMap::new();

    for target in targets {
        let camel = target.to_shouty_snake_case();
        let wasm = out_dir
            .join("wasm32-wasip1")
            .join("debug")
            .join(format!("{target}.wasm"));
        read_deps_of(&wasm);

        generated_code += &format!("pub const {camel}: &'static str = {wasm:?};\n");

        // Bucket, based on the name of the test, into a "kind" which generates
        // a `foreach_*` macro below.
        let kind = match target.as_str() {
            s if s.starts_with("http_") => "http",
            s if s.starts_with("preview1_") => "preview1",
            s if s.starts_with("preview2_") => "preview2",
            s if s.starts_with("cli_") => "cli",
            s if s.starts_with("api_") => "api",
            s if s.starts_with("nn_") => "nn",
            s if s.starts_with("piped_") => "piped",
            s if s.starts_with("dwarf_") => "dwarf",
            s if s.starts_with("config_") => "config",
            s if s.starts_with("keyvalue_") => "keyvalue",
            // If you're reading this because you hit this panic, either add it
            // to a test suite above or add a new "suite". The purpose of the
            // categorization above is to have a static assertion that tests
            // added are actually run somewhere, so as long as you're also
            // adding test code somewhere that's ok.
            other => {
                panic!("don't know how to classify test name `{other}` to a kind")
            }
        };
        if !kind.is_empty() {
            kinds.entry(kind).or_insert(Vec::new()).push(target);
        }

        // Generate a component from each test.
        if target == "dwarf_imported_memory"
            || target == "dwarf_shared_memory"
            || target.starts_with("nn_witx")
        {
            continue;
        }
        let adapter = match target.as_str() {
            "reactor" => &reactor_adapter,
            s if s.starts_with("api_proxy") => &proxy_adapter,
            _ => &command_adapter,
        };
        let path = compile_component(&wasm, adapter);
        generated_code += &format!("pub const {camel}_COMPONENT: &'static str = {path:?};\n");
    }

    for (kind, targets) in kinds {
        generated_code += &format!("#[macro_export]");
        generated_code += &format!("macro_rules! foreach_{kind} {{\n");
        generated_code += &format!("    ($mac:ident) => {{\n");
        for target in targets {
            generated_code += &format!("$mac!({target});\n")
        }
        generated_code += &format!("    }}\n");
        generated_code += &format!("}}\n");
    }

    std::fs::write(out_dir.join("gen.rs"), generated_code).unwrap();
}

// Build the WASI Preview 1 adapter, and get the binary:
fn build_adapter(out_dir: &PathBuf, name: &str, features: &[&str]) -> Vec<u8> {
    let mut cmd = cargo();
    cmd.arg("build")
        .arg("--release")
        .arg("--package=wasi-preview1-component-adapter")
        .arg("--target=wasm32-unknown-unknown")
        .env("CARGO_TARGET_DIR", out_dir)
        .env("RUSTFLAGS", rustflags())
        .env_remove("CARGO_ENCODED_RUSTFLAGS");
    for f in features {
        cmd.arg(f);
    }
    eprintln!("running: {cmd:?}");
    let status = cmd.status().unwrap();
    assert!(status.success());

    let artifact = out_dir
        .join("wasm32-unknown-unknown")
        .join("release")
        .join("wasi_snapshot_preview1.wasm");
    let adapter = out_dir.join(format!("wasi_snapshot_preview1.{name}.wasm"));
    std::fs::copy(&artifact, &adapter).unwrap();
    read_deps_of(&artifact);
    println!("wasi {name} adapter: {:?}", &adapter);
    fs::read(&adapter).unwrap()
}

fn rustflags() -> &'static str {
    match option_env!("RUSTFLAGS") {
        // If we're in CI which is denying warnings then deny warnings to code
        // built here too to keep the tree warning-free.
        Some(s) if s.contains("-D warnings") => "-D warnings",
        _ => "",
    }
}

// Compile a component, return the path of the binary:
fn compile_component(wasm: &Path, adapter: &[u8]) -> PathBuf {
    println!("creating a component from {wasm:?}");
    let module = fs::read(wasm).expect("read wasm module");
    let component = ComponentEncoder::default()
        .module(module.as_slice())
        .unwrap()
        .validate(true)
        .adapter("wasi_snapshot_preview1", adapter)
        .unwrap()
        .encode()
        .expect("module can be translated to a component");
    let out_dir = wasm.parent().unwrap();
    let stem = wasm.file_stem().unwrap().to_str().unwrap();
    let component_path = out_dir.join(format!("{stem}.component.wasm"));
    fs::write(&component_path, component).expect("write component to disk");
    component_path
}

fn cargo() -> Command {
    // Miri configures its own sysroot which we don't want to use, so remove
    // miri's own wrappers around rustc to ensure that we're using the real
    // rustc to build these programs.
    let mut cargo = Command::new("cargo");
    if std::env::var("CARGO_CFG_MIRI").is_ok() {
        cargo.env_remove("RUSTC").env_remove("RUSTC_WRAPPER");
    }
    cargo
}

/// Helper function to read the `*.d` file that corresponds to `artifact`, an
/// artifact of a Cargo compilation.
///
/// This function will "parse" the makefile-based dep-info format to learn about
/// what files each binary depended on to ensure that this build script reruns
/// if any of these files change.
///
/// See
/// <https://doc.rust-lang.org/nightly/cargo/reference/build-cache.html#dep-info-files>
/// for more info.
fn read_deps_of(artifact: &Path) {
    let deps_file = artifact.with_extension("d");
    let contents = std::fs::read_to_string(&deps_file).expect("failed to read deps file");
    for line in contents.lines() {
        let Some(pos) = line.find(": ") else {
            continue;
        };
        let line = &line[pos + 2..];
        let mut parts = line.split_whitespace();
        while let Some(part) = parts.next() {
            let mut file = part.to_string();
            while file.ends_with('\\') {
                file.pop();
                file.push(' ');
                file.push_str(parts.next().unwrap());
            }
            println!("cargo:rerun-if-changed={file}");
        }
    }
}
