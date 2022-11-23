//! Test command for testing the optimization phases.
//!
//! The `optimize` test command runs each function through the
//! optimization passes, but not lowering or regalloc. The output for
//! filecheck purposes is the resulting CLIF.
//!
//! Some legalization may be ISA-specific, so this requires an ISA
//! (for now).

use crate::subtest::{run_filecheck, Context, SubTest};
use anyhow::Result;
use cranelift_codegen::ir;
use cranelift_reader::TestCommand;
use std::borrow::Cow;

struct TestOptimize;

pub fn subtest(parsed: &TestCommand) -> Result<Box<dyn SubTest>> {
    assert_eq!(parsed.command, "optimize");
    Ok(Box::new(TestOptimize))
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
        run_filecheck(&clif, context)
    }
}
