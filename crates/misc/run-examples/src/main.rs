use anyhow::Context;
use std::collections::BTreeSet;
use std::process::Command;

fn main() -> anyhow::Result<()> {
    let example_to_run = std::env::args().nth(1);
    let mut examples = BTreeSet::new();
    for e in std::fs::read_dir("examples")? {
        let e = e?;
        let path = e.path();
        let dir = e.metadata()?.is_dir();
        if let Some("wat") = path.extension().and_then(|s| s.to_str()) {
            continue;
        }

        examples.insert((path.file_stem().unwrap().to_str().unwrap().to_owned(), dir));
    }

    println!("======== Prepare C/C++ CMake project ===========");
    run(Command::new("cmake")
        .arg("-Sexamples")
        .arg("-Bexamples/build")
        .arg("-DBUILD_SHARED_LIBS=OFF"))?;

    for (example, is_dir) in examples {
        if example == "README" || example == "CMakeLists" || example == "build" {
            continue;
        }
        if let Some(example_to_run) = &example_to_run {
            if !example.contains(&example_to_run[..]) {
                continue;
            }
        }
        if is_dir {
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

        println!("======== C/C++ example `{}` ============", example);
        let c_file = format!("examples/{}.c", example);
        if std::path::Path::new(&c_file).exists() {
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
