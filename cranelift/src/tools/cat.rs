//! The `cat` sub-command.
//!
//! Read a sequence of Cretonne IL files and print them again to stdout. This has the effect of
//! normalizing formatting and removing comments.

use std::borrow::Cow;
use cretonne::ir::Function;
use cton_reader::{parse_functions, TestCommand};
use CommandResult;
use utils::read_to_string;
use filetest::subtest::{self, SubTest, Context, Result as STResult};

pub fn run(files: Vec<String>) -> CommandResult {
    for (i, f) in files.into_iter().enumerate() {
        if i != 0 {
            println!("");
        }
        try!(cat_one(f))
    }
    Ok(())
}

fn cat_one(filename: String) -> CommandResult {
    let buffer = try!(read_to_string(&filename).map_err(|e| format!("{}: {}", filename, e)));
    let items = try!(parse_functions(&buffer).map_err(|e| format!("{}: {}", filename, e)));

    for (idx, func) in items.into_iter().enumerate() {
        if idx != 0 {
            println!("");
        }
        print!("{}", func);
    }

    Ok(())
}

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
        subtest::run_filecheck(&func.to_string(), context)
    }
}
