//! Test command for testing the optimization phases.
//!
//! The `optimize` test command runs each function through the
//! optimization passes, but not lowering or regalloc. The output for
//! filecheck purposes is the resulting CLIF.
//!
//! Some legalization may be ISA-specific, so this requires an ISA
//! (for now).

use crate::subtest::{run_filecheck, Context, SubTest};
use anyhow::{bail, Result};
use cranelift_codegen::ir;
use cranelift_reader::{TestCommand, TestOption};
use similar::TextDiff;
use std::borrow::Cow;
use std::env;

struct TestOptimize {
    /// Flag indicating that the text expectation, comments after the function,
    /// must be a precise 100% match on the compiled output of the function.
    /// This test assertion is also automatically-update-able to allow tweaking
    /// the code generator and easily updating all affected tests.
    precise_output: bool,
}

pub fn subtest(parsed: &TestCommand) -> Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "optimize");
    let mut test = TestOptimize {
        precise_output: false,
    };
    for option in parsed.options.iter() {
        match option {
            TestOption::Flag("precise-output") => test.precise_output = true,
            _ => anyhow::bail!("unknown option on {}", parsed),
        }
    }
    Ok(Box::new(test))
}

impl SubTest for TestOptimize {
    fn name(&self) -> &'static str {
        "optimize"
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn needs_isa(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<ir::Function>, context: &Context) -> Result<()> {
        let isa = context.isa.expect("optimize needs an ISA");
        let mut comp_ctx = cranelift_codegen::Context::for_function(func.into_owned());

        comp_ctx
            .optimize(isa)
            .map_err(|e| crate::pretty_anyhow_error(&comp_ctx.func, e))?;

        let clif = format!("{:?}", comp_ctx.func);
        let actual: Vec<_> = clif.lines().collect();

        if self.precise_output {
            check_precise_output(&actual, context)
        } else {
            run_filecheck(&clif, context)
        }
    }
}

fn check_precise_output(actual: &[&str], context: &Context) -> Result<()> {
    // Use the comments after the function to build the test expectation.
    let expected = context
        .details
        .comments
        .iter()
        .filter(|c| !c.text.starts_with(";;"))
        .map(|c| c.text.strip_prefix("; ").unwrap_or(c.text))
        .collect::<Vec<_>>();

    // If the expectation matches what we got, then there's nothing to do.
    if actual == expected {
        return Ok(());
    }

    // If we're supposed to automatically update the test, then do so here.
    if env::var("CRANELIFT_TEST_BLESS").unwrap_or(String::new()) == "1" {
        return update_test(&actual, context);
    }

    // Otherwise this test has failed, and we can print out as such.
    bail!(
        "compilation of function on line {} does not match\n\
         the text expectation\n\
         \n\
         {}\n\
         \n\
         This test assertion can be automatically updated by setting the\n\
         CRANELIFT_TEST_BLESS=1 environment variable when running this test.
         ",
        context.details.location.line_number,
        TextDiff::from_slices(&expected, &actual)
            .unified_diff()
            .header("expected", "actual")
    )
}

fn update_test(output: &[&str], context: &Context) -> Result<()> {
    context
        .file_update
        .update_at(&context.details.location, |new_test, old_test| {
            // blank newline after the function
            new_test.push_str("\n");

            // Splice in the test output
            for output in output {
                new_test.push_str("; ");
                new_test.push_str(output);
                new_test.push_str("\n");
            }

            // blank newline after test assertion
            new_test.push_str("\n");

            // Drop all remaining commented lines (presumably the old test expectation),
            // but after we hit a real line then we push all remaining lines.
            let mut in_next_function = false;
            for line in old_test {
                if !in_next_function
                    && (line.trim().is_empty()
                        || (line.starts_with(";") && !line.starts_with(";;")))
                {
                    continue;
                }
                in_next_function = true;
                new_test.push_str(line);
                new_test.push_str("\n");
            }
        })
}
