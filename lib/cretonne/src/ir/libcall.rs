//! Naming well-known routines in the runtime library.

use ir::{types, Opcode, Type};
use std::fmt;
use std::str::FromStr;

/// The name of a runtime library routine.
///
/// Runtime library calls are generated for Cretonne IL instructions that don't have an equivalent
/// ISA instruction or an easy macro expansion. A `LibCall` is used as a well-known name to refer to
/// the runtime library routine. This way, Cretonne doesn't have to know about the naming
/// convention in the embedding VM's runtime library.
///
/// This list is likely to grow over time.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LibCall {
    /// ceil.f32
    CeilF32,
    /// ceil.f64
    CeilF64,
    /// floor.f32
    FloorF32,
    /// floor.f64
    FloorF64,
    /// trunc.f32
    TruncF32,
    /// frunc.f64
    TruncF64,
    /// nearest.f32
    NearestF32,
    /// nearest.f64
    NearestF64,
}

const NAME: [&str; 8] = [
    "CeilF32",
    "CeilF64",
    "FloorF32",
    "FloorF64",
    "TruncF32",
    "TruncF64",
    "NearestF32",
    "NearestF64",
];

impl fmt::Display for LibCall {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(NAME[*self as usize])
    }
}

impl FromStr for LibCall {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CeilF32" => Ok(LibCall::CeilF32),
            "CeilF64" => Ok(LibCall::CeilF64),
            "FloorF32" => Ok(LibCall::FloorF32),
            "FloorF64" => Ok(LibCall::FloorF64),
            "TruncF32" => Ok(LibCall::TruncF32),
            "TruncF64" => Ok(LibCall::TruncF64),
            "NearestF32" => Ok(LibCall::NearestF32),
            "NearestF64" => Ok(LibCall::NearestF64),
            _ => Err(()),
        }
    }
}

impl LibCall {
    /// Get the well-known library call name to use as a replacement for an instruction with the
    /// given opcode and controlling type variable.
    ///
    /// Returns `None` if no well-known library routine name exists for that instruction.
    pub fn for_inst(opcode: Opcode, ctrl_type: Type) -> Option<LibCall> {
        Some(match ctrl_type {
            types::F32 => {
                match opcode {
                    Opcode::Ceil => LibCall::CeilF32,
                    Opcode::Floor => LibCall::FloorF32,
                    Opcode::Trunc => LibCall::TruncF32,
                    Opcode::Nearest => LibCall::NearestF32,
                    _ => return None,
                }
            }
            types::F64 => {
                match opcode {
                    Opcode::Ceil => LibCall::CeilF64,
                    Opcode::Floor => LibCall::FloorF64,
                    Opcode::Trunc => LibCall::TruncF64,
                    Opcode::Nearest => LibCall::NearestF64,
                    _ => return None,
                }
            }
            _ => return None,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::string::ToString;

    #[test]
    fn display() {
        assert_eq!(LibCall::CeilF32.to_string(), "CeilF32");
        assert_eq!(LibCall::NearestF64.to_string(), "NearestF64");
    }

    #[test]
    fn parsing() {
        assert_eq!("FloorF32".parse(), Ok(LibCall::FloorF32));
    }
}
