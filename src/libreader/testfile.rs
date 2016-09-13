//! Data structures representing a parsed test file.
//!
//! A test file is a `.cton` file which contains test commands and settings for running a
//! file-based test case.
//!

use cretonne::ir::Function;
use cretonne::ir::entities::AnyEntity;
use testcommand::TestCommand;

/// A parsed test case.
///
/// This is the result of parsing a `.cton` file which contains a number of test commands followed
/// by the functions that should be tested.
#[derive(Debug)]
pub struct TestFile<'a> {
    pub commands: Vec<TestCommand<'a>>,
    pub functions: Vec<DetailedFunction<'a>>,
}

/// A function parsed from a text string along with other details that are useful for running
/// tests.
#[derive(Debug)]
pub struct DetailedFunction<'a> {
    pub function: Function,
    pub comments: Vec<Comment<'a>>,
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
