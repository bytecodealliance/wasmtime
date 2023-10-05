//! Proof-carrying code. We attach "facts" to values and then check
//! that they remain true after compilation.
//!
//! TODO fill this out more
//!
//! TODO rename this to pcc.rs

use crate::ir;
use crate::isa::TargetIsa;
use crate::machinst::{InsnIndex, LowerBackend, VCode};
use crate::CodegenResult;
use regalloc2::{Function, OperandKind};
use std::borrow::Cow;

/// The result of fact-checking.
pub type FactResult<T> = std::result::Result<T, FactError>;

/// A fact-checking error.
pub struct FactError(Cow<'static, str>);

impl FactError {
    /// Create a new fact error.
    pub fn new(msg: impl Into<Cow<'static, str>>) -> Self {
        FactError(msg.into())
    }
}

/// A fact on a value.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Fact {
    /// A bitslice of a value (up to a bitwidth) is less than or equal
    /// to a given maximum value.
    ///
    /// The slicing behavior is needed because this fact can describe
    /// both an SSA `Value`, whose entire value is well-defined, and a
    /// `VReg` in VCode, whose bits beyond the type stored in that
    /// register are don't-care (undefined).
    ValueMax {
        /// The bitwidth of bits we care about, from the LSB upward.
        bit_width: u8,
        /// The maximum value that the bitslice can take (inclusive).
        max: u64,
    },

    /// A pointer value to a memory region that can be accessed.
    PointsTo {
        /// A description of the memory region this pointer is allowed
        /// to access (size, etc).
        region: MemoryRegion,
    },
    // Sym(SymId),
}

// struct SymId(u32);

/// A memory region that can be accessed. This description is attached
/// to a particular base pointer.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct MemoryRegion {
    /// Includes both the actual memory bound as well as any guard
    /// pages. Inclusive, so we can represent the full range of a
    /// `u64`.
    pub max: u64,
}

// enum MemoryType {
//     Struct(StructType),
//     Array {
//         element: Box<MemoryType>,
//         stride: u64,
//         bound: u64,
//         guard: u64,
//     },
// }

macro_rules! ensure {
    ( $condition:expr, $msg:expr $(,)? ) => {
        if !$condition {
            return Err(FactError::new($msg));
        }
    };
}

macro_rules! bail {
    ( $msg:expr ) => {{
        return Err(FactError::new($msg));
    }};
}

impl Fact {
    /// TODO FITZGEN
    pub fn infer(dfg: &ir::DataFlowGraph, value: ir::Value) -> Option<Fact> {
        todo!()
    }

    /// TODO FITZGEN
    pub fn check_add(
        lhs: Self,
        rhs: Self,
        result: Fact,
        add_width: u8,
        pointer_width: u8,
    ) -> FactResult<()> {
        match (lhs, rhs, result) {
            (
                Fact::ValueMax {
                    bit_width: bw_lhs,
                    max: lhs,
                },
                Fact::ValueMax {
                    bit_width: bw_rhs,
                    max: rhs,
                },
                Fact::ValueMax {
                    bit_width: bw_result,
                    max: result,
                },
            ) if bw_lhs == bw_rhs && bw_lhs == bw_result && add_width >= bw_lhs => {
                let computed_max = lhs
                    .checked_add(rhs)
                    .ok_or_else(|| FactError::new("value max overflow"))?;
                ensure!(
                    result <= computed_max,
                    "claimed max must fit within computed max"
                );
            }

            (
                Fact::ValueMax {
                    bit_width: bw_max,
                    max,
                },
                Fact::PointsTo { region },
                Fact::PointsTo {
                    region: result_region,
                },
            )
            | (
                Fact::PointsTo { region },
                Fact::ValueMax {
                    bit_width: bw_max,
                    max,
                },
                Fact::PointsTo {
                    region: result_region,
                },
            ) if bw_max >= pointer_width && add_width >= bw_max => {
                let computed_region = MemoryRegion {
                    max: region.max.checked_sub(max).ok_or_else(|| {
                        FactError::new("pointer offset beyond memory-region max offset")
                    })?,
                };
                ensure!(
                    result_region.max <= computed_region.max,
                    "claimed memory region must fit within computed memory region"
                );
            }

            _ => bail!("invalid add"),
        }

        Ok(())
    }

    /// TODO FITZGEN
    pub fn check_address(offset: u32, size: u32, address: Fact) -> FactResult<()> {
        let offset_and_size = offset
            .checked_add(size)
            .ok_or_else(|| FactError::new("offset and size overflow"))?;

        match address {
            Fact::PointsTo {
                region: MemoryRegion { max },
            } => ensure!(
                u64::from(offset_and_size) <= max,
                "potentially out of bounds memory access"
            ),
            _ => bail!("invalid address"),
        }

        Ok(())
    }
}

/// Top-level entry point after compilation: this checks the facts in
/// VCode.
pub fn check_facts<B: LowerBackend + TargetIsa>(
    f: &ir::Function,
    vcode: &VCode<B::MInst>,
    backend: &B,
) -> CodegenResult<()> {
    for inst in 0..vcode.num_insts() {
        let inst = InsnIndex::new(inst);
        if vcode.inst_defines_facts(inst) {
            // This instruction defines a register with a new
            // fact. We'll call into the backend to validate this fact
            // with respect to the instruction and the input facts.
            backend.check_fact(&vcode[inst], vcode)?;
        }
    }
    Ok(())
}
