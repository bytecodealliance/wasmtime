use std::collections::BTreeSet;
use std::process::Command;

fn main() {
    let example_to_run = std::env::args().nth(1);
    let examples = std::fs::read_dir("examples").unwrap();
    let examples = examples
        .filter_map(|e| {
            let e = e.unwrap();
            let path = e.path();
            let dir = e.metadata().unwrap().is_dir();
            if let Some("wat") = path.extension().and_then(|s| s.to_str()) {
                return None;
            }

            Some((path.file_stem().unwrap().to_str().unwrap().to_owned(), dir))
        })
        .collect::<BTreeSet<_>>();

    println!("======== Building libwasmtime.a ===========");
    run(Command::new("cargo")
        .args(&["build"])
        .current_dir("crates/c-api"));

    for (example, is_dir) in examples {
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
                .arg(target));
        }
        println!("======== Rust example `{}` ============", example);
        run(Command::new("cargo")
            .arg("run")
            .arg("--example")
            .arg(&example));

        println!("======== C example `{}` ============", example);
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
        if is_dir {
            cmd.arg(format!("examples/{}/main.c", example));
        } else {
            cmd.arg(format!("examples/{}.c", example));
        }
        let exe = if cfg!(windows) {
            cmd.arg("target/debug/wasmtime.lib")
                .arg("ws2_32.lib")
                .arg("advapi32.lib")
                .arg("userenv.lib")
                .arg("ntdll.lib")
                .arg("shell32.lib")
                .arg("ole32.lib");
            "./main.exe"
        } else {
            cmd.arg("target/debug/libwasmtime.a").arg("-o").arg("foo");
            "./foo"
        };
        if cfg!(target_os = "linux") {
            cmd.arg("-lpthread").arg("-ldl").arg("-lm");
        }
        run(&mut cmd);

        run(&mut Command::new(exe));
    }
}

fn run(cmd: &mut Command) {
    let s = cmd.status().unwrap();
    if !s.success() {
        eprintln!("failed to run {:?}", cmd);
        eprintln!("status: {}", s);
        std::process::exit(1);
    }
}
