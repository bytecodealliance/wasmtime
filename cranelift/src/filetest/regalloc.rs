//! Test command for testing the register allocator.
//!
//! The `regalloc` test command runs each function through the register allocator after ensuring
//! that all instructions are legal for the target.
//!
//! The resulting function is sent to `filecheck`.

use std::borrow::Cow;
use cretonne::{self, write_function};
use cretonne::ir::Function;
use cton_reader::TestCommand;
use filetest::subtest::{SubTest, Context, Result, run_filecheck};

struct TestRegalloc;

pub fn subtest(parsed: &TestCommand) -> Result<Box<SubTest>> {
    assert_eq!(parsed.command, "regalloc");
    if !parsed.options.is_empty() {
        Err(format!("No options allowed on {}", parsed))
    } else {
        Ok(Box::new(TestRegalloc))
    }
}

impl SubTest for TestRegalloc {
    fn name(&self) -> Cow<str> {
        Cow::from("regalloc")
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn needs_isa(&self) -> bool {
        true
    }

    fn run(&self, func: Cow<Function>, context: &Context) -> Result<()> {
        let isa = context.isa.expect("register allocator needs an ISA");

        // Create a compilation context, and drop in the function.
        let mut comp_ctx = cretonne::Context::new();
        comp_ctx.func = func.into_owned();

        comp_ctx.flowgraph();
        // TODO: Should we have an option to skip legalization?
        comp_ctx.legalize(isa);
        comp_ctx.regalloc(isa);

        let mut text = String::new();
        write_function(&mut text, &comp_ctx.func, Some(isa)).map_err(|e| e.to_string())?;
        run_filecheck(&text, context)
    }
}
