//! SubTest trait.

use std::result;
use std::borrow::Cow;
use cretonne::ir::Function;
use cton_reader::{TestCommand, Details};
use filecheck::{self, CheckerBuilder, Checker, Value as FCValue};

pub type Result<T> = result::Result<T, String>;

/// Create a new subcommand trait object to match `parsed.command`.
pub fn new(parsed: &TestCommand) -> Result<Box<SubTest>> {
    use cat;
    use print_cfg;
    use filetest::domtree;
    match parsed.command {
        "cat" => cat::subtest(parsed),
        "print-cfg" => print_cfg::subtest(parsed),
        "domtree" => domtree::subtest(parsed),
        _ => Err(format!("unknown test command '{}'", parsed.command)),
    }
}

/// Context for running a a test on a single function.
pub struct Context<'a> {
    /// Additional details about the function from the parser.
    pub details: Details<'a>,

    /// Was the function verified before running this test?
    pub verified: bool,
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
        self.details.map.lookup_str(varname).map(|e| FCValue::Regex(format!(r"\b{}\b", e).into()))
    }
}

/// Run filecheck on `text`, using directives extracted from `context`.
pub fn run_filecheck(text: &str, context: &Context) -> Result<()> {
    let checker = try!(build_filechecker(&context.details));
    if try!(checker.check(&text, context).map_err(|e| format!("filecheck: {}", e))) {
        Ok(())
    } else {
        // Filecheck mismatch. Emit an explanation as output.
        let (_, explain) = try!(checker.explain(&text, context)
            .map_err(|e| format!("explain: {}", e)));
        Err(format!("filecheck failed:\n{}{}", checker, explain))
    }
}

/// Build a filechecker using the directives in the function's comments.
pub fn build_filechecker(details: &Details) -> Result<Checker> {
    let mut builder = CheckerBuilder::new();
    for comment in &details.comments {
        try!(builder.directive(comment.text).map_err(|e| format!("filecheck: {}", e)));
    }
    let checker = builder.finish();
    if checker.is_empty() {
        Err("no filecheck directives in function".to_string())
    } else {
        Ok(checker)
    }
}
