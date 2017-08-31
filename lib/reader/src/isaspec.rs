//! Parsed representation of `set` and `isa` commands.
//!
//! A test case file can contain `set` commands that set ISA-independent settings, and it can
//! contain `isa` commands that select an ISA and applies ISA-specific settings.
//!
//! If a test case file contains `isa` commands, the tests will only be run against the specified
//! ISAs. If the file contains no `isa` commands, the tests will be run against all supported ISAs.

use cretonne::settings::{Flags, Configurable, Error as SetError};
use cretonne::isa::TargetIsa;
use error::{Result, Location};
use testcommand::TestOption;

/// The ISA specifications in a `.cton` file.
pub enum IsaSpec {
    /// The parsed file does not contain any `isa` commands, but it may contain `set` commands
    /// which are reflected in the finished `Flags` object.
    None(Flags),

    /// The parsed file does contains `isa` commands.
    /// Each `isa` command is used to configure a `TargetIsa` trait object.
    Some(Vec<Box<TargetIsa>>),
}

impl IsaSpec {
    /// If the `IsaSpec` contains exactly 1 `TargetIsa` we return a reference to it
    pub fn unique_isa(&self) -> Option<&TargetIsa> {
        if let IsaSpec::Some(ref isa_vec) = *self {
            if isa_vec.len() == 1 {
                return Some(&*isa_vec[0]);
            }
        }
        None
    }
}

/// Parse an iterator of command line options and apply them to `config`.
pub fn parse_options<'a, I>(iter: I, config: &mut Configurable, loc: &Location) -> Result<()>
where
    I: Iterator<Item = &'a str>,
{
    for opt in iter.map(TestOption::new) {
        match opt {
            TestOption::Flag(name) => {
                match config.enable(name) {
                    Ok(_) => {}
                    Err(SetError::BadName) => return err!(loc, "unknown flag '{}'", opt),
                    Err(_) => return err!(loc, "not a boolean flag: '{}'", opt),
                }
            }
            TestOption::Value(name, value) => {
                match config.set(name, value) {
                    Ok(_) => {}
                    Err(SetError::BadName) => return err!(loc, "unknown setting '{}'", opt),
                    Err(SetError::BadType) => return err!(loc, "invalid setting type: '{}'", opt),
                    Err(SetError::BadValue) => return err!(loc, "invalid setting value: '{}'", opt),
                }
            }
        }
    }
    Ok(())
}
