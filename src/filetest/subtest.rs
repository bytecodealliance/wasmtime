//! SubTest trait.

use std::result;
use std::borrow::Cow;
use cretonne::ir::Function;
use cretonne::isa::TargetIsa;
use cretonne::settings::{Flags, FlagsOrIsa};
use cton_reader::{Details, Comment};
use filecheck::{self, CheckerBuilder, Checker, Value as FCValue};

pub type Result<T> = result::Result<T, String>;

/// Context for running a test on a single function.
pub struct Context<'a> {
    /// Comments from the preamble f the test file. These apply to all functions.
    pub preamble_comments: &'a [Comment<'a>],

    /// Additional details about the function from the parser.
    pub details: Details<'a>,

    /// Was the function verified before running this test?
    pub verified: bool,

    /// ISA-independent flags for this test.
    pub flags: &'a Flags,

    /// Target ISA to test against. Only guaranteed to be present for sub-tests whose `needs_isa`
    /// method returned `true`. For other sub-tests, this is set if the test file has a unique ISA.
    pub isa: Option<&'a TargetIsa>,
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
/// Each `.cton` test file may contain multiple test commands, each represented by a `SubTest`
/// trait object.
pub trait SubTest {
    /// Name identifying this subtest. Typically the same as the test command.
    fn name(&self) -> Cow<str>;

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

    /// Run this test on `func`.
    fn run(&self, func: Cow<Function>, context: &Context) -> Result<()>;
}

/// Make the parser's source map available as filecheck variables.
///
/// This means that the filecheck directives can refer to entities like `jump $ebb3`, where `$ebb3`
/// will expand to the EBB number that was assigned to `ebb3` in the input source.
///
/// The expanded entity names are wrapped in word boundary regex guards so that 'inst1' doesn't
/// match 'inst10'.
impl<'a> filecheck::VariableMap for Context<'a> {
    fn lookup(&self, varname: &str) -> Option<FCValue> {
        self.details.map.lookup_str(varname).map(|e| {
            FCValue::Regex(format!(r"\b{}\b", e).into())
        })
    }
}

/// Run filecheck on `text`, using directives extracted from `context`.
pub fn run_filecheck(text: &str, context: &Context) -> Result<()> {
    let checker = build_filechecker(context)?;
    if checker.check(&text, context).map_err(
        |e| format!("filecheck: {}", e),
    )?
    {
        Ok(())
    } else {
        // Filecheck mismatch. Emit an explanation as output.
        let (_, explain) = checker.explain(&text, context).map_err(
            |e| format!("explain: {}", e),
        )?;
        Err(format!("filecheck failed:\n{}{}", checker, explain))
    }
}

/// Build a filechecker using the directives in the file preamble and the function's comments.
pub fn build_filechecker(context: &Context) -> Result<Checker> {
    let mut builder = CheckerBuilder::new();
    // Preamble comments apply to all functions.
    for comment in context.preamble_comments {
        builder.directive(comment.text).map_err(|e| {
            format!("filecheck: {}", e)
        })?;
    }
    for comment in &context.details.comments {
        builder.directive(comment.text).map_err(|e| {
            format!("filecheck: {}", e)
        })?;
    }
    let checker = builder.finish();
    if checker.is_empty() {
        Err("no filecheck directives in function".to_string())
    } else {
        Ok(checker)
    }
}
