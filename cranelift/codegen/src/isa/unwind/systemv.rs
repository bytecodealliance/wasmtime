//! System V ABI unwind information.

use alloc::vec::Vec;
use gimli::write::{Address, FrameDescriptionEntry};
use thiserror::Error;

#[cfg(feature = "enable-serde")]
use serde::{Deserialize, Serialize};

type Register = u16;
type Expression = Vec<u8>;

/// Enumerate the errors possible in mapping Cranelift registers to their DWARF equivalent.
#[allow(missing_docs)]
#[derive(Error, Debug, PartialEq, Eq)]
pub enum RegisterMappingError {
    #[error("unable to find bank for register info")]
    MissingBank,
    #[error("register mapping is currently only implemented for x86_64")]
    UnsupportedArchitecture,
    #[error("unsupported register bank: {0}")]
    UnsupportedRegisterBank(&'static str),
}

fn into_raw_expr(expr: gimli::write::Expression) -> Expression {
    use gimli::read::UnwindSection;
    use gimli::{read, write, Encoding, Format, Register, RunTimeEndian};

    pub struct Writer {
        writer: write::EndianVec<RunTimeEndian>,
    }
    impl write::Writer for Writer {
        type Endian = RunTimeEndian;
        fn endian(&self) -> Self::Endian {
            self.writer.endian()
        }
        fn len(&self) -> usize {
            self.writer.len()
        }
        fn write(&mut self, bytes: &[u8]) -> write::Result<()> {
            self.writer.write(bytes)
        }
        fn write_at(&mut self, offset: usize, bytes: &[u8]) -> write::Result<()> {
            self.writer.write_at(offset, bytes)
        }
    }

    let mut table = write::FrameTable::default();

    let cie = write::CommonInformationEntry::new(
        Encoding {
            address_size: 8,
            format: Format::Dwarf32,
            version: 1,
        },
        1,
        -8,
        Register(7),
    );
    let cie_id = table.add_cie(cie);
    let mut fde = write::FrameDescriptionEntry::new(Address::Constant(0), 1);
    fde.add_instruction(0, write::CallFrameInstruction::CfaExpression(expr));

    table.add_fde(cie_id, fde);

    let mut debug_frame = write::DebugFrame::from(Writer {
        writer: write::EndianVec::new(RunTimeEndian::Little),
    });
    table.write_debug_frame(&mut debug_frame).unwrap();
    let data = debug_frame.writer.take();

    let debug_frame = read::DebugFrame::from(read::EndianSlice::new(&data, RunTimeEndian::Little));
    let base_addrs = read::BaseAddresses::default();
    let mut it = debug_frame.entries(&base_addrs);
    let cie = match it.next().unwrap().unwrap() {
        read::CieOrFde::Cie(cie) => cie.clone(),
        _ => panic!(),
    };
    let fde = match it.next().unwrap().unwrap() {
        read::CieOrFde::Fde(fde) => fde,
        _ => panic!(),
    };
    match fde
        .parse(move |_, _, _| Ok(cie.clone()))
        .unwrap()
        .instructions(&debug_frame, &base_addrs)
        .next()
        .unwrap()
        .unwrap()
    {
        read::CallFrameInstruction::DefCfaExpression { expression } => expression.0.to_vec(),
        _ => panic!(),
    }
}

// This mirrors gimli's CallFrameInstruction, but is serializable
// TODO: if gimli ever adds serialization support, remove this type
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub(crate) enum CallFrameInstruction {
    Cfa(Register, i32),
    CfaRegister(Register),
    CfaOffset(i32),
    CfaExpression(Expression),
    Restore(Register),
    Undefined(Register),
    SameValue(Register),
    Offset(Register, i32),
    ValOffset(Register, i32),
    Register(Register, Register),
    Expression(Register, Expression),
    ValExpression(Register, Expression),
    RememberState,
    RestoreState,
    ArgsSize(u32),
}

impl From<gimli::write::CallFrameInstruction> for CallFrameInstruction {
    fn from(cfi: gimli::write::CallFrameInstruction) -> Self {
        use gimli::write::CallFrameInstruction;

        match cfi {
            CallFrameInstruction::Cfa(reg, offset) => Self::Cfa(reg.0, offset),
            CallFrameInstruction::CfaRegister(reg) => Self::CfaRegister(reg.0),
            CallFrameInstruction::CfaOffset(offset) => Self::CfaOffset(offset),
            CallFrameInstruction::CfaExpression(expr) => Self::CfaExpression(into_raw_expr(expr)),
            CallFrameInstruction::Restore(reg) => Self::Restore(reg.0),
            CallFrameInstruction::Undefined(reg) => Self::Undefined(reg.0),
            CallFrameInstruction::SameValue(reg) => Self::SameValue(reg.0),
            CallFrameInstruction::Offset(reg, offset) => Self::Offset(reg.0, offset),
            CallFrameInstruction::ValOffset(reg, offset) => Self::ValOffset(reg.0, offset),
            CallFrameInstruction::Register(reg1, reg2) => Self::Register(reg1.0, reg2.0),
            CallFrameInstruction::Expression(reg, expr) => {
                Self::Expression(reg.0, into_raw_expr(expr))
            }
            CallFrameInstruction::ValExpression(reg, expr) => {
                Self::ValExpression(reg.0, into_raw_expr(expr))
            }
            CallFrameInstruction::RememberState => Self::RememberState,
            CallFrameInstruction::RestoreState => Self::RestoreState,
            CallFrameInstruction::ArgsSize(size) => Self::ArgsSize(size),
        }
    }
}

impl Into<gimli::write::CallFrameInstruction> for CallFrameInstruction {
    fn into(self) -> gimli::write::CallFrameInstruction {
        use gimli::{
            write::{CallFrameInstruction, Expression},
            Register,
        };

        match self {
            Self::Cfa(reg, offset) => CallFrameInstruction::Cfa(Register(reg), offset),
            Self::CfaRegister(reg) => CallFrameInstruction::CfaRegister(Register(reg)),
            Self::CfaOffset(offset) => CallFrameInstruction::CfaOffset(offset),
            Self::CfaExpression(expr) => CallFrameInstruction::CfaExpression(Expression::raw(expr)),
            Self::Restore(reg) => CallFrameInstruction::Restore(Register(reg)),
            Self::Undefined(reg) => CallFrameInstruction::Undefined(Register(reg)),
            Self::SameValue(reg) => CallFrameInstruction::SameValue(Register(reg)),
            Self::Offset(reg, offset) => CallFrameInstruction::Offset(Register(reg), offset),
            Self::ValOffset(reg, offset) => CallFrameInstruction::ValOffset(Register(reg), offset),
            Self::Register(reg1, reg2) => {
                CallFrameInstruction::Register(Register(reg1), Register(reg2))
            }
            Self::Expression(reg, expr) => {
                CallFrameInstruction::Expression(Register(reg), Expression::raw(expr))
            }
            Self::ValExpression(reg, expr) => {
                CallFrameInstruction::ValExpression(Register(reg), Expression::raw(expr))
            }
            Self::RememberState => CallFrameInstruction::RememberState,
            Self::RestoreState => CallFrameInstruction::RestoreState,
            Self::ArgsSize(size) => CallFrameInstruction::ArgsSize(size),
        }
    }
}

/// Represents unwind information for a single System V ABI function.
///
/// This representation is not ISA specific.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct UnwindInfo {
    instructions: Vec<(u32, CallFrameInstruction)>,
    len: u32,
}

impl UnwindInfo {
    pub(crate) fn new(instructions: Vec<(u32, CallFrameInstruction)>, len: u32) -> Self {
        Self { instructions, len }
    }

    /// Converts the unwind information into a `FrameDescriptionEntry`.
    pub fn to_fde(&self, address: Address) -> gimli::write::FrameDescriptionEntry {
        let mut fde = FrameDescriptionEntry::new(address, self.len);

        for (offset, inst) in &self.instructions {
            fde.add_instruction(*offset, inst.clone().into());
        }

        fde
    }
}
