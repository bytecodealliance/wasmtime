//! Run commands.
//!
//! Functions in a `.clif` file can have *run commands* appended that control how a function is
//! invoked and tested within the `test run` context. The general syntax is:
//!
//! - `; run`: this assumes the function has a signature like `() -> b*`.
//! - `; run: %fn(42, 4.2) == false`: this syntax specifies the parameters and return values.

use cranelift_codegen::ir::immediates::{Ieee32, Ieee64};
use cranelift_codegen::ir::{self, ConstantData, Type};
use std::fmt::{self, Display, Formatter};

/// A run command appearing in a test file.
///
/// For parsing, see
/// [Parser::parse_run_command](crate::parser::Parser::parse_run_command).
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
    pub fn run<F>(&self, invoke_fn: F) -> Result<(), String>
    where
        F: FnOnce(&[DataValue]) -> Vec<DataValue>,
    {
        match self {
            RunCommand::Print(invoke) => {
                let actual = invoke_fn(&invoke.args);
                println!("{} -> {}", invoke, DisplayDataValues(&actual))
            }
            RunCommand::Run(invoke, compare, expected) => {
                let actual = invoke_fn(&invoke.args);
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
        write_data_value_list(f, &self.args)?;
        write!(f, ")")
    }
}

/// Represent a data value. Where [Value] is an SSA reference, [DataValue] is the type + value
/// that would be referred to by a [Value].
///
/// [Value]: cranelift_codegen::ir::Value
#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq)]
pub enum DataValue {
    B(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    V128([u8; 16]),
}

impl DataValue {
    /// Return the Cranelift IR [Type] for this [DataValue].
    pub fn ty(&self) -> Type {
        match self {
            DataValue::B(_) => ir::types::B8,
            DataValue::I8(_) => ir::types::I8,
            DataValue::I16(_) => ir::types::I16,
            DataValue::I32(_) => ir::types::I32,
            DataValue::I64(_) => ir::types::I64,
            DataValue::F32(_) => ir::types::F32,
            DataValue::F64(_) => ir::types::F64,
            DataValue::V128(_) => ir::types::I8X16,
        }
    }

    /// Return true if the value is a vector (i.e. `DataValue::V128`).
    pub fn is_vector(&self) -> bool {
        match self {
            DataValue::V128(_) => true,
            _ => false,
        }
    }
}

/// Helper for creating [From] implementations for [DataValue]
macro_rules! from_data {
    ( $ty:ty, $variant:ident ) => {
        impl From<$ty> for DataValue {
            fn from(data: $ty) -> Self {
                DataValue::$variant(data)
            }
        }
    };
}
from_data!(bool, B);
from_data!(i8, I8);
from_data!(i16, I16);
from_data!(i32, I32);
from_data!(i64, I64);
from_data!(f32, F32);
from_data!(f64, F64);
from_data!([u8; 16], V128);

impl Display for DataValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            DataValue::B(dv) => write!(f, "{}", dv),
            DataValue::I8(dv) => write!(f, "{}", dv),
            DataValue::I16(dv) => write!(f, "{}", dv),
            DataValue::I32(dv) => write!(f, "{}", dv),
            DataValue::I64(dv) => write!(f, "{}", dv),
            // Use the Ieee* wrappers here to maintain a consistent syntax.
            DataValue::F32(dv) => write!(f, "{}", Ieee32::from(*dv)),
            DataValue::F64(dv) => write!(f, "{}", Ieee64::from(*dv)),
            // Again, for syntax consistency, use ConstantData, which in this case displays as hex.
            DataValue::V128(dv) => write!(f, "{}", ConstantData::from(&dv[..])),
        }
    }
}

/// Helper structure for printing bracket-enclosed vectors of [DataValue]s.
/// - for empty vectors, display `[]`
/// - for single item vectors, display `42`, e.g.
/// - for multiple item vectors, display `[42, 43, 44]`, e.g.
struct DisplayDataValues<'a>(&'a [DataValue]);

impl<'a> Display for DisplayDataValues<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.0.len() == 1 {
            write!(f, "{}", self.0[0])
        } else {
            write!(f, "[")?;
            write_data_value_list(f, &self.0)?;
            write!(f, "]")
        }
    }
}

/// Helper function for displaying `Vec<DataValue>`.
fn write_data_value_list(f: &mut Formatter<'_>, list: &[DataValue]) -> fmt::Result {
    match list.len() {
        0 => Ok(()),
        1 => write!(f, "{}", list[0]),
        _ => {
            write!(f, "{}", list[0])?;
            for dv in list.iter().skip(1) {
                write!(f, ", {}", dv)?;
            }
            Ok(())
        }
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

        assert!(command.run(|_| vec![DataValue::I32(42)]).is_ok());
        assert!(command.run(|_| vec![DataValue::I32(43)]).is_err());
    }
}
