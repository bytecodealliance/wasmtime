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

        examples.insert((
            path.clone(),
            path.file_stem().unwrap().to_str().unwrap().to_owned(),
            dir,
        ));
    }

    println!("======== Building libwasmtime.a ===========");
    run(Command::new("cargo")
        .args(&["build"])
        .current_dir("crates/c-api"))?;

    for (example_path, example, is_dir) in examples {
        if example == "README" {
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
        if is_dir {
            let mut features_path = std::path::PathBuf::from(example_path);
            features_path.push("CARGO_FEATURES");
            if features_path.exists() {
                let features = std::fs::read_to_string(features_path)?;
                cargo_cmd.arg("--features").arg(features);
            }
        }
        run(&mut cargo_cmd)?;

        println!("======== C/C++ example `{}` ============", example);
        for extension in ["c", "cc"].iter() {
            let mut cmd = cc::Build::new()
                .opt_level(0)
                .cargo_metadata(false)
                .target(env!("TARGET"))
                .host(env!("TARGET"))
                .include("crates/c-api/include")
                .include("crates/c-api/wasm-c-api/include")
                .define("WASM_API_EXTERN", Some("")) // static linkage, not dynamic
                .warnings(false)
                .get_compiler()
                .to_command();

            let file = if is_dir {
                format!("examples/{}/main.{}", example, extension)
            } else {
                format!("examples/{}.{}", example, extension)
            };

            if extension == &"cc" && !std::path::Path::new(&file).exists() {
                // cc files are optional so we can skip them.
                continue;
            }

            cmd.arg(file);
            let exe = if cfg!(windows) {
                cmd.arg("target/debug/wasmtime.lib")
                    .arg("ws2_32.lib")
                    .arg("advapi32.lib")
                    .arg("userenv.lib")
                    .arg("ntdll.lib")
                    .arg("shell32.lib")
                    .arg("ole32.lib")
                    .arg("bcrypt.lib");
                if is_dir {
                    "main.exe".to_string()
                } else {
                    format!("./{}.exe", example)
                }
            } else {
                cmd.arg("target/debug/libwasmtime.a").arg("-o").arg("foo");
                "./foo".to_string()
            };
            if cfg!(target_os = "linux") {
                cmd.arg("-lpthread").arg("-ldl").arg("-lm");
            }
            run(&mut cmd)?;

            run(&mut Command::new(exe))?;
        }
    }

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
