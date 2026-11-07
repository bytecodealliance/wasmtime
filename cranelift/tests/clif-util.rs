//! CLI coverage for `clif-util compile` and `clif-util run`.
//!
//! File settings are defaults. Explicit `--set` and `--target` options win.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const VECTOR_RETURN: &str = "\
set enable_multi_ret_implicit_sret=true
target riscv64 has_v

function %vec_ret() -> i8x16 {
block0:
    v0 = vconst.i8x16 0x00
    return v0
}
";

const VECTOR_RETURN_SPEED: &str = "\
set opt_level=speed
set enable_multi_ret_implicit_sret=true
target riscv64 has_v

function %vec_ret_speed() -> i8x16 {
block0:
    v0 = vconst.i8x16 0x00
    return v0
}
";

const SCALAR_RUN: &str = "\
function %test() -> i64 {
block0:
    v0 = iconst.i64 1
    return v0
}
; run: %test() == 1
";

fn clif_util() -> Command {
    Command::new(env!("CARGO_BIN_EXE_clif-util"))
}

fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn run_clif(args: &[&str], stdin: Option<&str>) -> Output {
    let mut command = clif_util();
    command.args(args);
    if let Some(stdin) = stdin {
        command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let mut child = command.spawn().expect("spawn clif-util");
        use std::io::Write;
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        child.wait_with_output().expect("wait clif-util")
    } else {
        command.output().expect("run clif-util")
    }
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed: {}",
        output_text(output)
    );
}

fn assert_failure_contains(output: &Output, expected: &str) {
    assert!(
        !output.status.success(),
        "command succeeded unexpectedly: {}",
        output_text(output)
    );
    let text = output_text(output);
    assert!(
        text.contains(expected),
        "error `{text}` does not contain `{expected}`"
    );
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("clif-util-{}-{nanos}-{n}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.path) {
            eprintln!(
                "failed to remove temporary directory {}: {error}",
                self.path.display()
            );
        }
    }
}

fn write_clif(dir: &Path, name: &str, contents: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, contents).unwrap();
    path
}

#[test]
fn compile_honors_file_vector_return_settings() {
    let output = run_clif(&["compile", "--target=riscv64", "-"], Some(VECTOR_RETURN));
    assert_success(&output);
}

#[test]
fn compile_set_overrides_file_and_rejects_bad_input() {
    let overridden = run_clif(
        &[
            "compile",
            "--set",
            "enable_multi_ret_implicit_sret=false",
            "--target=riscv64",
            "-",
        ],
        Some(VECTOR_RETURN),
    );
    assert!(
        !overridden.status.success(),
        "CLI --set did not override the file setting: {}",
        output_text(&overridden)
    );

    let bad_value = run_clif(
        &[
            "compile",
            "--set",
            "opt_level=banana",
            "--target=riscv64",
            "-",
        ],
        Some(VECTOR_RETURN),
    );
    assert_failure_contains(&bad_value, "opt_level=banana");

    let bad_name = run_clif(
        &[
            "compile",
            "--set",
            "not_a_real_setting",
            "--target=riscv64",
            "-",
        ],
        Some(VECTOR_RETURN),
    );
    assert_failure_contains(&bad_name, "not_a_real_setting");

    let bad_target = run_clif(&["compile", "--target=riscv32", "-"], Some(VECTOR_RETURN));
    assert_failure_contains(&bad_target, "riscv32");
}

#[test]
fn object_output_requires_one_resolved_configuration() {
    let dir = TempDir::new();
    let first = write_clif(dir.path(), "vec.clif", VECTOR_RETURN);
    let second = write_clif(dir.path(), "vec-speed.clif", VECTOR_RETURN_SPEED);
    let object = dir.path().join("out.o");

    let compatible_copy = "\
set enable_multi_ret_implicit_sret=true
target riscv64 has_v

function %vec_ret_b() -> i8x16 {
block0:
    v0 = vconst.i8x16 0x00
    return v0
}
";
    let copy = write_clif(dir.path(), "vec-b.clif", compatible_copy);
    let ok = run_clif(
        &[
            "compile",
            "--target=riscv64",
            "-o",
            object.to_str().unwrap(),
            first.to_str().unwrap(),
            copy.to_str().unwrap(),
        ],
        None,
    );
    assert_success(&ok);
    let bytes = fs::read(&object).expect("object output");
    assert!(!bytes.is_empty());

    let _ = fs::remove_file(&object);
    let conflict = run_clif(
        &[
            "compile",
            "--target=riscv64",
            "-o",
            object.to_str().unwrap(),
            first.to_str().unwrap(),
            second.to_str().unwrap(),
        ],
        None,
    );
    assert_failure_contains(&conflict, "vec-speed.clif");
    assert_failure_contains(&conflict, "incompatible");
    assert!(!object.exists(), "incompatible object compile wrote output");

    let independent = run_clif(
        &[
            "compile",
            "--target=riscv64",
            first.to_str().unwrap(),
            second.to_str().unwrap(),
        ],
        None,
    );
    assert_success(&independent);
}

#[test]
fn native_scalar_run() {
    let output = run_clif(&["run", "-v", "-"], Some(SCALAR_RUN));
    if !output.status.success() {
        let text = output_text(&output);
        let unsupported = text.contains("unsupported architecture")
            || text.contains("support for architecture disabled")
            || text.contains("x86 support requires SSE2");
        if unsupported {
            return;
        }
        panic!("native scalar run failed: {text}");
    }
}
