//! TODO FITZGEN

use crate::ir;
use std::borrow::Cow;

/// TODO FITZGEN
pub type FactResult<T> = std::result::Result<T, FactError>;

/// TODO FITZGEN
pub struct FactError(Cow<'static, str>);

impl FactError {
    /// TODO FITZGEN
    pub fn new(msg: impl Into<Cow<'static, str>>) -> Self {
        FactError(msg.into())
    }
}

/// TODO FITZGEN
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Fact {
    /// TODO FITZGEN
    ValueMax {
        /// TODO FITZGEN
        bit_width: u8,
        /// TODO FITZGEN
        max: u64,
    },

    /// TODO FITZGEN
    PointsTo {
        /// TODO FITZGEN
        pointer_bit_width: u8,
        /// TODO FITZGEN
        region: MemoryRegion,
    },
    // Sym(SymId),
}

// struct SymId(u32);

/// TODO FITZGEN
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct MemoryRegion {
    // Includes both the actual memory bound as well as any guard
    // pages. Exclusive.
    bound: u64,
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
    pub fn check_add(lhs: Self, rhs: Self, result: Fact) -> FactResult<()> {
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
            ) if bw_lhs == bw_rhs && bw_rhs == bw_result => {
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
                    bit_width: bw_offset,
                    max,
                },
                Fact::PointsTo {
                    pointer_bit_width: bw_ptr,
                    region,
                },
                Fact::PointsTo {
                    pointer_bit_width: bw_ptr_result,
                    region: result_region,
                },
            )
            | (
                Fact::PointsTo {
                    pointer_bit_width: bw_ptr,
                    region,
                },
                Fact::ValueMax {
                    bit_width: bw_offset,
                    max,
                },
                Fact::PointsTo {
                    pointer_bit_width: bw_ptr_result,
                    region: result_region,
                },
            ) if bw_ptr == bw_offset && bw_offset == bw_ptr_result => {
                let computed_region = MemoryRegion {
                    bound: region
                        .bound
                        .checked_sub(max)
                        .ok_or_else(|| FactError::new("pointer offset beyond bound"))?,
                };
                ensure!(
                    result_region.bound <= computed_region.bound,
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
                pointer_bit_width: _,
                region: MemoryRegion { bound },
            } => ensure!(
                u64::from(offset_and_size) <= bound,
                "potentially out of bounds memory access"
            ),
            _ => bail!("invalid address"),
        }

        Ok(())
    }
}
