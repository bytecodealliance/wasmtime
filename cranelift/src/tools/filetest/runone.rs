//! Run the tests in a single test file.

use std::borrow::{Borrow, Cow};
use std::path::Path;
use std::time;
use cretonne::ir::Function;
use cretonne::verify_function;
use cton_reader::parse_test;
use utils::read_to_string;
use filetest::{TestResult, new_subtest};
use filetest::subtest::{SubTest, Context, Result};

/// Load `path` and run the test in it.
///
/// If running this test causes a panic, it will propagate as normal.
pub fn run(path: &Path) -> TestResult {
    let started = time::Instant::now();
    let buffer = try!(read_to_string(path).map_err(|e| e.to_string()));
    let testfile = try!(parse_test(&buffer).map_err(|e| e.to_string()));
    if testfile.functions.is_empty() {
        return Err("no functions found".to_string());
    }
    // Parse the test commands.
    let mut tests = try!(testfile.commands.iter().map(new_subtest).collect::<Result<Vec<_>>>());

    // Sort the tests so the mutators are at the end, and those that don't need the verifier are at
    // the front.
    tests.sort_by_key(|st| (st.is_mutating(), st.needs_verifier()));

    // Isolate the last test in the hope that this is the only mutating test.
    // If so, we can completely avoid cloning functions.
    let last_test = match tests.pop() {
        None => return Err("no test commands found".to_string()),
        Some(t) => t,
    };

    for (func, details) in testfile.functions {
        let mut context = Context {
            details: details,
            verified: false,
        };

        for test in &tests {
            try!(run_one_test(test.borrow(), Cow::Borrowed(&func), &mut context));
        }
        // Run the last test with an owned function which means it won't need to clone it before
        // mutating.
        try!(run_one_test(last_test.borrow(), Cow::Owned(func), &mut context));
    }


    // TODO: Actually run the tests.
    Ok(started.elapsed())
}

fn run_one_test(test: &SubTest, func: Cow<Function>, context: &mut Context) -> Result<()> {
    let name = format!("{}({})", test.name(), func.name);

    // Should we run the verifier before this test?
    if !context.verified && test.needs_verifier() {
        try!(verify_function(&func).map_err(|e| e.to_string()));
        context.verified = true;
    }

    test.run(func, context).map_err(|e| format!("{}: {}", name, e))
}
