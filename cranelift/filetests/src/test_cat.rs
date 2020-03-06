//! The `cat` subtest.

use crate::subtest::{self, Context, SubTest, SubtestResult};
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

pub fn subtest(parsed: &TestCommand) -> SubtestResult<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "cat");
    if !parsed.options.is_empty() {
        Err(format!("No options allowed on {}", parsed))
    } else {
        Ok(Box::new(TestCat))
    }
}

impl SubTest for TestCat {
    fn name(&self) -> &'static str {
        "cat"
    }

    fn needs_verifier(&self) -> bool {
        false
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> SubtestResult<()> {
        subtest::run_filecheck(&func.display(context.isa).to_string(), context)
    }
}
