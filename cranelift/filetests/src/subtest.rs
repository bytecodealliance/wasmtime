//! `SubTest` trait.

use crate::runone::FileUpdate;
use anyhow::Context as _;
use anyhow::{bail, Result};
use cranelift_codegen::ir::Function;
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::settings::{Flags, FlagsOrIsa};
use cranelift_reader::{Comment, Details, TestFile};
use filecheck::{Checker, CheckerBuilder, NO_VARIABLES};
use log::info;
use similar::TextDiff;
use std::borrow::Cow;
use std::env;

/// Context for running a test on a single function.
pub struct Context<'a> {
    /// Comments from the preamble f the test file. These apply to all functions.
    pub preamble_comments: &'a [Comment<'a>],

    /// Additional details about the function from the parser.
    pub details: &'a Details<'a>,

    /// ISA-independent flags for this test.
    pub flags: &'a Flags,

    /// Target ISA to test against. Only guaranteed to be present for sub-tests whose `needs_isa`
    /// method returned `true`. For other sub-tests, this is set if the test file has a unique ISA.
    pub isa: Option<&'a dyn TargetIsa>,

    /// Full path to the file containing the test.
    #[expect(dead_code, reason = "may get used later")]
    pub file_path: &'a str,

    /// Context used to update the original `file_path` in-place with its test
    /// expectations if so configured in the environment.
    pub file_update: &'a FileUpdate,
}

impl<'a> Context<'a> {
    /// Get a `FlagsOrIsa` object for passing to the verifier.
    pub fn flags_or_isa(&self) -> FlagsOrIsa<'a> {
        FlagsOrIsa {
            flags: self.flags,
            isa: self.isa,
        }
    }
}

/// Common interface for implementations of test commands.
///
/// Each `.clif` test file may contain multiple test commands, each represented by a `SubTest`
/// trait object.
pub trait SubTest {
    /// Name identifying this subtest. Typically the same as the test command.
    fn name(&self) -> &'static str;

    /// Should the verifier be run on the function before running the test?
    fn needs_verifier(&self) -> bool {
        true
    }

    /// Does this test mutate the function when it runs?
    /// This is used as a hint to avoid cloning the function needlessly.
    fn is_mutating(&self) -> bool {
        false
    }

    /// Does this test need a `TargetIsa` trait object?
    fn needs_isa(&self) -> bool {
        false
    }

    /// Runs the entire subtest for a given target, invokes [Self::run] for running
    /// individual tests.
    fn run_target<'a>(
        &self,
        testfile: &TestFile,
        file_update: &mut FileUpdate,
        file_path: &'a str,
        flags: &'a Flags,
        isa: Option<&'a dyn TargetIsa>,
    ) -> anyhow::Result<()> {
        for (func, details) in &testfile.functions {
            info!(
                "Test: {}({}) {}",
                self.name(),
                func.name,
                isa.map_or("-", TargetIsa::name)
            );

            let context = Context {
                preamble_comments: &testfile.preamble_comments,
                details,
                flags,
                isa,
                file_path: file_path.as_ref(),
                file_update,
            };

            self.run(Cow::Borrowed(&func), &context)
                .context(self.name())?;
        }

        Ok(())
    }

    /// Run this test on `func`.
    fn run(&self, func: Cow<Function>, context: &Context) -> anyhow::Result<()>;
}

/// Run filecheck on `text`, using directives extracted from `context`.
pub fn run_filecheck(text: &str, context: &Context) -> anyhow::Result<()> {
    log::debug!(
        "Filecheck Input:\n\
         =======================\n\
         {text}\n\
         ======================="
    );
    let checker = build_filechecker(context)?;
    if checker
        .check(text, NO_VARIABLES)
        .context("filecheck failed")?
    {
        Ok(())
    } else {
        // Filecheck mismatch. Emit an explanation as output.
        let (_, explain) = checker
            .explain(text, NO_VARIABLES)
            .context("filecheck explain failed")?;
        anyhow::bail!(
            "filecheck failed for function on line {}:\n{}{}",
            context.details.location.line_number,
            checker,
            explain
        );
    }
}

/// Build a filechecker using the directives in the file preamble and the function's comments.
pub fn build_filechecker(context: &Context) -> anyhow::Result<Checker> {
    let mut builder = CheckerBuilder::new();
    // Preamble comments apply to all functions.
    for comment in context.preamble_comments {
        builder
            .directive(comment.text)
            .context("filecheck directive failed")?;
    }
    for comment in &context.details.comments {
        builder
            .directive(comment.text)
            .context("filecheck directive failed")?;
    }
    Ok(builder.finish())
}

pub fn check_precise_output(actual: &[&str], context: &Context) -> Result<()> {
    // Use the comments after the function to build the test expectation.
    let expected = context
        .details
        .comments
        .iter()
        .filter(|c| !c.text.starts_with(";;"))
        .map(|c| {
            c.text
                .strip_prefix("; ")
                .or_else(|| c.text.strip_prefix(";"))
                .unwrap_or(c.text)
        })
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
                new_test.push(';');
                if !output.is_empty() {
                    new_test.push(' ');
                    new_test.push_str(output);
                }
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
