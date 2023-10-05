//! Proof-carrying code. We attach "facts" to values and then check
//! that they remain true after compilation.
//!
//! TODO fill this out more
//!
//! TODO rename this to pcc.rs; also maybe put in `ir` module?
//!
//! TODO: add another mode that checks facts at CLIF level, before
//! lowering. Use this for fuzzing?

use crate::ir;
use crate::isa::TargetIsa;
use crate::machinst::{InsnIndex, LowerBackend, MachInst, VCode};
use crate::CodegenResult;
use regalloc2::{Function, OperandKind};
use std::borrow::Cow;

/// The result of fact-checking.
pub type FactResult<T> = std::result::Result<T, FactError>;

/// A fact-checking error.
/// TODO: make this an enum
#[derive(Debug, Clone)]
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

    // TODO: "subsumes" method

    // TODO: change sigs below to take &Self instead

    /// TODO DOCS
    pub fn add(lhs: Self, rhs: Self, add_width: u8, pointer_width: u8) -> Option<Self> {
        match (lhs, rhs) {
            (
                Fact::ValueMax {
                    bit_width: bw_lhs,
                    max: lhs,
                },
                Fact::ValueMax {
                    bit_width: bw_rhs,
                    max: rhs,
                },
            ) if bw_lhs == bw_rhs && add_width >= bw_lhs => {
                let computed_max = lhs.checked_add(rhs)?;
                Some(Fact::ValueMax {
                    bit_width: bw_lhs,
                    max: computed_max,
                })
            }

            (
                Fact::ValueMax {
                    bit_width: bw_max,
                    max,
                },
                Fact::PointsTo { region },
            )
            | (
                Fact::PointsTo { region },
                Fact::ValueMax {
                    bit_width: bw_max,
                    max,
                },
            ) if bw_max >= pointer_width && add_width >= bw_max => {
                let computed_region = MemoryRegion {
                    max: region.max.checked_sub(max)?,
                };
                Some(Fact::PointsTo {
                    region: computed_region,
                })
            }

            _ => None,
        }
    }

    /// TODO DOCS
    pub fn uextend(lhs: Self, from_width: u8, to_width: u8) -> Option<Self> {
        match lhs {
            // If we have a defined value in bits 0..bit_width, and we
            // are filling zeroes into from_bits..to_bits, and
            // bit_width and from_bits are exactly contiguous, then we
            // have defined values in 0..to_bits (and because this is
            // a zero-extend, the max value is the same).
            Fact::ValueMax { bit_width, max } if bit_width == from_width => Some(Fact::ValueMax {
                bit_width: to_width,
                max,
            }),
            _ => None,
        }
    }

    /// TODO DOCS
    pub fn sextend(lhs: Self, from_width: u8, to_width: u8) -> Option<Self> {
        unimplemented!("HELP I AM NOT IMPLEMENTED")
    }

    /// TODO FITZGEN
    pub fn check_add(
        lhs: Self,
        rhs: Self,
        result: Fact,
        add_width: u8,
        pointer_width: u8,
    ) -> FactResult<()> {
        panic!("rewrite me in terms of Self::add");
        Ok(())
    }

    // TODO: use `u16` or more for bits (consider e.g. AVX-512 and flexible SIMD)?
    /// Check a `uextend`.
    pub fn check_uextend(value: Fact, result: Fact, from_bits: u8, to_bits: u8) -> FactResult<()> {
        // TODO

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
) -> FactResult<()> {
    for inst in 0..vcode.num_insts() {
        let inst = InsnIndex::new(inst);
        if vcode.inst_defines_facts(inst) || vcode[inst].is_mem_access() {
            // This instruction defines a register with a new fact, or
            // has some side-effect we want to be careful to
            // verify. We'll call into the backend to validate this
            // fact with respect to the instruction and the input
            // facts.
            //
            // TODO: check insts that can trap as well?
            backend.check_fact(&vcode[inst], vcode)?;
        }
    }
    Ok(())
}
