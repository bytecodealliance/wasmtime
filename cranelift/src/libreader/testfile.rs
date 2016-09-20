//! Data structures representing a parsed test file.
//!
//! A test file is a `.cton` file which contains test commands and settings for running a
//! file-based test case.
//!

use cretonne::ir::Function;
use cretonne::ir::entities::AnyEntity;
use testcommand::TestCommand;
use isaspec::IsaSpec;
use sourcemap::SourceMap;
use error::Location;

/// A parsed test case.
///
/// This is the result of parsing a `.cton` file which contains a number of test commands and ISA
/// specs followed by the functions that should be tested.
pub struct TestFile<'a> {
    /// `test foo ...` lines.
    pub commands: Vec<TestCommand<'a>>,
    /// `isa bar ...` lines.
    pub isa_spec: IsaSpec,
    pub functions: Vec<(Function, Details<'a>)>,
}

/// Additional details about a function parsed from a text string.
/// These are useful for detecting test commands embedded in comments etc.
/// The details to not affect the semantics of the function.
#[derive(Debug)]
pub struct Details<'a> {
    pub location: Location,
    pub comments: Vec<Comment<'a>>,
    pub map: SourceMap,
}

/// A comment in a parsed function.
///
/// The comment belongs to the immediately preceeding entity, whether that is an EBB header, and
/// instruction, or one of the preamble declarations.
///
/// Comments appearing inside the function but before the preamble, as well as comments appearing
/// after the function are tagged as `AnyEntity::Function`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Comment<'a> {
    pub entity: AnyEntity,
    pub text: &'a str,
}
