//! The `cat` subtest.

use std::borrow::Cow;
use cretonne::ir::Function;
use cton_reader::TestCommand;
use subtest::{self, SubTest, Context, Result as STResult};

/// Object implementing the `test cat` sub-test.
///
/// This command is used for testing the parser and function printer. It simply parses a function
/// and prints it out again.
///
/// The result is verified by filecheck.
struct TestCat;

pub fn subtest(parsed: &TestCommand) -> STResult<Box<SubTest>> {
    assert_eq!(parsed.command, "cat");
    if !parsed.options.is_empty() {
        Err(format!("No options allowed on {}", parsed))
    } else {
        Ok(Box::new(TestCat))
    }
}

impl SubTest for TestCat {
    fn name(&self) -> Cow<str> {
        Cow::from("cat")
    }

    fn needs_verifier(&self) -> bool {
        false
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> STResult<()> {
        subtest::run_filecheck(&func.display(context.isa).to_string(), context)
    }
}
