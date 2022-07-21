use anyhow::Context;
use serde_json::Value;
use std::process::Command;
use std::str::from_utf8;

fn main() -> anyhow::Result<()> {
    let example_to_run = std::env::args().nth(1);

    let mut rust_targets: Vec<String> = Vec::new();
    match Command::new("cargo").arg("read-manifest").output() {
        Ok(cargo_manifest_output) => match from_utf8(cargo_manifest_output.stdout.as_slice()) {
            Ok(stdout) => {
                let cargo_manifest: Value = serde_json::from_str(stdout)?;
                for target in cargo_manifest["targets"].as_array().unwrap() {
                    let is_example = target["kind"].is_array()
                        && target["kind"].as_array().unwrap()[0].as_str().unwrap() == "example";
                    if is_example {
                        rust_targets.push(target["name"].as_str().unwrap().to_string());
                    }
                }
            }
            Err(error) => panic!("Problem getting cargo manifest stdout: {:?}", error),
        },
        Err(error) => panic!("Problem getting cargo manifest: {:?}", error),
    };

    if let Some(example) = &example_to_run {
        // If explicit example is provided, use that instead
        rust_targets.retain(|e| *e == *example);
    }

    println!("======== Prepare C/C++ CMake project ===========");
    run(Command::new("cmake")
        .arg("-Sexamples")
        .arg("-Bexamples/build")
        .arg("-DBUILD_SHARED_LIBS=OFF"))?;

    let mut c_targets: Vec<String> = Vec::new();
    match Command::new("cmake")
        .arg("--build")
        .arg("examples/build")
        .arg("--target")
        .arg("help")
        .output()
    {
        Ok(cmake_help_output) => match from_utf8(cmake_help_output.stdout.as_slice()) {
            Ok(stdout) => {
                for possible_target_line in stdout.lines() {
                    let possible_location = possible_target_line.find("wasmtime-");
                    if let Some(location) = possible_location {
                        let line = &possible_target_line
                            [(location + "wasmtime-".len())..possible_target_line.len()];
                        // "crate" is the wasmtime-c-api itself
                        if line != "crate" {
                            c_targets.push(line.to_string());
                        }
                    }
                }
            }
            Err(error) => panic!("Problem getting cmake help stdout: {:?}", error),
        },
        Err(error) => panic!("Problem getting cmake help: {:?}", error),
    };

    if let Some(example) = &example_to_run {
        // If explicit example is provided, use that instead
        c_targets.retain(|e| *e == *example);
    }

    for example in rust_targets {
        if example == "fib-debug" || example == "tokio" || example == "wasi" {
            println!("======== Rust wasm file `{}` ============", example);
            let target = if example == "fib-debug" {
                "wasm32-unknown-unknown"
            } else {
                "wasm32-wasi"
            };
            run(Command::new("cargo")
                .arg("build")
                .arg("-p")
                .arg(format!("example-{}-wasm", example))
                .arg("--target")
                .arg(target))?;
        }
        println!("======== Rust example `{}` ============", example);
        let mut cargo_cmd = Command::new("cargo");
        cargo_cmd.arg("run").arg("--example").arg(&example);

        if example.contains("tokio") {
            cargo_cmd.arg("--features").arg("wasmtime-wasi/tokio");
        }
        run(&mut cargo_cmd)?;
    }

    for example in c_targets {
        println!("======== C/C++ example `{}` ============", example);
        run(Command::new("cmake")
            .arg("--build")
            .arg("examples/build")
            .arg("--target")
            .arg(format!("wasmtime-{}", example))
            .arg("--config")
            .arg("Debug"))?;

        if cfg!(windows) {
            run(&mut Command::new(format!(
                "examples/build/wasmtime-{}.exe",
                example
            )))?;
        } else {
            run(&mut Command::new(format!(
                "examples/build/wasmtime-{}",
                example
            )))?;
        };
    }

    println!("======== Remove examples binaries ===========");
    std::fs::remove_dir_all("examples/build")?;

    Ok(())
}

fn run(cmd: &mut Command) -> anyhow::Result<()> {
    (|| -> anyhow::Result<()> {
        let s = cmd.status()?;
        if !s.success() {
            anyhow::bail!("Exited with failure status: {}", s);
        }
        Ok(())
    })()
    .with_context(|| format!("failed to run `{:?}`", cmd))
}
