//! The `cat` subtest.

use crate::subtest::{self, Context, SubTest};
use cranelift_codegen::ir::Function;
use cranelift_reader::TestCommand;
use std::borrow::Cow;

/// Object implementing the `test cat` sub-test.
///
/// This command is used for testing the parser and function printer. It simply parses a function
/// and prints it out again.
///
/// The result is verified by filecheck.
struct TestCat;

pub fn subtest(parsed: &TestCommand) -> anyhow::Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "cat");
    if !parsed.options.is_empty() {
        anyhow::bail!("No options allowed on {}", parsed);
    }
    Ok(Box::new(TestCat))
}

impl SubTest for TestCat {
    fn name(&self) -> &'static str {
        "cat"
    }

    fn needs_verifier(&self) -> bool {
        false
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> anyhow::Result<()> {
        subtest::run_filecheck(&func.display().to_string(), context)
    }
}
