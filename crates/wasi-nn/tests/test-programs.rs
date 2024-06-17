//! Run the wasi-nn tests in `crates/test-programs`.
//!
//! It may be difficult to run to run all tests on all platforms; we check the
//! pre-requisites for each test dynamically (see [`check`]). Using
//! `libtest-mimic` allows us then to dynamically ignore tests that cannot run
//! on the current machine.
//!
//! There are two modes these tests run in:
//! - "ignore if unavailable" mode: if the checks for a test fail (e.g., the
//!   backend is not installed, test artifacts cannot download, we're on the
//!   wrong platform), the test is ignored.
//! - "fail if unavailable" mode: when the `CI` or `FORCE_WASINN_TEST_CHECK`
//!   environment variables are set, any checks that fail cause the test to fail
//!   early.

mod check;
mod exec;

use anyhow::{bail, Result};
use libtest_mimic::{Arguments, Trial};
use std::{borrow::Cow, env};
use test_programs_artifacts::*;
use wasmtime_wasi_nn::{backend, Backend};

fn main() -> Result<()> {
    if cfg!(miri) {
        return Ok(());
    }

    // Gather a list of the test-program names.
    let mut programs = Vec::new();
    macro_rules! add_to_list {
        ($name:ident) => {
            programs.push(stringify!($name));
        };
    }
    foreach_nn!(add_to_list);

    // Make ignored tests turn into failures.
    let error_on_failed_check =
        env::var_os("CI").is_some() || env::var_os("FORCE_WASINN_TEST_CHECK").is_some();

    // Inform `libtest-mimic` how to run each test program.
    let arguments = Arguments::from_args();
    let mut trials = Vec::new();
    for program in programs {
        // Either ignore the test if it cannot run (i.e., downgrade `Fail` to
        // `Ignore`) or pre-emptively fail it if `error_on_failed_check` is set.
        let (run_test, mut check) = check_test_program(program);
        if !error_on_failed_check {
            check = check.downgrade_failure(); // Downgrade `Fail` to `Ignore`.
        }
        let should_ignore = check.is_ignore();
        if arguments.nocapture && should_ignore {
            println!("> ignoring {program}: {}", check.reason());
        }
        let trial = Trial::test(program, move || {
            run_test().map_err(|e| format!("{:?}", e).into())
        })
        .with_ignored_flag(should_ignore);
        trials.push(trial);
    }

    // Run the tests.
    libtest_mimic::run(&arguments, trials).exit()
}

/// Return the test program to run and a check that must pass for the test to
/// run.
fn check_test_program(name: &str) -> (fn() -> Result<()>, IgnoreCheck) {
    use IgnoreCheck::*;
    match name {
        "nn_image_classification" => (
            nn_image_classification,
            if !cfg!(target_arch = "x86_64") {
                Fail("requires x86_64".into())
            } else if !cfg!(target_os = "linux") && !cfg!(target_os = "windows") {
                Fail("requires linux or windows".into())
            } else if let Err(e) = check::openvino::is_installed() {
                Fail(e.to_string().into())
            } else {
                Run
            },
        ),
        "nn_image_classification_named" => (
            nn_image_classification_named,
            if !cfg!(target_arch = "x86_64") {
                Fail("requires x86_64".into())
            } else if !cfg!(target_os = "linux") && !cfg!(target_os = "windows") {
                Fail("requires linux or windows or macos".into())
            } else if let Err(e) = check::openvino::is_installed() {
                Fail(e.to_string().into())
            } else {
                Run
            },
        ),
        "nn_image_classification_onnx" => (
            nn_image_classification_onnx,
            #[cfg(feature = "onnx")]
            if !cfg!(target_arch = "x86_64") && !cfg!(target_arch = "aarch64") {
                Fail("requires x86_64 or aarch64".into())
            } else if !cfg!(target_os = "linux")
                && !cfg!(target_os = "windows")
                && !cfg!(target_os = "macos")
            {
                Fail("requires linux, windows, or macos".into())
            } else {
                Run
            },
            #[cfg(not(feature = "onnx"))]
            Ignore("requires the `onnx` feature".into()),
        ),
        "nn_image_classification_winml" => (
            nn_image_classification_winml,
            #[cfg(all(feature = "winml", target_os = "windows"))]
            if !cfg!(target_arch = "x86_64") {
                Fail("requires x86_64".into())
            } else if cfg!(target_os = "windows") {
                Fail("requires windows".into())
            } else if let Err(e) = check::winml::is_available() {
                Fail(e.to_string().into())
            } else {
                Run
            },
            #[cfg(not(all(feature = "winml", target_os = "windows")))]
            Ignore("requires the `winml` feature on windows".into()),
        ),
        _ => panic!("unknown test program: {} (add to this `match`)", name),
    }
}

fn nn_image_classification() -> Result<()> {
    check::openvino::is_installed()?;
    check::openvino::are_artifacts_available()?;
    let backend = Backend::from(backend::openvino::OpenvinoBackend::default());
    exec::run(NN_IMAGE_CLASSIFICATION, backend, false)
}

fn nn_image_classification_named() -> Result<()> {
    check::openvino::is_installed()?;
    check::openvino::are_artifacts_available()?;
    let backend = Backend::from(backend::openvino::OpenvinoBackend::default());
    exec::run(NN_IMAGE_CLASSIFICATION_NAMED, backend, true)
}

#[cfg(feature = "onnx")]
fn nn_image_classification_onnx() -> Result<()> {
    check::onnx::are_artifacts_available()?;
    let backend = Backend::from(backend::onnxruntime::OnnxBackend::default());
    exec::run(NN_IMAGE_CLASSIFICATION_ONNX, backend, false)
}

#[cfg(not(feature = "onnx"))]
fn nn_image_classification_onnx() -> Result<()> {
    bail!("this test requires the `onnx` feature")
}

#[cfg(all(feature = "winml", target_os = "windows"))]
fn nn_image_classification_winml() -> Result<()> {
    check::winml::is_available()?;
    check::onnx::are_artifacts_available()?;
    let backend = Backend::from(backend::winml::WinMLBackend::default());
    exec::run(NN_IMAGE_CLASSIFICATION_WINML, backend, false)
}

#[cfg(not(all(feature = "winml", target_os = "windows")))]
fn nn_image_classification_winml() -> Result<()> {
    bail!("this test requires the `winml` feature and only runs on windows")
}

/// Helper for keeping track of what tests should do when pre-test checks fail.
#[derive(Clone)]
enum IgnoreCheck {
    Run,
    Ignore(Cow<'static, str>),
    Fail(Cow<'static, str>),
}

impl IgnoreCheck {
    fn reason(&self) -> &str {
        match self {
            IgnoreCheck::Run => panic!("cannot get reason for `Run`"),
            IgnoreCheck::Ignore(reason) => reason,
            IgnoreCheck::Fail(reason) => reason,
        }
    }

    fn downgrade_failure(self) -> Self {
        if let IgnoreCheck::Fail(reason) = self {
            IgnoreCheck::Ignore(reason)
        } else {
            self
        }
    }

    fn is_ignore(&self) -> bool {
        matches!(self, IgnoreCheck::Ignore(_))
    }
}
