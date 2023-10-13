//! Proof-carrying code. We attach "facts" to values and then check
//! that they remain true after compilation.
//!
//! A few key design principle of this approach are:
//!
//! - The producer of the IR provides the axioms. All "ground truth",
//!   such as what memory is accessible -- is meant to come by way of
//!   facts on the function arguments and global values. In some
//!   sense, all we are doing here is validating the "internal
//!   consistency" of the facts that are provided on values, and the
//!   actions performed on those values.
//!
//! - We do not derive and forward-propagate facts eagerly. Rather,
//!   the producer needs to provide breadcrumbs -- a "proof witness"
//!   of sorts -- to allow the checking to complete. That means that
//!   as an address is computed, or pointer chains are dereferenced,
//!   each intermediate value will likely have some fact attached.
//!
//!   This does create more verbose IR, but a significant positive
//!   benefit is that it avoids unnecessary work: we do not build up a
//!   knowledge base that effectively encodes the integer ranges of
//!   many or most values in the program. Rather, we only check
//!   specifically the memory-access sequences. In practice, each such
//!   sequence is likely to be a carefully-controlled sequence of IR
//!   operations from, e.g., a sandboxing compiler (such as
//!   `cranelift-wasm`) so adding annotations here to communicate
//!   intent (ranges, bounds-checks, and the like) is no problem.
//!
//! Facts are attached to SSA values in CLIF, and are maintained
//! through optimizations and through lowering. They are thus also
//! present on VRegs in the VCode. In theory, facts could be checked
//! at either level, though in practice it is most useful to check
//! them at the VCode level if the goal is an end-to-end verification
//! of certain properties (e.g., memory sandboxing).
//!
//! Checking facts entails visiting each instruction that defines a
//! value with a fact, and checking the result's fact against the
//! facts on arguments and the operand. For VCode, this is
//! fundamentally a question of the target ISA's semantics, so we call
//! into the `LowerBackend` for this. Note that during checking there
//! is also limited forward propagation / inference, but only within
//! an instruction: for example, an addressing mode commonly can
//! include an addition, multiplication/shift, or extend operation,
//! and there is no way to attach facts to the intermediate values
//! "inside" the instruction, so instead the backend can use
//! `FactContext::add()` and friends to forward-propagate facts.
//!
//! TODO:
//!
//! Completeness:
//! - Propagate facts through optimization (egraph layer).
//! - Generate facts in cranelift-wasm frontend when lowering memory ops.
//! - Support bounds-checking-type operations for dynamic memories and
//!   tables.
//!
//! Generality:
//! - facts on outputs (in func signature)?
//! - Implement checking at the CLIF level as well.
//! - Check instructions that can trap as well?
//!
//! Nicer errors:
//! - attach instruction index or some other identifier to errors
//!
//! Refactoring:
//! - avoid the "default fact" infra everywhere we fetch facts,
//!   instead doing it in the subsume check (and take the type with
//!   subsume)?
//!
//! Text format cleanup:
//! - make the bitwidth on `max` facts optional in the CLIF text
//!   format?
//! - make offset in `mem` fact optional in the text format?
//!
//! Bikeshed colors (syntax):
//! - Put fact bang-annotations after types?
//!   `v0: i64 ! fact(..)` vs. `v0 ! fact(..): i64`

use crate::ir;
use crate::ir::types::*;
use crate::isa::TargetIsa;
use crate::machinst::{BlockIndex, LowerBackend, MachInst, VCode};
use regalloc2::Function as _;
use std::fmt;

#[cfg(feature = "enable-serde")]
use serde_derive::{Deserialize, Serialize};

/// The result of checking proof-carrying-code facts.
pub type PccResult<T> = std::result::Result<T, PccError>;

/// An error or inconsistency discovered when checking proof-carrying
/// code.
#[derive(Debug, Clone)]
pub enum PccError {
    /// An operation wraps around, invalidating the stated value
    /// range.
    Overflow,
    /// An input to an operator that produces a fact-annotated value
    /// does not have a fact describing it, and one is needed.
    MissingFact,
    /// A derivation of an output fact is unsupported (incorrect or
    /// not derivable).
    UnsupportedFact,
    /// A block parameter claims a fact that one of its predecessors
    /// does not support.
    UnsupportedBlockparam,
    /// A memory access is out of bounds.
    OutOfBounds,
    /// Proof-carrying-code checking is not implemented for a
    /// particular compiler backend.
    UnimplementedBackend,
    /// Proof-carrying-code checking is not implemented for a
    /// particular instruction that instruction-selection chose. This
    /// is an internal compiler error.
    UnimplementedInst,
    /// Access to an invalid or undefined field offset in a struct.
    InvalidFieldOffset,
    /// Access to a field via the wrong type.
    BadFieldType,
    /// Store to a read-only field.
    WriteToReadOnlyField,
    /// Store of data to a field with a fact that does not subsume the
    /// field's fact.
    InvalidStoredFact,
}

/// A fact on a value.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
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
        bit_width: u16,
        /// The maximum value that the bitslice can take
        /// (inclusive). The range is unsigned: the bits of the value
        /// will be within the range `0..=max`.
        max: u64,
    },

    /// A pointer to a memory type.
    Mem {
        /// The memory type.
        ty: ir::MemoryType,
        /// The offset into the memory type.
        offset: i64,
    },
}

impl fmt::Display for Fact {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Fact::ValueMax { bit_width, max } => write!(f, "max({}, {:#x})", bit_width, max),
            Fact::Mem { ty, offset } => write!(f, "mem({}, {:#x})", ty, offset),
        }
    }
}

impl Fact {
    /// Try to infer a minimal fact for a value of the given IR type.
    pub fn infer_from_type(ty: ir::Type) -> Option<&'static Self> {
        static FACTS: [Fact; 4] = [
            Fact::ValueMax {
                bit_width: 8,
                max: u8::MAX as u64,
            },
            Fact::ValueMax {
                bit_width: 16,
                max: u16::MAX as u64,
            },
            Fact::ValueMax {
                bit_width: 32,
                max: u32::MAX as u64,
            },
            Fact::ValueMax {
                bit_width: 64,
                max: u64::MAX,
            },
        ];
        match ty {
            I8 => Some(&FACTS[0]),
            I16 => Some(&FACTS[1]),
            I32 => Some(&FACTS[2]),
            I64 => Some(&FACTS[3]),
            _ => None,
        }
    }
}

macro_rules! ensure {
    ( $condition:expr, $err:tt $(,)? ) => {
        if !$condition {
            return Err(PccError::$err);
        }
    };
}

macro_rules! bail {
    ( $err:tt ) => {{
        return Err(PccError::$err);
    }};
}

/// A "context" in which we can evaluate and derive facts. This
/// context carries environment/global properties, such as the machine
/// pointer width.
pub struct FactContext<'a> {
    function: &'a ir::Function,
    pointer_width: u16,
}

impl<'a> FactContext<'a> {
    /// Create a new "fact context" in which to evaluate facts.
    pub fn new(function: &'a ir::Function, pointer_width: u16) -> Self {
        FactContext {
            function,
            pointer_width,
        }
    }

    /// Computes whether `lhs` "subsumes" (implies) `rhs`.
    pub fn subsumes(&self, lhs: &Fact, rhs: &Fact) -> bool {
        match (lhs, rhs) {
            // Reflexivity.
            (l, r) if l == r => true,

            // Any value on the LHS subsumes a minimal (always-true)
            // fact about the max value of a given bitwidth on the
            // RHS: e.g., no matter the value, the bottom 8 bits will
            // always be <= 255.
            (_, Fact::ValueMax { bit_width, max }) if *max == max_value_for_width(*bit_width) => {
                true
            }

            (
                Fact::ValueMax {
                    bit_width: bw_lhs,
                    max: max_lhs,
                },
                Fact::ValueMax {
                    bit_width: bw_rhs,
                    max: max_rhs,
                },
            ) => {
                // If the bitwidths we're claiming facts about are the
                // same, and if the value is less than or equal to
                // `max_lhs`, and if `max_rhs` is less than `max_lhs`,
                // then it is certainly less than or equal to
                // `max_rhs`.
                //
                // In other words, we can always expand the claimed
                // possible value range.
                bw_lhs == bw_rhs && max_lhs <= max_rhs
            }

            _ => false,
        }
    }

    /// Computes whether the optional fact `lhs` subsumes (implies)
    /// the optional fact `lhs`. A `None` never subsumes any fact, and
    /// is always subsumed by any fact at all (or no fact).
    pub fn subsumes_fact_optionals(&self, lhs: Option<&Fact>, rhs: Option<&Fact>) -> bool {
        match (lhs, rhs) {
            (None, None) => true,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (Some(lhs), Some(rhs)) => self.subsumes(lhs, rhs),
        }
    }

    /// Computes whatever fact can be known about the sum of two
    /// values with attached facts. The add is performed to the given
    /// bit-width. Note that this is distinct from the machine or
    /// pointer width: e.g., many 64-bit machines can still do 32-bit
    /// adds that wrap at 2^32.
    pub fn add(&self, lhs: &Fact, rhs: &Fact, add_width: u16) -> Option<Fact> {
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
            ) if bw_lhs == bw_rhs && add_width >= *bw_lhs => {
                let computed_max = lhs.checked_add(*rhs)?;
                let computed_max = std::cmp::min(max_value_for_width(add_width), computed_max);
                Some(Fact::ValueMax {
                    bit_width: *bw_lhs,
                    max: computed_max,
                })
            }

            (
                Fact::ValueMax {
                    bit_width: bw_max,
                    max,
                },
                Fact::Mem { ty, offset },
            )
            | (
                Fact::Mem { ty, offset },
                Fact::ValueMax {
                    bit_width: bw_max,
                    max,
                },
            ) if *bw_max >= self.pointer_width && add_width >= *bw_max => {
                let offset = offset.checked_add(i64::try_from(*max).ok()?)?;
                Some(Fact::Mem { ty: *ty, offset })
            }

            _ => None,
        }
    }

    /// Computes the `uextend` of a value with the given facts.
    pub fn uextend(&self, fact: &Fact, from_width: u16, to_width: u16) -> Option<Fact> {
        match fact {
            // If we have a defined value in bits 0..bit_width, and we
            // are filling zeroes into from_bits..to_bits, and
            // bit_width and from_bits are exactly contiguous, then we
            // have defined values in 0..to_bits (and because this is
            // a zero-extend, the max value is the same).
            Fact::ValueMax { bit_width, max } if *bit_width == from_width => Some(Fact::ValueMax {
                bit_width: to_width,
                max: *max,
            }),
            // Otherwise, we can at least claim that the value is
            // within the range of `to_width`.
            Fact::ValueMax { .. } => Some(Fact::ValueMax {
                bit_width: to_width,
                max: max_value_for_width(to_width),
            }),
            _ => None,
        }
    }

    /// Computes the `sextend` of a value with the given facts.
    pub fn sextend(&self, fact: &Fact, from_width: u16, to_width: u16) -> Option<Fact> {
        match fact {
            // If we have a defined value in bits 0..bit_width, and
            // the MSB w.r.t. `from_width` is *not* set, then we can
            // do the same as `uextend`.
            Fact::ValueMax { bit_width, max }
                if *bit_width == from_width && (*max & (1 << (*bit_width - 1)) == 0) =>
            {
                self.uextend(fact, from_width, to_width)
            }
            _ => None,
        }
    }

    /// Scales a value with a fact by a known constant.
    pub fn scale(&self, fact: &Fact, width: u16, factor: u32) -> Option<Fact> {
        // The minimal (loosest) fact we can claim: the value will be
        // within the range implied by its bitwidth.
        let minimal_fact = Fact::ValueMax {
            bit_width: width,
            max: max_value_for_width(width),
        };
        match fact {
            Fact::ValueMax { bit_width, max } if *bit_width == width => {
                let max = match max.checked_mul(u64::from(factor)) {
                    Some(max) => max,
                    None => return Some(minimal_fact),
                };
                if *bit_width < 64 && max > max_value_for_width(width) {
                    return Some(minimal_fact);
                }
                Some(Fact::ValueMax {
                    bit_width: *bit_width,
                    max,
                })
            }
            _ => None,
        }
    }

    /// Left-shifts a value with a fact by a known constant.
    pub fn shl(&self, fact: &Fact, width: u16, amount: u16) -> Option<Fact> {
        if amount >= 32 {
            return None;
        }
        let factor: u32 = 1 << amount;
        self.scale(fact, width, factor)
    }

    /// Offsets a value with a fact by a known amount.
    pub fn offset(&self, fact: &Fact, width: u16, offset: i64) -> Option<Fact> {
        match fact {
            Fact::ValueMax { bit_width, max } if *bit_width == width => {
                // If we eventually support two-sided ranges, we can
                // represent (0..n) + m -> ((0+m)..(n+m)). However,
                // right now, all ranges start with zero, so any
                // negative offset could underflow, and removes all
                // claims of constrained range.
                let offset = u64::try_from(offset).ok()?;

                let max = match max.checked_add(offset) {
                    Some(max) => max,
                    None => {
                        return Some(Fact::ValueMax {
                            bit_width: width,
                            max: max_value_for_width(width),
                        })
                    }
                };

                Some(Fact::ValueMax {
                    bit_width: *bit_width,
                    max,
                })
            }
            Fact::Mem {
                ty,
                offset: mem_offset,
            } => {
                let offset = mem_offset.checked_sub(offset)?;
                Some(Fact::Mem { ty: *ty, offset })
            }
            _ => None,
        }
    }

    /// Check that accessing memory via a pointer with this fact, with
    /// a memory access of the given size, is valid.
    ///
    /// If valid, returns the memory type and offset into that type
    /// that this address accesses.
    fn check_address(&self, fact: &Fact, size: u32) -> PccResult<(ir::MemoryType, i64)> {
        match fact {
            Fact::Mem { ty, offset } => {
                let end_offset: i64 = offset
                    .checked_add(i64::from(size))
                    .ok_or(PccError::Overflow)?;
                let end_offset: u64 =
                    u64::try_from(end_offset).map_err(|_| PccError::OutOfBounds)?;
                match &self.function.memory_types[*ty] {
                    ir::MemoryTypeData::Struct { size, .. }
                    | ir::MemoryTypeData::Memory { size } => {
                        ensure!(end_offset <= *size, OutOfBounds)
                    }
                    ir::MemoryTypeData::Empty => bail!(OutOfBounds),
                }
                Ok((*ty, *offset))
            }
            _ => bail!(OutOfBounds),
        }
    }

    /// Get the access struct field, if any, by a pointer with the
    /// given fact and an access of the given type.
    pub fn struct_field<'b>(
        &'b self,
        fact: &Fact,
        access_ty: ir::Type,
    ) -> PccResult<Option<&'b ir::MemoryTypeField>> {
        let (ty, offset) = self.check_address(fact, access_ty.bytes())?;
        let offset =
            u64::try_from(offset).expect("valid access address cannot have a negative offset");

        if let ir::MemoryTypeData::Struct { fields, .. } = &self.function.memory_types[ty] {
            let field = fields
                .iter()
                .find(|field| field.offset == offset)
                .ok_or(PccError::InvalidFieldOffset)?;
            if field.ty != access_ty {
                bail!(BadFieldType);
            }
            Ok(Some(field))
        } else {
            // Access to valid memory, but not a struct: no facts can be attached to the result.
            Ok(None)
        }
    }

    /// Check a load, and determine what fact, if any, the result of the load might have.
    pub fn load<'b>(&'b self, fact: &Fact, access_ty: ir::Type) -> PccResult<Option<&'b Fact>> {
        Ok(self
            .struct_field(fact, access_ty)?
            .and_then(|field| field.fact())
            .or_else(|| Fact::infer_from_type(access_ty)))
    }

    /// Check a store.
    pub fn store(
        &self,
        fact: &Fact,
        access_ty: ir::Type,
        data_fact: Option<&Fact>,
    ) -> PccResult<()> {
        if let Some(field) = self.struct_field(fact, access_ty)? {
            // If it's a read-only field, disallow.
            if field.readonly {
                bail!(WriteToReadOnlyField);
            }
            // Check that the fact on the stored data subsumes the field's fact.
            if !self.subsumes_fact_optionals(data_fact, field.fact()) {
                bail!(InvalidStoredFact);
            }
        }
        Ok(())
    }
}

fn max_value_for_width(bits: u16) -> u64 {
    assert!(bits <= 64);
    if bits == 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    }
}

/// Top-level entry point after compilation: this checks the facts in
/// VCode.
pub fn check_vcode_facts<B: LowerBackend + TargetIsa>(
    f: &ir::Function,
    vcode: &VCode<B::MInst>,
    backend: &B,
) -> PccResult<()> {
    let ctx = FactContext::new(f, backend.triple().pointer_width().unwrap().bits().into());

    // Check that individual instructions are valid according to input
    // facts, and support the stated output facts.
    for block in 0..vcode.num_blocks() {
        let block = BlockIndex::new(block);
        for inst in vcode.block_insns(block).iter() {
            if vcode.inst_defines_facts(inst) || vcode[inst].is_mem_access() {
                // This instruction defines a register with a new fact, or
                // has some side-effect we want to be careful to
                // verify. We'll call into the backend to validate this
                // fact with respect to the instruction and the input
                // facts.
                backend.check_fact(&ctx, vcode, &vcode[inst])?;
            }

            // If this is a branch, check that all block arguments subsume
            // the assumed facts on the blockparams of successors.
            if vcode.is_branch(inst) {
                for (succ_idx, succ) in vcode.block_succs(block).iter().enumerate() {
                    for (arg, param) in vcode
                        .branch_blockparams(block, inst, succ_idx)
                        .iter()
                        .zip(vcode.block_params(*succ).iter())
                    {
                        let arg_fact = vcode.vreg_fact(*arg);
                        let param_fact = vcode.vreg_fact(*param);
                        if !ctx.subsumes_fact_optionals(arg_fact, param_fact) {
                            return Err(PccError::UnsupportedBlockparam);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
