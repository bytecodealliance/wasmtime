//! Run commands.
//!
//! Functions in a `.clif` file can have *run commands* appended that control how a function is
//! invoked and tested within the `test run` context. The general syntax is:
//!
//! - `; run`: this assumes the function has a signature like `() -> b*`.
//! - `; run: %fn(42, 4.2) == false`: this syntax specifies the parameters and return values.

use cranelift_codegen::data_value::{self, DataValue, DisplayDataValues};
use std::fmt::{self, Display, Formatter};

/// A run command appearing in a test file.
///
/// For parsing, see `Parser::parse_run_command`
#[derive(PartialEq, Debug)]
pub enum RunCommand {
    /// Invoke a function and print its result.
    Print(Invocation),
    /// Invoke a function and compare its result to a value sequence.
    Run(Invocation, Comparison, Vec<DataValue>),
}

impl RunCommand {
    /// Run the [RunCommand]:
    ///  - for [RunCommand::Print], print the returned values from invoking the function.
    ///  - for [RunCommand::Run], compare the returned values from the invoked function and
    ///    return an `Err` with a descriptive string if the comparison fails.
    ///
    /// Accepts a function used for invoking the actual execution of the command. This function,
    /// `invoked_fn`, is passed the _function name_ and _function arguments_ of the [Invocation].
    pub fn run<F>(&self, invoke_fn: F) -> Result<(), String>
    where
        F: FnOnce(&str, &[DataValue]) -> Result<Vec<DataValue>, String>,
    {
        match self {
            RunCommand::Print(invoke) => {
                let actual = invoke_fn(&invoke.func, &invoke.args)?;
                println!("{} -> {}", invoke, DisplayDataValues(&actual))
            }
            RunCommand::Run(invoke, compare, expected) => {
                let actual = invoke_fn(&invoke.func, &invoke.args)?;
                let matched = match compare {
                    Comparison::Equals => *expected == actual,
                    Comparison::NotEquals => *expected != actual,
                };
                if !matched {
                    let actual = DisplayDataValues(&actual);
                    return Err(format!("Failed test: {}, actual: {}", self, actual));
                }
            }
        }
        Ok(())
    }
}

impl Display for RunCommand {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            RunCommand::Print(invocation) => write!(f, "print: {}", invocation),
            RunCommand::Run(invocation, comparison, expected) => {
                let expected = DisplayDataValues(expected);
                write!(f, "run: {} {} {}", invocation, comparison, expected)
            }
        }
    }
}

/// Represent a function call; [RunCommand]s invoke a CLIF function using an [Invocation].
#[derive(Debug, PartialEq)]
pub struct Invocation {
    /// The name of the function to call. Note: this field is for mostly included for informational
    /// purposes and may not always be necessary for identifying which function to call.
    pub func: String,
    /// The arguments to be passed to the function when invoked.
    pub args: Vec<DataValue>,
}

impl Invocation {
    pub(crate) fn new(func: &str, args: Vec<DataValue>) -> Self {
        let func = func.to_string();
        Self { func, args }
    }
}

impl Display for Invocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "%{}(", self.func)?;
        data_value::write_data_value_list(f, &self.args)?;
        write!(f, ")")
    }
}

/// A CLIF comparison operation; e.g. `==`.
#[allow(missing_docs)]
#[derive(Debug, PartialEq)]
pub enum Comparison {
    Equals,
    NotEquals,
}

impl Display for Comparison {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Comparison::Equals => write!(f, "=="),
            Comparison::NotEquals => write!(f, "!="),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::parse_run_command;
    use cranelift_codegen::ir::{types, AbiParam, Signature};
    use cranelift_codegen::isa::CallConv;

    #[test]
    fn run_a_command() {
        let mut signature = Signature::new(CallConv::Fast);
        signature.returns.push(AbiParam::new(types::I32));
        let command = parse_run_command(";; run: %return42() == 42 ", &signature)
            .unwrap()
            .unwrap();

        assert!(command.run(|_, _| Ok(vec![DataValue::I32(42)])).is_ok());
        assert!(command.run(|_, _| Ok(vec![DataValue::I32(43)])).is_err());
    }
}
