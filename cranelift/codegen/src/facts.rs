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
        max: u64,
    },

    /// TODO FITZGEN
    PointsTo {
        /// TODO FITZGEN
        region: MemoryRegion,
    },
    // Sym(SymId),
}

// struct SymId(u32);

/// TODO FITZGEN
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct MemoryRegion {
    /// Includes both the actual memory bound as well as any guard
    /// pages. Exclusive.
    pub bound: u64,
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
    //
    // TODO: add bitwidth of actual operation
    pub fn check_add(lhs: Self, rhs: Self, result: Fact) -> FactResult<()> {
        match (lhs, rhs, result) {
            (
                Fact::ValueMax { max: lhs },
                Fact::ValueMax { max: rhs },
                Fact::ValueMax { max: result },
            ) => {
                let computed_max = lhs
                    .checked_add(rhs)
                    .ok_or_else(|| FactError::new("value max overflow"))?;
                ensure!(
                    result <= computed_max,
                    "claimed max must fit within computed max"
                );
            }

            (
                Fact::ValueMax { max },
                Fact::PointsTo { region },
                Fact::PointsTo {
                    region: result_region,
                },
            )
            | (
                Fact::PointsTo { region },
                Fact::ValueMax { max },
                Fact::PointsTo {
                    region: result_region,
                },
            ) => {
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
