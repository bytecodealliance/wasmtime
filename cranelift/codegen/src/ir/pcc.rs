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
//! Correctness:
//! - Underflow/overflow: clear min and max respectively on all adds
//!   and subs
//!
//! Deployment:
//! - Add to fuzzing
//! - Turn on during wasm spec-tests
//!
//! More checks:
//! - Check that facts on `vmctx` GVs are subsumed by the actual facts
//!   on the vmctx arg in block0 (function arg).
//!
//! Generality:
//! - facts on outputs (in func signature)?
//! - Implement checking at the CLIF level as well.
//! - Check instructions that can trap as well?
//!
//! Nicer errors:
//! - attach instruction index or some other identifier to errors
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
use crate::machinst::{BlockIndex, LowerBackend, VCode};
use crate::trace;
use regalloc2::Function as _;
use smallvec::{smallvec, SmallVec};
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

/// A range in an integer space. This can be used to describe a value
/// or an offset into a memtype.
///
/// The value is described by three lists of symbolic expressions:
/// lower bounds (inclusive), exact equalities, and upper bounds
/// (inclusive).
///
/// We may need multiple such lower and upper bounds, and may want
/// bounds even if we have exact equalities, because comparison is a
/// *partial* relation: we can't say anything about how `v1` and `v2`
/// are related, so it may be useful to know that `x < v1`, and also
/// `x < v2`; or, say, that `x == v1` and also `x < v2`.
///
/// When producing a new range, we simplify these lists against each
/// other, so if one lower bound is greater than or equal to another,
/// or one upper bound is less than or equal to another, it will be
/// removed.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub struct ValueRange {
    /// Lower bounds (inclusive). The list specifies a set of bounds;
    /// the concrete value is greater than or equal to *all* of these
    /// bounds. If the list is empty, then there is no lower bound.
    pub min: SmallVec<[Expr; 1]>,
    /// Upper bounds (inclusive). The list specifies a set of bounds;
    /// the concrete value is less than or equal to *all* of these
    /// bounds. If the list is empty, then there is no upper bound.
    pub max: SmallVec<[Expr; 1]>,
    /// Equalties (inclusive). The list specifies a set of values all
    /// of which are known to be equal to the value described by this
    /// range. Note that if this is non-empty, the range's "size"
    /// (cardinality of the set of possible values) is exactly one
    /// value; but we may not know a concrete constant, and it is
    /// still useful to carry around the lower/upper bounds to enable
    /// further comparisons to be resolved.
    pub equal: SmallVec<[Expr; 1]>,
}

/// A fact on a value.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub enum Fact {
    /// A bitslice of a value (up to a bitwidth) is within the given
    /// integer range.
    ///
    /// The slicing behavior is needed because this fact can describe
    /// both an SSA `Value`, whose entire value is well-defined, and a
    /// `VReg` in VCode, whose bits beyond the type stored in that
    /// register are don't-care (undefined).
    Range {
        /// The bitwidth of bits we care about, from the LSB upward.
        bit_width: u16,
        /// The actual range.
        range: ValueRange,
    },

    /// A pointer to a memory type, with an offset inside the memory
    /// type specified as a range, and optionally nullable (can take
    /// on a zero/NULL pointer value) as well.
    Mem {
        /// The memory type.
        ty: ir::MemoryType,
        /// The range of offsets into this type.
        range: ValueRange,
        /// This pointer can also be null.
        nullable: bool,
    },

    /// A definition of a value to be used as a symbol in
    /// Exprs. There can only be one of these per value number.
    ///
    /// Note that this differs from a `DynamicRange` specifying that
    /// some value in the program is the same as `value`. A `def(v1)`
    /// fact is propagated to machine code and serves as a source of
    /// truth: the value or location labeled with this fact *defines*
    /// what `v1` is, and any `dynamic_range(64, v1, v1)`-labeled
    /// values elsewhere are claiming to be equal to this value.
    ///
    /// This is necessary because we don't propagate SSA value labels
    /// down to machine code otherwise; so when referring symbolically
    /// to addresses and expressions derived from addresses, we need
    /// to introduce the symbol first.
    Def {
        /// The SSA value this value defines.
        value: ir::Value,
    },

    /// A comparison result between two dynamic values with a
    /// comparison of a certain kind.
    Compare {
        /// The kind of comparison.
        kind: ir::condcodes::IntCC,
        /// The left-hand side of the comparison.
        lhs: Expr,
        /// The right-hand side of the comparison.
        rhs: Expr,
    },

    /// A "conflict fact": this fact results from merging two other
    /// facts, and it can never be satisfied -- checking any value
    /// against this fact will fail.
    Conflict,
}

/// A bound expression.
///
/// An expression consists of an (optional) symbolic base -- an SSA
/// value or a GlobalValue -- and a static offset.
///
/// Note that `Expr` obeys structural equality -- that is, two `Expr`s
/// represent actually-equal program values if the `Expr`s themselves
/// are structurally equal, and conversely, if they are not
/// structurally equal, then we *cannot prove* equality. There is no
/// such thing as an "unsimplified" form of an expression that is
/// statically equal but structurally unequal.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub enum Expr {
    /// A concrete static value in the range of a `u64`.
    Absolute(u64),
    /// An offset from an SSA value as a symbolic value. This can be
    /// referenced in facts even after we've lowered out of SSA -- it
    /// becomes an arbitrary symbolic base. The offset is an `i128`
    /// because it may be negative, but also must carry teh full range
    /// of a `u64` (e.g. if we add an `Absolute`).
    Value(ir::Value, i128),
    /// An offset from a GlobalValue as a symbolic value, as `Value`
    /// is for SSA values.
    GlobalValue(ir::GlobalValue, i128),
    /// Top of the address space. This is "saturating": the offset
    /// doesn't matter.
    Max,
}

impl Expr {
    /// Constant value.
    pub const fn constant(value: u64) -> Self {
        Expr::Absolute(value)
    }

    /// Maximum (saturated) value.
    pub const fn max_value() -> Self {
        Expr::Max
    }

    /// The value of an SSA value.
    pub const fn value(value: ir::Value) -> Self {
        Expr::Value(value, 0)
    }

    /// The value of an SSA value plus some offset.
    pub const fn value_offset(value: ir::Value, offset: i128) -> Self {
        Expr::Value(value, offset)
    }

    /// The value of a global value.
    pub const fn global_value(gv: ir::GlobalValue) -> Self {
        Expr::GlobalValue(gv, 0)
    }

    /// The value of a global value plus some offset.
    pub const fn global_value_offset(gv: ir::GlobalValue, offset: i128) -> Self {
        Expr::GlobalValue(gv, offset)
    }

    /// Is one expression definitely less than or equal to another?
    /// (We can't always know; in such cases, returns `false`.)
    fn le(lhs: &Expr, rhs: &Expr) -> bool {
        let result = match (lhs, rhs) {
            (_, Expr::Max) => true,
            (Expr::Absolute(0), _) => true,
            (Expr::Absolute(x), Expr::Absolute(y)) => x <= y,
            (Expr::Value(v1, x), Expr::Value(v2, y)) if v1 == v2 => x <= y,
            (Expr::GlobalValue(gv1, x), Expr::GlobalValue(gv2, y)) if gv1 == gv2 => x <= y,
            _ => false,
        };
        trace!("Expr::le: {lhs:?} {rhs:?} -> {result}");
        result
    }

    /// Add one expression to another.
    fn add(lhs: &Expr, rhs: &Expr) -> Expr {
        let result = match (lhs, rhs) {
            (Expr::Max, _) | (_, Expr::Max) => Expr::Max,
            (Expr::Absolute(x), Expr::Absolute(y)) => {
                x.checked_add(*y).map(Expr::Absolute).unwrap_or(Expr::Max)
            }
            (Expr::Value(v1, x), Expr::Value(v2, y)) if v1 == v2 => x
                .checked_add(*y)
                .map(|sum| Expr::Value(*v1, sum))
                .unwrap_or(Expr::Max),
            (Expr::GlobalValue(gv1, x), Expr::GlobalValue(gv2, y)) if gv1 == gv2 => x
                .checked_add(*y)
                .map(|sum| Expr::GlobalValue(*gv1, sum))
                .unwrap_or(Expr::Max),
            (Expr::Value(v, x), Expr::Absolute(off)) | (Expr::Absolute(off), Expr::Value(v, x)) => {
                Expr::Value(*v, *x + i128::from(*off))
            }
            (Expr::GlobalValue(gv, x), Expr::Absolute(off))
            | (Expr::Absolute(off), Expr::GlobalValue(gv, x)) => {
                Expr::GlobalValue(*gv, *x + i128::from(*off))
            }
            _ => Expr::Max,
        };
        trace!("Expr::add: {lhs:?} + {rhs:?} -> {result:?}");
        result
    }

    /// Add a static offset to an expression.
    pub fn offset(lhs: &Expr, rhs: i64) -> Option<Expr> {
        match lhs {
            Expr::Absolute(x) => Some(Expr::Absolute(
                u64::try_from(i128::from(*x) + i128::from(rhs)).ok()?,
            )),
            Expr::Value(v, x) => Some(Expr::Value(*v, *x + i128::from(rhs))),
            Expr::GlobalValue(gv, x) => Some(Expr::GlobalValue(*gv, *x + i128::from(rhs))),
            Expr::Max => Some(Expr::Max),
        }
    }

    /// Determine if we can know the difference between two expressions.
    pub fn difference(lhs: &Expr, rhs: &Expr) -> Option<i64> {
        match (lhs, rhs) {
            (Expr::Max, _) | (_, Expr::Max) => None,
            (Expr::Absolute(x), Expr::Absolute(y)) => {
                i64::try_from(*x).ok()?.checked_sub(i64::try_from(*y).ok()?)
            }
            (Expr::Value(v1, x), Expr::Value(v2, y)) if v1 == v2 => {
                i64::try_from(x.checked_sub(*y)?).ok()
            }
            (Expr::GlobalValue(gv1, x), Expr::GlobalValue(gv2, y)) if gv1 == gv2 => {
                i64::try_from(x.checked_sub(*y)?).ok()
            }
            _ => None,
        }
    }

    /// Multiply an expression by a constant, if possible.
    fn scale(&self, factor: u32) -> Option<Expr> {
        match self {
            Expr::Absolute(x) => Some(Expr::Absolute(x.checked_mul(u64::from(factor))?)),
            Expr::Max => Some(Expr::Max),
            _ => None,
        }
    }

    /// Multiply an expression by a constant, rounding downward if we
    /// must approximate.
    ///
    /// This is necessary to compute new lower bounds when scaling a range.
    fn scale_downward(&self, factor: u32) -> Expr {
        self.scale(factor).unwrap_or(Expr::constant(0))
    }

    /// Multiply an expression by a constant, rounding upward if we
    /// must approximate.
    ///
    /// This is necessary to compute new upper bounds when scaling a range.
    fn scale_upward(&self, factor: u32) -> Expr {
        self.scale(factor).unwrap_or(Expr::max_value())
    }

    /// Is this Expr an integer constant?
    fn as_const(&self) -> Option<i128> {
        match self {
            Expr::Absolute(x) => Some(i128::from(*x)),
            _ => None,
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expr::Absolute(x) => write!(f, "{x:#x}"),
            Expr::Value(v, off) => {
                if *off > 0 {
                    write!(f, "{v}+{off:#x}")
                } else if *off == 0 {
                    write!(f, "{v}")
                } else {
                    write!(f, "{v}-{neg:#x}", neg = -off)
                }
            }
            Expr::GlobalValue(gv, off) => {
                if *off >= 0 {
                    write!(f, "{gv}+{off:#x}")
                } else if *off == 0 {
                    write!(f, "{gv}")
                } else {
                    write!(f, "{gv}-{neg:#x}", neg = -off)
                }
            }
            Expr::Max => write!(f, "max"),
        }
    }
}

struct DisplayExprs<'a>(&'a [Expr]);

impl<'a> fmt::Display for DisplayExprs<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0.len() {
            0 => write!(f, "{{}}"),
            1 => write!(f, "{}", self.0[0]),
            _ => {
                write!(f, "{{")?;

                let mut first = true;
                for expr in self.0 {
                    if first {
                        write!(f, " {expr}")?;
                        first = false;
                    } else {
                        write!(f, ", {expr}")?;
                    }
                }

                write!(f, " }}")?;
                Ok(())
            }
        }
    }
}

impl fmt::Display for ValueRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.equal.is_empty() {
            write!(
                f,
                "{}, {}",
                DisplayExprs(&self.min[..]),
                DisplayExprs(&self.max[..])
            )
        } else if self.min.is_empty() && self.max.is_empty() {
            write!(f, "={}", DisplayExprs(&self.equal[..]))
        } else {
            write!(
                f,
                "{}, ={}, {}",
                DisplayExprs(&self.min[..]),
                DisplayExprs(&self.equal[..]),
                DisplayExprs(&self.max[..])
            )
        }
    }
}

impl fmt::Display for Fact {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Fact::Range { bit_width, range } => {
                write!(f, "range({bit_width}, {range})")
            }
            Fact::Mem {
                ty,
                range,
                nullable,
            } => {
                let nullable_flag = if *nullable { ", nullable" } else { "" };
                write!(f, "mem({ty}{nullable_flag}, {range})")
            }
            Fact::Def { value } => write!(f, "def({value})"),
            Fact::Compare { kind, lhs, rhs } => {
                write!(f, "compare({kind}, {lhs}, {rhs})")
            }
            Fact::Conflict => write!(f, "conflict"),
        }
    }
}

impl ValueRange {
    /// Create a range that is exactly one expression.
    pub fn exact(expr: Expr) -> Self {
        ValueRange {
            equal: smallvec![expr],
            min: smallvec![],
            max: smallvec![],
        }
    }

    /// Create a range that has a min and max.
    pub fn min_max(min: Expr, max: Expr) -> Self {
        ValueRange {
            equal: smallvec![],
            min: smallvec![min],
            max: smallvec![max],
        }
    }

    /// Create a range that is exactly one expression, with another expression as an upper bound.
    pub fn exact_with_max(expr: Expr, max: Expr) -> Self {
        ValueRange {
            equal: smallvec![expr],
            min: smallvec![],
            max: smallvec![max],
        }
    }

    /// Is this ValueRange an exact integer constant?
    pub fn as_const(&self) -> Option<i128> {
        self.equal.iter().find_map(|&e| e.as_const())
    }

    /// Is this ValueRange definitely less than or equal to the given expression?
    pub fn le_expr(&self, expr: &Expr) -> bool {
        // The range is <= the expr if *any* of its upper bounds are
        // <= the expr, because each upper bound constrains the whole
        // range (i.e., the range is the intersection of all
        // combinations of bounds). Likewise, if any expression that
        // exactly determines the value less than `expr`, then we can
        // definitely say the range is less than `expr`.
        let result = self
            .equal
            .iter()
            .chain(self.max.iter())
            .any(|e| Expr::le(e, expr));
        trace!("ValueRange::le_expr: {self:?} {expr:?} -> {result}");
        result
    }

    /// Is the expression definitely within the ValueRange?
    pub fn contains_expr(&self, expr: &Expr) -> bool {
        let result = ((!self.min.is_empty() || !self.max.is_empty())
            && self
                .min
                .iter()
                .all(|lower_bound| Expr::le(lower_bound, expr))
            && self
                .max
                .iter()
                .all(|upper_bound| Expr::le(expr, upper_bound)))
            || self.equal.iter().any(|equiv| equiv == expr);
        trace!("ValueRange::contains_expr: {self:?} {expr:?} -> {result}");
        result
    }

    /// Simplify a ValueRange by removing redundant bounds. Any lower
    /// bound greater than another lower bound, or any upper bound
    /// less than another upper bound, can be removed.
    pub fn simplify(&mut self) {
        trace!("simplify: {self:?}");

        // Note an important invariant: syntactic equality of Exprs
        // implies symbolic equality. This is required to ensure we
        // don't remove both `x` and `y` if `x <= y` and `y <= x`,
        // given the logic below.
        self.equal.sort();
        self.equal.dedup();

        // A lower bound `e` is not redundant if for all other
        // lower bounds `other`, we cannot show that `e >=
        // other`.
        self.min.sort();
        self.min.dedup();
        let min = self
            .min
            .iter()
            .filter(|&e| {
                self.min
                    .iter()
                    .all(|other| e == other || !Expr::le(other, e))
            })
            .cloned()
            .collect::<SmallVec<[Expr; 1]>>();
        self.min = min;

        // Likewise, an upper bound `e` is not redundant if
        // for all other upper bounds `other`, we cannot show
        // that `other >= e`.
        self.max.sort();
        self.max.dedup();
        let max = self
            .max
            .iter()
            .filter(|&e| {
                self.min
                    .iter()
                    .all(|other| e == other || !Expr::le(e, other))
            })
            .cloned()
            .collect::<SmallVec<[Expr; 1]>>();
        self.max = max;

        trace!("simplify: produced {self:?}");
    }

    /// Does one ValueRange contain another? Assumes both sides are already simplified.
    pub fn contains(&self, other: &ValueRange) -> bool {
        let result = other.equal.iter().any(|e| self.contains_expr(e)) ||
        // *Some* lower bound and *some* upper bound of the RHS must
        // be contained in the LHS. Either those lower and upper
        // bounds are tight, in which case all values between them are
        // then contained in the LHS; or they are loose, and the true
        // range is contained within them, which in turn is contained
        // in the LHS.
            (other.min
             .iter()
             .any(|lower_bound2| self.contains_expr(lower_bound2))
             || self.contains_expr(&Expr::constant(0)))
            && (other.max
                .iter()
                .any(|upper_bound2| self.contains_expr(upper_bound2))
                || self.contains_expr(&Expr::max_value()));
        trace!("ValueRange::contains: {self:?} {other:?} -> {result}");
        result
    }

    /// Intersect two ValueRanges.
    pub fn intersect(lhs: &ValueRange, rhs: &ValueRange) -> ValueRange {
        let equal = lhs
            .equal
            .iter()
            .cloned()
            .chain(rhs.equal.iter().cloned())
            .collect();
        let min = lhs
            .min
            .iter()
            .cloned()
            .chain(rhs.min.iter().cloned())
            .collect();
        let max = lhs
            .max
            .iter()
            .cloned()
            .chain(rhs.max.iter().cloned())
            .collect();
        let mut result = ValueRange { equal, min, max };
        result.simplify();
        result
    }

    /// Take the union of two ranges.
    pub fn union(lhs: &ValueRange, rhs: &ValueRange) -> ValueRange {
        // Take lower bounds from LHS that are less than all
        // lower bounds on the RHS; and likewise the other
        // way; and likewise for upper bounds.
        let min = lhs
            .min
            .iter()
            .filter(|&e| rhs.min.iter().all(|e2| Expr::le(e, e2)))
            .cloned()
            .chain(
                rhs.min
                    .iter()
                    .filter(|e| lhs.min.iter().all(|e2| Expr::le(e, e2)))
                    .cloned(),
            )
            .collect();
        let max = lhs
            .max
            .iter()
            .filter(|&e| rhs.max.iter().all(|e2| Expr::le(e2, e)))
            .cloned()
            .chain(
                rhs.max
                    .iter()
                    .filter(|e| lhs.max.iter().all(|e2| Expr::le(e2, e)))
                    .cloned(),
            )
            .collect();
        let equal = lhs
            .equal
            .iter()
            .filter(|&e| rhs.equal.iter().any(|e2| e == e2))
            .cloned()
            .chain(
                rhs.equal
                    .iter()
                    .filter(|&e| lhs.equal.iter().any(|e2| e == e2))
                    .cloned(),
            )
            .collect();
        let mut result = ValueRange { min, max, equal };
        result.simplify();
        result
    }

    /// Scale a range by a factor.
    pub fn scale(&self, factor: u32) -> ValueRange {
        let equal = self.equal.iter().filter_map(|e| e.scale(factor)).collect();
        let min = self.min.iter().map(|e| e.scale_downward(factor)).collect();
        let max = self.max.iter().map(|e| e.scale_upward(factor)).collect();
        let mut result = ValueRange { equal, min, max };
        result.simplify();
        result
    }

    /// Add an offset to the lower and upper bounds of a range.
    pub fn offset(&self, offset: i64) -> ValueRange {
        let equal = self
            .equal
            .iter()
            .flat_map(|e| Expr::offset(e, offset))
            .collect();
        let min = self
            .min
            .iter()
            .flat_map(|e| Expr::offset(e, offset))
            .collect();
        let max = self
            .max
            .iter()
            .flat_map(|e| Expr::offset(e, offset))
            .collect();
        let mut result = ValueRange { equal, min, max };
        result.simplify();
        result
    }

    /// Find the range of the sum of two values described by ranges.
    pub fn add(lhs: &ValueRange, rhs: &ValueRange) -> ValueRange {
        trace!("ValueRange::add: {lhs:?} + {rhs:?}");
        let min = lhs
            .min
            .iter()
            .chain(lhs.equal.iter())
            .flat_map(|m1| {
                rhs.min
                    .iter()
                    .chain(rhs.equal.iter())
                    .map(|m2| Expr::add(m1, m2))
            })
            .collect();
        let max = lhs
            .max
            .iter()
            .chain(lhs.equal.iter())
            .flat_map(|m1| {
                rhs.max
                    .iter()
                    .chain(rhs.equal.iter())
                    .map(|m2| Expr::add(m1, m2))
            })
            .collect();
        let equal = lhs
            .equal
            .iter()
            .flat_map(|m1| rhs.equal.iter().map(|m2| Expr::add(m1, m2)))
            .collect();
        let mut result = ValueRange { equal, min, max };
        trace!(" -> inclusive + inclusive: {result:?}");
        result.simplify();
        trace!(" -> {result:?}");
        result
    }

    /// Clamp a ValueRange given a bit-width for the result.
    fn clamp(mut self, width: u16) -> ValueRange {
        trace!("ValueRange::clamp: {self:?} width {width}");
        self.max.push(Expr::constant(max_value_for_width(width)));
        self.simplify();
        trace!("ValueRange::clamp: -> {self:?}");
        self
    }
}

impl Fact {
    /// Create a range fact that specifies a single known constant value.
    pub fn constant(bit_width: u16, value: u64) -> Self {
        debug_assert!(value <= max_value_for_width(bit_width));
        // `min` and `max` are inclusive, so this specifies a range of
        // exactly one value.
        Fact::Range {
            bit_width,
            range: ValueRange::exact(Expr::constant(value)),
        }
    }

    /// Create a range fact that points to the base of a memory type.
    pub fn dynamic_base_ptr(ty: ir::MemoryType) -> Self {
        Fact::Mem {
            ty,
            range: ValueRange::exact(Expr::constant(0)),
            nullable: false,
        }
    }

    /// Create a fact that specifies the value is exactly an SSA value.
    ///
    /// Note that this differs from a `def` fact: it is not *defining*
    /// a symbol to have the value that this fact is attached to;
    /// rather it is claiming that this value is the same as whatever
    /// that symbol is. (In other words, the def should be elsewhere,
    /// and we are tying ourselves to it.)
    pub fn value(bit_width: u16, value: ir::Value) -> Self {
        Fact::Range {
            bit_width,
            range: ValueRange::exact_with_max(
                Expr::value(value),
                Expr::constant(max_value_for_width(bit_width)),
            ),
        }
    }

    /// Create a fact that specifies the value is exactly an SSA value plus some offset.
    pub fn value_offset(bit_width: u16, value: ir::Value, offset: i64) -> Self {
        Fact::Range {
            bit_width,
            range: ValueRange::exact_with_max(
                Expr::value_offset(value, offset.into()),
                Expr::constant(max_value_for_width(bit_width)),
            ),
        }
    }

    /// Create a fact that specifies the value is exactly the value of a GV.
    pub fn global_value(bit_width: u16, gv: ir::GlobalValue) -> Self {
        Fact::Range {
            bit_width,
            range: ValueRange::exact_with_max(
                Expr::global_value(gv),
                Expr::constant(max_value_for_width(bit_width)),
            ),
        }
    }

    /// Create a fact that specifies the value is exactly the value of a GV plus some offset.
    pub fn global_value_offset(bit_width: u16, gv: ir::GlobalValue, offset: i64) -> Self {
        Fact::Range {
            bit_width,
            range: ValueRange::exact_with_max(
                Expr::global_value_offset(gv, offset.into()),
                Expr::constant(max_value_for_width(bit_width)),
            ),
        }
    }

    /// Create a fact that expresses a given static range, from zero
    /// up to `max` (inclusive).
    pub fn static_value_range(bit_width: u16, max: u64) -> Self {
        Fact::Range {
            bit_width,
            range: ValueRange::min_max(Expr::constant(0), Expr::constant(max)),
        }
    }

    /// Create a fact that expresses a given static range, from `min`
    /// (inclusive) up to `max` (inclusive).
    pub fn static_value_two_ended_range(bit_width: u16, min: u64, max: u64) -> Self {
        if min == max {
            Fact::constant(bit_width, min)
        } else {
            Fact::Range {
                bit_width,
                range: ValueRange::min_max(Expr::constant(min), Expr::constant(max)),
            }
        }
    }

    /// Create a fact that expresses a given dynamic range, from zero up to `expr`.
    pub fn dynamic_value_range(bit_width: u16, max: Expr) -> Self {
        Fact::Range {
            bit_width,
            range: ValueRange::min_max(Expr::constant(0), max),
        }
    }

    /// Create a range fact that specifies the maximum range for a
    /// value of the given bit-width.
    pub fn max_range_for_width(bit_width: u16) -> Self {
        Fact::Range {
            bit_width,
            range: ValueRange::min_max(
                Expr::constant(0),
                Expr::constant(max_value_for_width(bit_width)),
            ),
        }
    }

    /// Create a fact that describes the base pointer for a memory
    /// type.
    pub fn memory_base(ty: ir::MemoryType) -> Self {
        Fact::Mem {
            ty,
            range: ValueRange::exact(Expr::constant(0)),
            nullable: false,
        }
    }

    /// Create a fact that describes a pointer to the given memory
    /// type with an offset described by the given fact.
    pub fn memory_with_range(
        ty: ir::MemoryType,
        offset_fact: Fact,
        nullable: bool,
    ) -> Option<Self> {
        let Fact::Range {
            bit_width: _,
            range,
        } = offset_fact
        else {
            return None;
        };
        Some(Fact::Mem {
            ty,
            range,
            nullable,
        })
    }

    /// Create a range fact that specifies the maximum range for a
    /// value of the given bit-width, zero-extended into a wider
    /// width.
    pub fn max_range_for_width_extended(from_width: u16, to_width: u16) -> Self {
        debug_assert!(from_width <= to_width);
        let upper_bound = if from_width <= 64 {
            Expr::constant(max_value_for_width(from_width))
        } else {
            Expr::max_value()
        };
        Fact::Range {
            bit_width: to_width,
            range: ValueRange::min_max(Expr::constant(0), upper_bound),
        }
    }

    /// Try to infer a minimal fact for a value of the given IR type.
    pub fn infer_from_type(ty: ir::Type) -> Option<Self> {
        match ty {
            I8 | I16 | I32 | I64 => {
                Some(Self::max_range_for_width(u16::try_from(ty.bits()).unwrap()))
            }
            _ => None,
        }
    }

    /// Does this fact "propagate" automatically, i.e., cause
    /// instructions that process it to infer their own output facts?
    /// Not all facts propagate automatically; otherwise, verification
    /// would be much slower.
    pub fn propagates(&self) -> bool {
        match self {
            Fact::Mem { .. } => true,
            _ => false,
        }
    }

    /// Merge two facts. We take the *intersection*: that is, we know
    /// both facts to be true, so we can intersect ranges. (This
    /// differs from the usual static analysis approach, where we are
    /// merging multiple possibilities into a generalized / widened
    /// fact. We want to narrow here.)
    pub fn intersect(a: &Fact, b: &Fact) -> Fact {
        let result = match (a, b) {
            (
                Fact::Range {
                    bit_width: bw_lhs,
                    range: range1,
                },
                Fact::Range {
                    bit_width: bw_rhs,
                    range: range2,
                },
            ) if bw_lhs == bw_rhs => Fact::Range {
                bit_width: *bw_lhs,
                range: ValueRange::intersect(range1, range2),
            },

            (
                Fact::Mem {
                    ty: ty_lhs,
                    range: range1,
                    nullable: nullable_lhs,
                },
                Fact::Mem {
                    ty: ty_rhs,
                    range: range2,
                    nullable: nullable_rhs,
                },
            ) if ty_lhs == ty_rhs => Fact::Mem {
                ty: *ty_lhs,
                range: ValueRange::intersect(range1, range2),
                nullable: *nullable_lhs && *nullable_rhs,
            },

            (Fact::Def { value: v1 }, Fact::Def { value: v2 }) => Fact::Def {
                value: std::cmp::min(*v1, *v2),
            },

            (
                Fact::Compare {
                    kind: kind1,
                    lhs: lhs1,
                    rhs: rhs1,
                },
                Fact::Compare {
                    kind: kind2,
                    lhs: lhs2,
                    rhs: rhs2,
                },
            ) if kind1 == kind2 => {
                if (lhs1, rhs1) <= (lhs2, rhs2) {
                    Fact::Compare {
                        kind: *kind1,
                        lhs: *lhs1,
                        rhs: *rhs1,
                    }
                } else {
                    Fact::Compare {
                        kind: *kind2,
                        lhs: *lhs2,
                        rhs: *rhs2,
                    }
                }
            }

            _ => Fact::Conflict,
        };
        trace!("Fact::intersect: {a:?} {b:?} -> {result:?}");
        result
    }

    /// Take the union of two facts: produce a fact that applies to a
    /// value that has either one fact or another (e.g., at a
    /// control-flow merge point or a conditional-select operator).
    pub fn union(a: &Fact, b: &Fact) -> Fact {
        let result = match (a, b) {
            (
                Fact::Range {
                    bit_width: bw_lhs,
                    range: range1,
                },
                Fact::Range {
                    bit_width: bw_rhs,
                    range: range2,
                },
            ) if bw_lhs == bw_rhs => Fact::Range {
                bit_width: *bw_lhs,
                range: ValueRange::union(range1, range2),
            },

            (
                Fact::Mem {
                    ty: ty_lhs,
                    range: range1,
                    nullable: nullable_lhs,
                },
                Fact::Mem {
                    ty: ty_rhs,
                    range: range2,
                    nullable: nullable_rhs,
                },
            ) if ty_lhs == ty_rhs => Fact::Mem {
                ty: *ty_lhs,
                range: ValueRange::union(range1, range2),
                nullable: *nullable_lhs || *nullable_rhs,
            },

            (
                Fact::Mem {
                    ty: ty_mem,
                    range: range_mem,
                    nullable: _,
                },
                Fact::Range {
                    bit_width: _,
                    range: range_offset,
                },
            )
            | (
                Fact::Range {
                    bit_width: _,
                    range: range_offset,
                },
                Fact::Mem {
                    ty: ty_mem,
                    range: range_mem,
                    nullable: _,
                },
            ) if range_offset.le_expr(&Expr::constant(0)) => Fact::Mem {
                ty: *ty_mem,
                range: range_mem.clone(),
                nullable: true,
            },

            _ => Fact::Conflict,
        };
        trace!("Fact::union: {a:?} {b:?} -> {result:?}");
        result
    }

    /// Does this fact describe an exact expression?
    pub fn as_expr(&self) -> Option<&Expr> {
        match self {
            Fact::Range {
                range: ValueRange { equal, .. },
                ..
            } => equal.first(),
            _ => None,
        }
    }

    /// Does this fact describe a constant?
    pub fn as_const(&self) -> Option<i128> {
        match self {
            Fact::Range { range, .. } => range.as_const(),
            _ => None,
        }
    }

    /// Offsets a value with a fact by a known amount.
    pub fn offset(&self, width: u16, offset: i64) -> Option<Fact> {
        if offset == 0 {
            return Some(self.clone());
        }

        let result = match self {
            Fact::Range { bit_width, range } if *bit_width == width => Some(Fact::Range {
                bit_width: *bit_width,
                range: range.offset(offset.into()).clamp(width),
            }),
            Fact::Mem {
                ty,
                range,
                nullable: false,
            } => Some(Fact::Mem {
                ty: *ty,
                range: range.offset(offset.into()).clamp(width),
                nullable: false,
            }),
            _ => None,
        };
        trace!("offset: {self:?} + {offset} in width {width} -> {result:?}");
        result
    }

    /// Get the range of a fact: either the actual value range, or the
    /// range of offsets into a memory type.
    pub fn range(&self) -> Option<&ValueRange> {
        match self {
            Fact::Range { range, .. } | Fact::Mem { range, .. } => Some(range),
            _ => None,
        }
    }

    /// Update the range in either a Range or Mem fact.
    pub fn with_range(&self, range: ValueRange) -> Fact {
        match self {
            Fact::Range { bit_width, .. } => Fact::Range {
                bit_width: *bit_width,
                range,
            },
            Fact::Mem { ty, nullable, .. } => Fact::Mem {
                ty: *ty,
                nullable: *nullable,
                range,
            },
            f => f.clone(),
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

/// The two kinds of inequalities: "strict" (`<`, `>`) and "loose"
/// (`<=`, `>=`), the latter of which admit equality.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InequalityKind {
    /// Strict inequality: {less,greater}-than.
    Strict,
    /// Loose inequality: {less,greater}-than-or-equal.
    Loose,
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
        trace!("subsumes {lhs:?} {rhs:?}");
        match (lhs, rhs) {
            // Reflexivity.
            (l, r) if l == r => true,

            (
                Fact::Range {
                    bit_width: bw_lhs,
                    range: range_lhs,
                },
                Fact::Range {
                    bit_width: bw_rhs,
                    range: range_rhs,
                },
            ) if bw_lhs == bw_rhs => range_rhs.contains(range_lhs),

            (
                Fact::Range {
                    bit_width: bw_lhs,
                    range: range_lhs,
                },
                Fact::Range {
                    bit_width: bw_rhs,
                    range: range_rhs,
                },
            ) if bw_lhs > bw_rhs => {
                // If the LHS makes a claim about a larger bitwidth,
                // then it can still imply the RHS if the RHS claims
                // the full range of its width.
                let rhs_is_trivially_true = range_rhs.contains_expr(&Expr::constant(0))
                    && range_rhs.contains_expr(&Expr::constant(max_value_for_width(*bw_rhs)));
                // It can also still imply the RHS if the LHS's range
                // is within the bitwidth of the RHS and the RHS
                // otherwise contains the LHS's range, so we don't
                // have to worry about truncation/aliasing effects.
                let lhs_is_in_rhs_width_range =
                    range_lhs.le_expr(&Expr::constant(max_value_for_width(*bw_rhs)));

                rhs_is_trivially_true
                    || (lhs_is_in_rhs_width_range && range_rhs.contains(range_lhs))
            }

            (
                Fact::Mem {
                    ty: ty_lhs,
                    range: range_lhs,
                    nullable: nullable_lhs,
                },
                Fact::Mem {
                    ty: ty_rhs,
                    range: range_rhs,
                    nullable: nullable_rhs,
                },
            ) => {
                ty_lhs == ty_rhs
                    && range_rhs.contains(range_lhs)
                    && (*nullable_lhs || !*nullable_rhs)
            }

            // Constant zero subsumes nullable DynamicMem pointers.
            (
                Fact::Range {
                    bit_width, range, ..
                },
                Fact::Mem { nullable: true, .. },
            ) if *bit_width == self.pointer_width && range.le_expr(&Expr::constant(0)) => true,

            // Any fact subsumes a Def, because the Def makes no
            // claims about the actual value (it ties a symbol to that
            // value, but the value is fed to the symbol, not the
            // other way around).
            (_, Fact::Def { .. }) => true,

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
        let result = match (lhs, rhs) {
            (
                Fact::Range {
                    bit_width: bw_lhs,
                    range: range_lhs,
                },
                Fact::Range {
                    bit_width: bw_rhs,
                    range: range_rhs,
                },
            ) if bw_lhs == bw_rhs && add_width >= *bw_lhs => Some(Fact::Range {
                bit_width: *bw_lhs,
                range: ValueRange::add(range_lhs, range_rhs).clamp(add_width),
            }),

            (
                Fact::Range {
                    bit_width: bw_lhs,
                    range: range_lhs,
                },
                Fact::Mem {
                    ty,
                    range: range_rhs,
                    nullable,
                },
            )
            | (
                Fact::Mem {
                    ty,
                    range: range_rhs,
                    nullable,
                },
                Fact::Range {
                    bit_width: bw_lhs,
                    range: range_lhs,
                },
            ) if *bw_lhs >= self.pointer_width
                && add_width >= *bw_lhs
                // A null pointer doesn't remain a null pointer unless
                // the right-hand side is constant zero.
                && (!*nullable || range_lhs.le_expr(&Expr::constant(0))) =>
            {
                Some(Fact::Mem {
                    ty: *ty,
                    range: ValueRange::add(range_lhs, range_rhs).clamp(add_width),
                    nullable: *nullable,
                })
            }

            _ => None,
        };

        trace!("add({add_width}): {lhs:?} + {rhs:?} -> {result:?}");
        result
    }

    /// Computes the `uextend` of a value with the given facts.
    pub fn uextend(&self, fact: &Fact, from_width: u16, to_width: u16) -> Option<Fact> {
        if from_width == to_width {
            return Some(fact.clone());
        }

        let result = match fact {
            Fact::Range { bit_width, range } if *bit_width == from_width => Some(Fact::Range {
                bit_width: to_width,
                range: range.clone(),
            }),

            // If the claim is a definition of a value, we can say
            // that the output has a range of exactly that value.
            Fact::Def { value } => Some(Fact::Range {
                bit_width: to_width,
                range: ValueRange::exact_with_max(
                    Expr::value(*value),
                    Expr::constant(max_value_for_width(from_width)),
                ),
            }),

            // Otherwise, we can at least claim that the value is
            // within the range of `from_width`.
            Fact::Range { .. } => Some(Fact::max_range_for_width_extended(from_width, to_width)),

            _ => None,
        };
        trace!("uextend: fact {fact:?} from {from_width} to {to_width} -> {result:?}");
        result
    }

    /// Computes the `sextend` of a value with the given facts.
    pub fn sextend(&self, fact: &Fact, from_width: u16, to_width: u16) -> Option<Fact> {
        let max_positive_value = 1u64 << (from_width - 1);
        match fact {
            // If we have a defined value in bits 0..bit_width, and
            // the MSB w.r.t. `from_width` is *not* set, then we can
            // do the same as `uextend`.
            Fact::Range {
                bit_width, range, ..
            } if *bit_width == from_width && range.le_expr(&Expr::constant(max_positive_value)) => {
                self.uextend(fact, from_width, to_width)
            }
            _ => None,
        }
    }

    /// Computes the bit-truncation of a value with the given fact.
    pub fn truncate(&self, fact: &Fact, from_width: u16, to_width: u16) -> Option<Fact> {
        if from_width == to_width {
            return Some(fact.clone());
        }

        trace!(
            "truncate: fact {:?} from {} to {}",
            fact,
            from_width,
            to_width
        );

        match fact {
            Fact::Range { bit_width, range } if *bit_width == from_width => {
                let max_val = (1u64 << to_width) - 1;
                if range.le_expr(&Expr::constant(max_val)) {
                    Some(Fact::Range {
                        bit_width: to_width,
                        range: range.clone(),
                    })
                } else {
                    Some(Fact::max_range_for_width(to_width))
                }
            }
            _ => None,
        }
    }

    /// Scales a value with a fact by a known constant.
    pub fn scale(&self, fact: &Fact, width: u16, factor: u32) -> Option<Fact> {
        let result = match fact {
            x if factor == 1 => Some(x.clone()),
            Fact::Range { bit_width, range } if *bit_width == width => Some(Fact::Range {
                bit_width: *bit_width,
                range: range.scale(factor).clamp(width),
            }),
            _ => None,
        };
        trace!("scale: {fact:?} * {factor} at width {width} -> {result:?}");
        result
    }

    /// Left-shifts a value with a fact by a known constant.
    pub fn shl(&self, fact: &Fact, width: u16, amount: u16) -> Option<Fact> {
        if amount >= 32 {
            return None;
        }
        let factor: u32 = 1 << amount;
        self.scale(fact, width, factor)
    }

    /// Check that accessing memory via a pointer with this fact, with
    /// a memory access of the given size, is valid.
    ///
    /// If valid, returns the memory type and offset into that type
    /// that this address accesses, if known, or `None` if the range
    /// doesn't constrain the access to exactly one location.
    fn check_address(
        &self,
        fact: &Fact,
        access_size: u32,
    ) -> PccResult<Option<(ir::MemoryType, u64)>> {
        trace!("check_address: fact {:?} access_size {}", fact, access_size);

        match fact {
            Fact::Mem {
                ty,
                range,
                nullable: _,
            } => {
                trace!(" -> memory type: {}", self.function.memory_types[*ty]);
                match &self.function.memory_types[*ty] {
                    ir::MemoryTypeData::Struct { size, .. }
                    | ir::MemoryTypeData::Memory { size } => {
                        ensure!(u64::from(access_size) <= *size, OutOfBounds);
                        let effective_size = *size - u64::from(access_size);
                        ensure!(range.le_expr(&Expr::constant(effective_size)), OutOfBounds);
                    }
                    ir::MemoryTypeData::DynamicMemory {
                        gv,
                        size: mem_static_size,
                    } => {
                        let effective_size = i128::from(*mem_static_size) - i128::from(access_size);
                        let end = Expr::global_value_offset(*gv, effective_size);
                        ensure!(range.le_expr(&end), OutOfBounds)
                    }
                    ir::MemoryTypeData::Empty => bail!(OutOfBounds),
                }
                let specific_ty_and_offset =
                    if let Some(constant) = range.as_const().and_then(|i| u64::try_from(i).ok()) {
                        Some((*ty, constant))
                    } else {
                        None
                    };
                trace!(" -> specific type and offset: {specific_ty_and_offset:?}");
                Ok(specific_ty_and_offset)
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
        let (ty, offset) = match self.check_address(fact, access_ty.bytes())? {
            Some((ty, offset)) => (ty, offset),
            None => return Ok(None),
        };

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
            // Access to valid memory, but not a struct: no facts can
            // be attached to the result.
            Ok(None)
        }
    }

    /// Check a load, and determine what fact, if any, the result of
    /// the load might have.
    pub fn load<'b>(&'b self, fact: &Fact, access_ty: ir::Type) -> PccResult<Option<&'b Fact>> {
        Ok(self
            .struct_field(fact, access_ty)?
            .and_then(|field| field.fact()))
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
            // Check that the fact on the stored data subsumes the
            // field's fact.
            if !self.subsumes_fact_optionals(data_fact, field.fact()) {
                bail!(InvalidStoredFact);
            }
        }
        Ok(())
    }

    /// Apply a known inequality to rewrite dynamic bounds using
    /// transitivity, if possible.
    ///
    /// Given that `lhs >= rhs` (if `kind` is not `strict`) or `lhs >
    /// rhs` (if `kind` is `strict`), update `fact`.
    pub fn apply_inequality(
        &self,
        fact: &Fact,
        lhs: &Fact,
        rhs: &Fact,
        kind: InequalityKind,
    ) -> Fact {
        trace!("apply_inequality: fact {fact:?} lhs {lhs:?} rhs {rhs:?} kind {kind:?}");

        // The basic idea is that if `fact` is <= RHS, and RHS <= LHS,
        // then we know that `fact` is <= LHS as well (transitivity).
        //
        // We thus first check if `fact` is indeed <= RHS: are any of
        // its upper bounds <= any lower or equal bounds on RHS? If
        // so, what is the minimum headroom (known difference)? E.g.,
        // if `fact` is known to be `v1`, and RHS is equal to or
        // greater than `v1 + 4`, then the known difference is at
        // least 4.
        //
        // If such a difference is known, we then take all lower,
        // equal and upper bounds of LHS, add that offset, and add
        // these as upper bounds on `fact`. So for example, if we know
        // that `v1 + 4 <= gv1`, then we can update the fact to be
        // `range(bit_width, {}, =v1, gv1 - 4)`: it is still equal to
        // `v1`, but it is also at most `gv1 - 4`.

        let result = if let (Some(fact_range), Some(lhs_range), Some(rhs_range)) =
            (fact.range(), lhs.range(), rhs.range())
        {
            let offset = fact_range
                .equal
                .iter()
                .chain(fact_range.max.iter())
                .flat_map(|fact_expr| {
                    rhs_range
                        .min
                        .iter()
                        .chain(rhs_range.equal.iter())
                        .flat_map(|rhs_expr| Expr::difference(rhs_expr, fact_expr))
                })
                .max();

            // Positive offset indicates that RHS is greater than fact by that amount.
            if let Some(offset) = offset {
                let offset = match kind {
                    InequalityKind::Loose => offset,
                    // If the inequality is strict, we get
                    // one extra free increment: x < y
                    // implies x <= y - 1.
                    InequalityKind::Strict => offset + 1,
                };
                let new_upper_bounds = lhs_range
                    .min
                    .iter()
                    .chain(lhs_range.equal.iter())
                    .flat_map(|e| Expr::offset(e, -offset));
                let max = fact_range
                    .max
                    .iter()
                    .cloned()
                    .chain(new_upper_bounds)
                    .collect::<SmallVec<[Expr; 1]>>();
                fact.with_range(ValueRange {
                    min: fact_range.min.clone(),
                    equal: fact_range.equal.clone(),
                    max,
                })
            } else {
                fact.clone()
            }
        } else {
            fact.clone()
        };

        trace!("apply_inequality({fact:?}, {lhs:?}, {rhs:?}, {kind:?} -> {result:?}");
        result
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
    vcode: &mut VCode<B::MInst>,
    backend: &B,
) -> PccResult<()> {
    let ctx = FactContext::new(f, backend.triple().pointer_width().unwrap().bits().into());

    // Check that individual instructions are valid according to input
    // facts, and support the stated output facts.
    for block in 0..vcode.num_blocks() {
        let block = BlockIndex::new(block);
        let mut flow_state = B::FactFlowState::default();
        for inst in vcode.block_insns(block).iter() {
            // Check any output facts on this inst.
            if let Err(e) = backend.check_fact(&ctx, vcode, inst, &mut flow_state) {
                log::info!("Error checking instruction: {:?}", vcode[inst]);
                return Err(e);
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
