//! Run the tests in a single test file.

use crate::new_subtest;
use crate::subtest::{Context, SubTest};
use anyhow::Context as _;
use cranelift_codegen::ir::Function;
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::print_errors::pretty_verifier_error;
use cranelift_codegen::settings::Flags;
use cranelift_codegen::timing;
use cranelift_codegen::verify_function;
use cranelift_reader::{parse_test, Feature, IsaSpec, ParseOptions, TestFile};
use log::info;
use std::borrow::Cow;
use std::fs;
use std::path::Path;
use std::time;

/// Skip the tests which define features and for which there's a feature mismatch.
///
/// When a test must be skipped, returns an Option with a string containing an explanation why;
/// otherwise, return None.
fn skip_feature_mismatches(testfile: &TestFile) -> Option<&'static str> {
    let mut has_experimental_x64 = false;
    let mut has_experimental_arm32 = false;

    for feature in &testfile.features {
        if let Feature::With(name) = feature {
            match *name {
                "experimental_x64" => has_experimental_x64 = true,
                "experimental_arm32" => has_experimental_arm32 = true,
                _ => {}
            }
        }
    }

    // On the experimental x64 backend, skip tests which are not marked with the feature and
    // that want to run on the x86_64 target isa.
    #[cfg(feature = "experimental_x64")]
    if let IsaSpec::Some(ref isas) = testfile.isa_spec {
        if isas.iter().any(|isa| isa.name() == "x64") && !has_experimental_x64 {
            return Some("test requiring x86_64 not marked with experimental_x64");
        }
    }

    // On other targets, ignore tests marked as experimental_x64 only.
    #[cfg(not(feature = "experimental_x64"))]
    if has_experimental_x64 {
        return Some("missing support for experimental_x64");
    }

    // Don't run tests if the experimental support for arm32 is disabled.
    #[cfg(not(feature = "experimental_arm32"))]
    if has_experimental_arm32 {
        return Some("missing support for experimental_arm32");
    }

    None
}

/// Load `path` and run the test in it.
///
/// If running this test causes a panic, it will propagate as normal.
pub fn run(
    path: &Path,
    passes: Option<&[String]>,
    target: Option<&str>,
) -> anyhow::Result<time::Duration> {
    let _tt = timing::process_file();
    info!("---\nFile: {}", path.to_string_lossy());
    let started = time::Instant::now();
    let buffer =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let options = ParseOptions {
        target,
        passes,
        ..ParseOptions::default()
    };

    let testfile = match parse_test(&buffer, options) {
        Ok(testfile) => testfile,
        Err(e) => {
            if e.is_warning {
                println!(
                    "skipping test {:?} (line {}): {}",
                    path, e.location.line_number, e.message
                );
                return Ok(started.elapsed());
            }
            return Err(e)
                .context(format!("failed to parse {}", path.display()))
                .into();
        }
    };

    if let Some(msg) = skip_feature_mismatches(&testfile) {
        println!("skipped {:?}: {}", path, msg);
        return Ok(started.elapsed());
    }

    if testfile.functions.is_empty() {
        anyhow::bail!("no functions found");
    }

    // Parse the test commands.
    let mut tests = testfile
        .commands
        .iter()
        .map(new_subtest)
        .collect::<anyhow::Result<Vec<_>>>()?;

    // Flags to use for those tests that don't need an ISA.
    // This is the cumulative effect of all the `set` commands in the file.
    let flags = match testfile.isa_spec {
        IsaSpec::None(ref f) => f,
        IsaSpec::Some(ref v) => v.last().expect("Empty ISA list").flags(),
    };

    // Sort the tests so the mutators are at the end, and those that don't need the verifier are at
    // the front.
    tests.sort_by_key(|st| (st.is_mutating(), st.needs_verifier()));

    // Expand the tests into (test, flags, isa) tuples.
    let mut tuples = test_tuples(&tests, &testfile.isa_spec, flags)?;

    // Isolate the last test in the hope that this is the only mutating test.
    // If so, we can completely avoid cloning functions.
    let last_tuple = match tuples.pop() {
        None => anyhow::bail!("no test commands found"),
        Some(t) => t,
    };

    let file_path = path.to_string_lossy();
    for (func, details) in testfile.functions {
        let mut context = Context {
            preamble_comments: &testfile.preamble_comments,
            details,
            verified: false,
            flags,
            isa: None,
            file_path: file_path.as_ref(),
        };

        for tuple in &tuples {
            run_one_test(*tuple, Cow::Borrowed(&func), &mut context)?;
        }
        // Run the last test with an owned function which means it won't need to clone it before
        // mutating.
        run_one_test(last_tuple, Cow::Owned(func), &mut context)?;
    }

    Ok(started.elapsed())
}

// Given a slice of tests, generate a vector of (test, flags, isa) tuples.
fn test_tuples<'a>(
    tests: &'a [Box<dyn SubTest>],
    isa_spec: &'a IsaSpec,
    no_isa_flags: &'a Flags,
) -> anyhow::Result<Vec<(&'a dyn SubTest, &'a Flags, Option<&'a dyn TargetIsa>)>> {
    let mut out = Vec::new();
    for test in tests {
        if test.needs_isa() {
            match *isa_spec {
                IsaSpec::None(_) => {
                    // TODO: Generate a list of default ISAs.
                    anyhow::bail!("test {} requires an ISA", test.name());
                }
                IsaSpec::Some(ref isas) => {
                    for isa in isas {
                        out.push((&**test, isa.flags(), Some(&**isa)));
                    }
                }
            }
        } else {
            // This test doesn't require an ISA, and we only want to run one instance of it.
            // Still, give it an ISA ref if we happen to have a unique one.
            // For example, `test cat` can use this to print encodings and register names.
            out.push((&**test, no_isa_flags, isa_spec.unique_isa()));
        }
    }
    Ok(out)
}

fn run_one_test<'a>(
    tuple: (&'a dyn SubTest, &'a Flags, Option<&'a dyn TargetIsa>),
    func: Cow<Function>,
    context: &mut Context<'a>,
) -> anyhow::Result<()> {
    let (test, flags, isa) = tuple;
    let name = format!("{}({})", test.name(), func.name);
    info!("Test: {} {}", name, isa.map_or("-", TargetIsa::name));

    context.flags = flags;
    context.isa = isa;

    // Should we run the verifier before this test?
    if !context.verified && test.needs_verifier() {
        verify_function(&func, context.flags_or_isa()).map_err(|errors| {
            anyhow::anyhow!("{}", pretty_verifier_error(&func, isa, None, errors))
        })?;
        context.verified = true;
    }

    test.run(func, context).context(test.name())?;
    Ok(())
}
