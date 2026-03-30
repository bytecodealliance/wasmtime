//! Aarch64 addressing mode.

use super::regs;
use crate::Result;
use crate::{
    masm::{IntScratch, MacroAssembler as Masm, OperandSize, RegImm},
    reg::Reg,
};
use cranelift_codegen::ir::{Type, types};
use cranelift_codegen::isa::aarch64::inst::{
    AMode, ExtendOp, PairAMode, SImm7Scaled, SImm9, UImm12Scaled,
};

/// Aarch64 indexing mode.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum Indexing {
    /// Pre-indexed.
    Pre,
    /// Post-indexed.
    Post,
}

/// Memory address representation.
#[derive(Debug, Copy, Clone)]
pub(crate) enum Address {
    /// Base register with an arbitrary offset.
    Offset {
        /// Base register.
        base: Reg,
        /// Offset.
        offset: i64,
    },
    /// SP-indexed addressing mode for single register loads/stores.
    SPIndexedSingle {
        /// 9-bit signed offset.
        offset: SImm9,
        /// Indexing mode (pre or post).
        indexing: Indexing,
    },
    /// SP-indexed addressing mode for register pair loads/stores.
    SPIndexedPair {
        /// 7-bit signed scaled offset.
        offset: SImm7Scaled,
        /// Indexing mode (pre or post).
        indexing: Indexing,
    },
}

impl Address {
    /// Create a pre-indexed addressing mode from the stack pointer for single register operations.
    pub fn pre_indexed_from_sp(offset: SImm9) -> Self {
        Self::SPIndexedSingle {
            offset,
            indexing: Indexing::Pre,
        }
    }

    /// Create a post-indexed addressing mode from the stack pointer for single register operations.
    pub fn post_indexed_from_sp(offset: SImm9) -> Self {
        Self::SPIndexedSingle {
            offset,
            indexing: Indexing::Post,
        }
    }

    /// Create a pre-indexed addressing mode from the stack pointer for register pair operations.
    pub fn pre_indexed_from_sp_for_pair(offset: SImm7Scaled) -> Self {
        Self::SPIndexedPair {
            offset,
            indexing: Indexing::Pre,
        }
    }

    /// Create a post-indexed addressing mode from the stack pointer for register pair operations.
    pub fn post_indexed_from_sp_for_pair(offset: SImm7Scaled) -> Self {
        Self::SPIndexedPair {
            offset,
            indexing: Indexing::Post,
        }
    }

    /// Create an offset addressing mode with the shadow stack pointer register as a base.
    pub fn from_shadow_sp(offset: i64) -> Self {
        Self::Offset {
            base: regs::shadow_sp(),
            offset,
        }
    }

    /// Create register and arbitrary offset addressing mode.
    pub fn offset(base: Reg, offset: i64) -> Self {
        // This exists to enforce the sp vs shadow_sp invariant, the
        // sp generally should not be used as a base register in an
        // address. In the cases where its usage is required and where
        // we are sure that it's 16-byte aligned, the address should
        // be constructed via the SP-indexed constructors.
        // For more details around the stack pointer and shadow stack
        // pointer see the docs at regs::shadow_sp().
        assert!(
            base != regs::sp(),
            "stack pointer not allowed in arbitrary offset addressing mode"
        );
        Self::Offset { base, offset }
    }

    /// Converts self to cranelift's [`PairAMode`].
    /// # Panics
    /// This function panics if self cannot be converted to [`PairAMode`].
    /// NB: that all uses of this function currently guarantee that
    /// the offset will fit in a 7-bit signed offset.
    pub fn to_pair_addressing_mode(self) -> PairAMode {
        match self {
            Self::SPIndexedPair { offset, indexing } => {
                if indexing == Indexing::Pre {
                    PairAMode::SPPreIndexed { simm7: offset }
                } else {
                    PairAMode::SPPostIndexed { simm7: offset }
                }
            }
            _ => panic!("Could not convert addressing mode to PairAMode"),
        }
    }

    /// Converts self to cranelift's [`AMode`].
    /// The closure parameter ensures that the caller scope is kept in
    /// sync with the scratch register used for materializing the
    /// general register and offset addressing mode.
    /// # Panics
    /// This function panics if self cannot be converted to [`AMode`].
    pub fn to_addressing_mode<M: Masm>(
        self,
        masm: &mut M,
        size: OperandSize,
        f: impl FnOnce(&mut M, AMode) -> Result<()>,
    ) -> Result<()> {
        use Address::*;
        use Indexing::*;

        match self {
            SPIndexedSingle { offset, indexing } => {
                let amode = if indexing == Pre {
                    AMode::SPPreIndexed { simm9: offset }
                } else {
                    AMode::SPPostIndexed { simm9: offset }
                };

                f(masm, amode)
            }
            Offset { base, offset } => {
                if let Some(simm9) = SImm9::maybe_from_i64(offset) {
                    f(
                        masm,
                        AMode::Unscaled {
                            rn: base.into(),
                            simm9,
                        },
                    )
                } else if let Some(uimm12) =
                    UImm12Scaled::maybe_from_i64(offset, map_to_scale_type(size))
                {
                    f(
                        masm,
                        AMode::UnsignedOffset {
                            rn: base.into(),
                            uimm12,
                        },
                    )
                } else {
                    masm.with_scratch::<IntScratch, _>(|masm, temp| {
                        masm.mov(temp.writable(), RegImm::i64(offset), OperandSize::S64)?;
                        f(
                            masm,
                            AMode::RegExtended {
                                rn: base.into(),
                                rm: temp.inner().into(),
                                extendop: ExtendOp::SXTX,
                            },
                        )
                    })
                }
            }
            _ => panic!("Could not convert addressing mode to AMode"),
        }
    }

    /// Returns the register base and immediate offset of the given [`Address`].
    ///
    /// # Panics
    /// This function panics if the [`Address`] is not [`Address::Offset`].
    pub fn unwrap_offset(&self) -> (Reg, i64) {
        match self {
            Self::Offset { base, offset } => (*base, *offset),
            _ => panic!("Expected register and offset addressing mode"),
        }
    }
}

fn map_to_scale_type(size: OperandSize) -> Type {
    match size {
        OperandSize::S8 => types::I8,
        OperandSize::S16 => types::I16,
        OperandSize::S32 => types::I32,
        OperandSize::S64 => types::I64,
        OperandSize::S128 => types::I8X16,
    }
}
