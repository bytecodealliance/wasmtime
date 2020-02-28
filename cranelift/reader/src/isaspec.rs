//! Parsed representation of `set` and `isa` commands.
//!
//! A test case file can contain `set` commands that set ISA-independent settings, and it can
//! contain `isa` commands that select an ISA and applies ISA-specific settings.
//!
//! If a test case file contains `isa` commands, the tests will only be run against the specified
//! ISAs. If the file contains no `isa` commands, the tests will be run against all supported ISAs.

use crate::error::{Location, ParseResult};
use crate::testcommand::TestOption;
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::settings::{Configurable, Flags, SetError};

/// The ISA specifications in a `.clif` file.
pub enum IsaSpec {
    /// The parsed file does not contain any `isa` commands, but it may contain `set` commands
    /// which are reflected in the finished `Flags` object.
    None(Flags),

    /// The parsed file does contain `isa` commands.
    /// Each `isa` command is used to configure a `TargetIsa` trait object.
    Some(Vec<Box<dyn TargetIsa>>),
}

impl IsaSpec {
    /// If the `IsaSpec` contains exactly 1 `TargetIsa` we return a reference to it
    pub fn unique_isa(&self) -> Option<&dyn TargetIsa> {
        if let Self::Some(ref isa_vec) = *self {
            if isa_vec.len() == 1 {
                return Some(&*isa_vec[0]);
            }
        }
        None
    }
}

/// Parse an iterator of command line options and apply them to `config`.
pub fn parse_options<'a, I>(
    iter: I,
    config: &mut dyn Configurable,
    loc: Location,
) -> ParseResult<()>
where
    I: Iterator<Item = &'a str>,
{
    for opt in iter.map(TestOption::new) {
        match opt {
            TestOption::Flag(name) => match config.enable(name) {
                Ok(_) => {}
                Err(SetError::BadName(name)) => return err!(loc, "unknown flag '{}'", name),
                Err(_) => return err!(loc, "not a boolean flag: '{}'", opt),
            },
            TestOption::Value(name, value) => match config.set(name, value) {
                Ok(_) => {}
                Err(SetError::BadName(name)) => return err!(loc, "unknown setting '{}'", name),
                Err(SetError::BadType) => return err!(loc, "invalid setting type: '{}'", opt),
                Err(SetError::BadValue(expected)) => {
                    return err!(
                        loc,
                        "invalid setting value for '{}', expected {}",
                        opt,
                        expected
                    );
                }
            },
        }
    }
    Ok(())
}
