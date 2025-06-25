use heck::*;
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use wit_component::ComponentEncoder;

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    Artifacts {
        out_dir,
        deps: HashSet::default(),
    }
    .build();
}

struct Artifacts {
    out_dir: PathBuf,
    deps: HashSet<String>,
}

struct Test {
    /// Not all tests can be built at build-time, for example C/C++ tests require
    /// the `WASI_SDK_PATH` environment variable which isn't available on all
    /// machines. The `Option` here encapsulates tests that were not able to be
    /// built.
    ///
    /// For tests that were not able to be built their error is deferred to
    /// test-time when the test is actually run. For C/C++ tests this means that
    /// only when running debuginfo tests does the error show up, for example.
    core_wasm: Option<PathBuf>,

    name: String,
}

impl Artifacts {
    fn build(&mut self) {
        // Build adapters used below for componentization.
        let reactor_adapter = self.build_adapter("reactor", &[]);
        let command_adapter =
            self.build_adapter("command", &["--no-default-features", "--features=command"]);
        let proxy_adapter =
            self.build_adapter("proxy", &["--no-default-features", "--features=proxy"]);

        // Build all test programs both in Rust and C/C++.
        let mut tests = Vec::new();
        self.build_rust_tests(&mut tests);
        self.build_non_rust_tests(&mut tests);

        // With all our `tests` now compiled generate various macos for each
        // test along with constants pointing to various paths. Note that
        // components are created here as well from core modules.
        let mut kinds = BTreeMap::new();
        let mut generated_code = String::new();
        let missing_sdk_path =
            PathBuf::from("Asset not compiled, WASI_SDK_PATH missing at compile time");
        for test in tests.iter() {
            let camel = test.name.to_shouty_snake_case();

            generated_code += &format!(
                "pub const {camel}: &'static str = {:?};\n",
                test.core_wasm.as_deref().unwrap_or(&missing_sdk_path)
            );

            // Bucket, based on the name of the test, into a "kind" which
            // generates a `foreach_*` macro below.
            let kind = match test.name.as_str() {
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
                s if s.starts_with("tls_") => "tls",
                s if s.starts_with("async_") => "async",
                s if s.starts_with("p3_http_") => "p3_http",
                s if s.starts_with("p3_api_") => "p3_api",
                s if s.starts_with("p3_") => "p3",
                // If you're reading this because you hit this panic, either add
                // it to a test suite above or add a new "suite". The purpose of
                // the categorization above is to have a static assertion that
                // tests added are actually run somewhere, so as long as you're
                // also adding test code somewhere that's ok.
                other => {
                    panic!("don't know how to classify test name `{other}` to a kind")
                }
            };
            if !kind.is_empty() {
                kinds.entry(kind).or_insert(Vec::new()).push(&test.name);
            }

            // Generate a component from each test.
            if test.name == "dwarf_imported_memory"
                || test.name == "dwarf_shared_memory"
                || test.name.starts_with("nn_witx")
            {
                continue;
            }
            let adapter = match test.name.as_str() {
                "reactor" => &reactor_adapter,
                s if s.starts_with("api_proxy") => &proxy_adapter,
                _ => &command_adapter,
            };
            let path = match &test.core_wasm {
                Some(path) => self.compile_component(path, adapter),
                None => missing_sdk_path.clone(),
            };
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

        std::fs::write(self.out_dir.join("gen.rs"), generated_code).unwrap();
    }

    fn build_rust_tests(&mut self, tests: &mut Vec<Test>) {
        println!("cargo:rerun-if-env-changed=MIRI_TEST_CWASM_DIR");
        let release_mode = env::var_os("MIRI_TEST_CWASM_DIR").is_some();

        let mut cmd = cargo();
        cmd.arg("build");
        if release_mode {
            cmd.arg("--release");
        }
        cmd.arg("--target=wasm32-wasip1")
            .arg("--package=test-programs")
            .env("CARGO_TARGET_DIR", &self.out_dir)
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
            .filter(move |t| t.kind == &[cargo_metadata::TargetKind::Bin])
            .map(|t| &t.name)
            .collect::<Vec<_>>();

        for target in targets {
            let wasm = self
                .out_dir
                .join("wasm32-wasip1")
                .join(if release_mode { "release" } else { "debug" })
                .join(format!("{target}.wasm"));
            self.read_deps_of(&wasm);
            tests.push(Test {
                core_wasm: Some(wasm),
                name: target.to_string(),
            })
        }
    }

    // Build the WASI Preview 1 adapter, and get the binary:
    fn build_adapter(&mut self, name: &str, features: &[&str]) -> Vec<u8> {
        let mut cmd = cargo();
        cmd.arg("build")
            .arg("--release")
            .arg("--package=wasi-preview1-component-adapter")
            .arg("--target=wasm32-unknown-unknown")
            .env("CARGO_TARGET_DIR", &self.out_dir)
            .env("RUSTFLAGS", rustflags())
            .env_remove("CARGO_ENCODED_RUSTFLAGS");
        for f in features {
            cmd.arg(f);
        }
        eprintln!("running: {cmd:?}");
        let status = cmd.status().unwrap();
        assert!(status.success());

        let artifact = self
            .out_dir
            .join("wasm32-unknown-unknown")
            .join("release")
            .join("wasi_snapshot_preview1.wasm");
        let adapter = self
            .out_dir
            .join(format!("wasi_snapshot_preview1.{name}.wasm"));
        std::fs::copy(&artifact, &adapter).unwrap();
        self.read_deps_of(&artifact);
        println!("wasi {name} adapter: {:?}", &adapter);
        fs::read(&adapter).unwrap()
    }

    // Compile a component, return the path of the binary:
    fn compile_component(&self, wasm: &Path, adapter: &[u8]) -> PathBuf {
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

    fn build_non_rust_tests(&mut self, tests: &mut Vec<Test>) {
        const ASSETS_REL_SRC_DIR: &'static str = "../src/bin";
        println!("cargo:rerun-if-changed={ASSETS_REL_SRC_DIR}");

        for entry in fs::read_dir(ASSETS_REL_SRC_DIR).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
            match path.extension().and_then(|s| s.to_str()) {
                // Compile C/C++ tests with clang
                Some("c") | Some("cpp") | Some("cc") => self.build_c_or_cpp_test(path, name, tests),

                // just a header, part of another test.
                Some("h") => {}

                // Convert the text format to binary and use it as a test.
                Some("wat") => {
                    let wasm = wat::parse_file(&path).unwrap();
                    let core_wasm = self.out_dir.join(&name).with_extension("wasm");
                    fs::write(&core_wasm, &wasm).unwrap();
                    tests.push(Test {
                        name,
                        core_wasm: Some(core_wasm),
                    });
                }

                // these are built above in `build_rust_tests`
                Some("rs") => {}

                // Prevent stray files for now that we don't understand.
                Some(_) => panic!("unknown file extension on {path:?}"),

                None => unreachable!(),
            }
        }
    }

    fn build_c_or_cpp_test(&mut self, path: PathBuf, name: String, tests: &mut Vec<Test>) {
        println!("compiling {path:?}");
        println!("cargo:rerun-if-changed={}", path.display());
        let contents = std::fs::read_to_string(&path).unwrap();
        let config =
            wasmtime_test_util::wast::parse_test_config::<CTestConfig>(&contents, "//!").unwrap();

        if config.skip {
            return;
        }

        // The debug tests relying on these assets are ignored by default,
        // so we cannot force the requirement of having a working WASI SDK
        // install on everyone. At the same time, those tests (due to their
        // monolithic nature), are always compiled, so we still have to
        // produce the path constants. To solve this, we move the failure
        // of missing WASI SDK from compile time to runtime by producing
        // fake paths (that themselves will serve as diagnostic messages).
        let wasi_sdk_path = match env::var_os("WASI_SDK_PATH") {
            Some(path) => PathBuf::from(path),
            None => {
                tests.push(Test {
                    name,
                    core_wasm: None,
                });
                return;
            }
        };

        let wasm_path = self.out_dir.join(&name).with_extension("wasm");

        let mut cmd = Command::new(wasi_sdk_path.join("bin/wasm32-wasip1-clang"));
        cmd.arg(&path);
        for file in config.extra_files.iter() {
            cmd.arg(path.parent().unwrap().join(file));
        }
        cmd.arg("-g");
        cmd.args(&config.flags);
        cmd.arg("-o");
        cmd.arg(&wasm_path);
        // If optimizations are enabled, clang will look for wasm-opt in PATH
        // and run it. This will strip DWARF debug info, which we don't want.
        cmd.env("PATH", "");
        println!("running: {cmd:?}");
        let result = cmd.status().expect("failed to spawn clang");
        assert!(result.success());

        if config.dwp {
            let mut dwp = Command::new(wasi_sdk_path.join("bin/llvm-dwp"));
            dwp.arg("-e")
                .arg(&wasm_path)
                .arg("-o")
                .arg(self.out_dir.join(&name).with_extension("dwp"));
            assert!(dwp.status().expect("failed to spawn llvm-dwp").success());
        }

        tests.push(Test {
            name,
            core_wasm: Some(wasm_path),
        });
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
    fn read_deps_of(&mut self, artifact: &Path) {
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
                if !self.deps.contains(&file) {
                    println!("cargo:rerun-if-changed={file}");
                    self.deps.insert(file);
                }
            }
        }
    }
}

#[derive(serde_derive::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct CTestConfig {
    #[serde(default)]
    flags: Vec<String>,
    #[serde(default)]
    extra_files: Vec<String>,
    #[serde(default)]
    dwp: bool,
    #[serde(default)]
    skip: bool,
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

fn rustflags() -> &'static str {
    match option_env!("RUSTFLAGS") {
        // If we're in CI which is denying warnings then deny warnings to code
        // built here too to keep the tree warning-free.
        Some(s) if s.contains("-D warnings") => "-D warnings",
        _ => "",
    }
}
