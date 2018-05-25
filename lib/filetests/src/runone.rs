//! Run the tests in a single test file.

use cretonne_codegen::ir::Function;
use cretonne_codegen::isa::TargetIsa;
use cretonne_codegen::print_errors::pretty_verifier_error;
use cretonne_codegen::settings::Flags;
use cretonne_codegen::timing;
use cretonne_codegen::verify_function;
use cretonne_reader::IsaSpec;
use cretonne_reader::parse_test;
use std::borrow::Cow;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::time;
use subtest::{Context, Result, SubTest};
use {new_subtest, TestResult};

/// Read an entire file into a string.
fn read_to_string<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;
    Ok(buffer)
}

/// Load `path` and run the test in it.
///
/// If running this test causes a panic, it will propagate as normal.
pub fn run(path: &Path) -> TestResult {
    let _tt = timing::process_file();
    dbg!("---\nFile: {}", path.to_string_lossy());
    let started = time::Instant::now();
    let buffer = read_to_string(path).map_err(|e| e.to_string())?;
    let testfile = parse_test(&buffer).map_err(|e| e.to_string())?;
    if testfile.functions.is_empty() {
        return Err("no functions found".to_string());
    }

    // Parse the test commands.
    let mut tests = testfile
        .commands
        .iter()
        .map(new_subtest)
        .collect::<Result<Vec<_>>>()?;

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
        None => return Err("no test commands found".to_string()),
        Some(t) => t,
    };

    for (func, details) in testfile.functions {
        let mut context = Context {
            preamble_comments: &testfile.preamble_comments,
            details,
            verified: false,
            flags,
            isa: None,
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
    tests: &'a [Box<SubTest>],
    isa_spec: &'a IsaSpec,
    no_isa_flags: &'a Flags,
) -> Result<Vec<(&'a SubTest, &'a Flags, Option<&'a TargetIsa>)>> {
    let mut out = Vec::new();
    for test in tests {
        if test.needs_isa() {
            match *isa_spec {
                IsaSpec::None(_) => {
                    // TODO: Generate a list of default ISAs.
                    return Err(format!("test {} requires an ISA", test.name()));
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
    tuple: (&'a SubTest, &'a Flags, Option<&'a TargetIsa>),
    func: Cow<Function>,
    context: &mut Context<'a>,
) -> Result<()> {
    let (test, flags, isa) = tuple;
    let name = format!("{}({})", test.name(), func.name);
    dbg!("Test: {} {}", name, isa.map_or("-", TargetIsa::name));

    context.flags = flags;
    context.isa = isa;

    // Should we run the verifier before this test?
    if !context.verified && test.needs_verifier() {
        verify_function(&func, context.flags_or_isa())
            .map_err(|e| pretty_verifier_error(&func, isa, &e))?;
        context.verified = true;
    }

    test.run(func, context)
        .map_err(|e| format!("{}: {}", name, e))
}
