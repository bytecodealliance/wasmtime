//! The `print-cfg` sub-command.
//!
//! Read a series of Cranelift IR files and print their control flow graphs
//! in graphviz format.

use std::borrow::Cow;

use crate::subtest::{self, Context, SubTest, SubtestResult};
use cranelift_codegen::cfg_printer::CFGPrinter;
use cranelift_codegen::ir::Function;
use cranelift_reader::TestCommand;

/// Object implementing the `test print-cfg` sub-test.
struct TestPrintCfg;

pub fn subtest(parsed: &TestCommand) -> SubtestResult<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "print-cfg");
    if !parsed.options.is_empty() {
        Err(format!("No options allowed on {}", parsed))
    } else {
        Ok(Box::new(TestPrintCfg))
    }
}

impl SubTest for TestPrintCfg {
    fn name(&self) -> &'static str {
        "print-cfg"
    }

    fn needs_verifier(&self) -> bool {
        false
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> SubtestResult<()> {
        subtest::run_filecheck(&CFGPrinter::new(&func).to_string(), context)
    }
}
