use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // If we're not able to build our sibling crate because this is an
    // isolated published copy of the crate (e.g. to crates.io), fail
    // cleanly. The gdbstub component is included only in the published
    // wasmtime build artifacts or builds from the source tree, not
    // crates.io.
    if !PathBuf::from("../Cargo.toml").exists() {
        std::fs::write(
            out_dir.join("gen.rs"),
            concat!(
                "compile_error!(\"Cannot build gdbstub Wasm artifact when ",
                "compiled as a published crate from crates.io.\");\n"
            ),
        )
        .unwrap();
        return;
    }

    let mut cmd = cargo();
    cmd.arg("build")
        .arg("--release")
        .arg("--target=wasm32-wasip2")
        .arg("--package=wasmtime-internal-gdbstub-component")
        .env("CARGO_TARGET_DIR", &out_dir)
        .env("RUSTFLAGS", rustflags())
        .env_remove("CARGO_ENCODED_RUSTFLAGS");
    eprintln!("running: {cmd:?}");
    let status = cmd.status().unwrap();
    assert!(status.success());

    let wasm = out_dir
        .join("wasm32-wasip2")
        .join("release")
        .join("wasmtime_internal_gdbstub_component.wasm");

    // Read dep-info to get proper rerun-if-changed directives.
    let deps_file = wasm.with_extension("d");
    if let Ok(contents) = std::fs::read_to_string(&deps_file) {
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

    let generated = format!("pub const GDBSTUB_COMPONENT: &[u8] = include_bytes!({wasm:?});\n");
    std::fs::write(out_dir.join("gen.rs"), generated).unwrap();
}

fn cargo() -> Command {
    let mut cargo = Command::new("cargo");
    if std::env::var("CARGO_CFG_MIRI").is_ok() {
        cargo.env_remove("RUSTC").env_remove("RUSTC_WRAPPER");
    }
    cargo
}

fn rustflags() -> &'static str {
    match option_env!("RUSTFLAGS") {
        Some(s) if s.contains("-D warnings") => "-D warnings",
        _ => "",
    }
}
