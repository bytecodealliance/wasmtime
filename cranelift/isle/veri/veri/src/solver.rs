use std::{cmp::Ordering, collections::HashSet, iter::zip};

use anyhow::{Context as _, Error, Result, bail, format_err};
use easy_smt::{Context, Response, SExpr, SExprData};
use num_bigint::BigUint;
use num_traits::Num as _;

use crate::{
    program::Program,
    type_inference::Assignment,
    types::{Const, Type, Width},
    veri::{Conditions, Expr, ExprId, Model},
};

use crate::encoded::cls::*;
use crate::encoded::clz::*;
use crate::encoded::popcnt::*;
use crate::encoded::rev::*;

#[derive(Debug, PartialEq, Eq)]
pub enum Applicability {
    Applicable,
    Inapplicable,
    Unknown,
}

impl std::fmt::Display for Applicability {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(match self {
            Applicability::Applicable => "applicable",
            Applicability::Inapplicable => "inapplicable",
            Applicability::Unknown => "unknown",
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Verification {
    Success,
    Failure(Model),
    Unknown,
}

impl std::fmt::Display for Verification {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(match self {
            Verification::Success => "success",
            Verification::Failure(_) => "failure",
            Verification::Unknown => "unknown",
        })
    }
}

enum RotationDirection {
    Left,
    Right,
}

static UNSPECIFIED_SORT: &str = "Unspecified";
static UNIT_SORT: &str = "Unit";

static ROUND_NEAREST_TIES_TO_EVEN: &str = "roundNearestTiesToEven";
static ROUND_TOWARD_ZERO: &str = "roundTowardZero";
static ROUND_TOWARD_POSITIVE: &str = "roundTowardPositive";
static ROUND_TOWARD_NEGATIVE: &str = "roundTowardNegative";
static ROUNDING_MODE: &str = ROUND_NEAREST_TIES_TO_EVEN;

/// SMT Dialect.
#[derive(Default, Debug, Clone, Copy)]
pub enum Dialect {
    /// SMT-LIB2 standard.
    #[default]
    SMTLIB2,
    /// SMT-LIB2 with Z3 extensions.
    Z3,
}

pub struct Solver<'a> {
    smt: Context,
    dialect: Dialect,
    prog: &'a Program,
    conditions: &'a Conditions,
    assignment: &'a Assignment,
    tmp_idx: usize,

    /// Widths for which the deterministic `fp.sqrt` uninterpreted function has
    /// already been declared (see [`Solver::fp_sqrt`]).
    sqrt_uf_widths: HashSet<usize>,
}

impl Drop for Solver<'_> {
    fn drop(&mut self) {
        // Attempt clean exit.
        let _ = self.exit();
    }
}

impl<'a> Solver<'a> {
    pub fn new(
        smt: Context,
        prog: &'a Program,
        conditions: &'a Conditions,
        assignment: &'a Assignment,
    ) -> Result<Self> {
        let mut solver = Self {
            smt,
            dialect: Dialect::default(),
            prog,
            conditions,
            assignment,
            tmp_idx: 0,
            sqrt_uf_widths: HashSet::new(),
        };
        solver.prelude()?;
        Ok(solver)
    }

    pub fn set_dialect(&mut self, dialect: Dialect) {
        self.dialect = dialect;
    }

    fn prelude(&mut self) -> Result<()> {
        // Set logic. Required for some SMT solvers.
        self.smt.set_logic("ALL")?;

        // Declare sorts for special-case types.
        self.smt.declare_sort(UNSPECIFIED_SORT, 0)?;
        self.smt.declare_sort(UNIT_SORT, 0)?;

        Ok(())
    }

    pub fn encode(&mut self) -> Result<()> {
        // Expressions
        for (i, expr) in self.conditions.exprs.iter().enumerate() {
            let x = ExprId(i);
            self.declare_expr(x)?;
            if !expr.is_variable() {
                self.assign_expr(x, expr)?;
            }
        }

        Ok(())
    }

    pub fn check_assumptions_feasibility(&mut self) -> Result<Applicability> {
        // Enter solver context frame.
        self.smt.push()?;

        // Assumptions
        let assumptions = self.all(&self.conditions.assumptions);
        self.smt.assert(assumptions)?;

        // Check
        let verdict = match self.check()? {
            Response::Sat => Applicability::Applicable,
            Response::Unsat => Applicability::Inapplicable,
            Response::Unknown => Applicability::Unknown,
        };

        // Leave solver context frame.
        self.smt.pop()?;

        Ok(verdict)
    }

    pub fn check_verification_condition(&mut self) -> Result<Verification> {
        // Enter solver context frame.
        self.smt.push()?;

        // Verification Condition
        self.verification_condition()?;

        // Check
        let verdict = match self.check()? {
            Response::Sat => Verification::Failure(self.model()?),
            Response::Unsat => Verification::Success,
            Response::Unknown => Verification::Unknown,
        };

        // Leave solver context frame.
        self.smt.pop()?;

        Ok(verdict)
    }

    fn check(&mut self) -> Result<Response> {
        // Send check-sat command. Prefer (check-sat-using default) for Z3.
        let cmd = self.smt.list(match self.dialect {
            Dialect::SMTLIB2 => vec![self.smt.atoms().check_sat],
            Dialect::Z3 => vec![self.smt.atom("check-sat-using"), self.smt.atom("default")],
        });

        self.smt.raw_send(cmd)?;

        // Parse response.
        let resp = self.smt.raw_recv()?;
        let atoms = self.smt.atoms();
        if resp == atoms.sat {
            Ok(Response::Sat)
        } else if resp == atoms.unsat {
            Ok(Response::Unsat)
        } else if resp == atoms.unknown {
            Ok(Response::Unknown)
        } else {
            bail!("bad solver check response: {}", self.smt.display(resp))
        }
    }

    pub fn exit(&mut self) -> Result<()> {
        // Send (exit) command.
        let exit = self.smt.list(vec![self.smt.atom("exit")]);
        self.smt.raw_send(exit)?;

        // Expect success response.
        let resp = self.smt.raw_recv()?;
        let atoms = self.smt.atoms();
        if resp != atoms.success {
            bail!("bad solver exit: {}", self.smt.display(resp))
        }
        Ok(())
    }

    pub fn model(&mut self) -> Result<Model> {
        let xs: Vec<_> = (0..self.conditions.exprs.len()).map(ExprId).collect();
        let expr_atoms = xs.iter().map(|x| self.expr_atom(*x)).collect();
        let values = self.smt.get_value(expr_atoms)?;
        let consts = values
            .iter()
            .map(|(_, v)| self.const_from_sexpr(*v))
            .collect::<Result<Vec<_>>>()?;
        Ok(zip(xs, consts).collect())
    }

    fn declare_expr(&mut self, x: ExprId) -> Result<()> {
        // Determine expression type value.
        let tv = self.assignment.try_assignment(x)?;

        // Map to corresponding SMT2 type.
        let sort = self.type_to_sort(&tv.ty())?;

        // Declare.
        self.smt.declare_const(self.expr_name(x), sort)?;

        Ok(())
    }

    fn type_to_sort(&self, ty: &Type) -> Result<SExpr> {
        match *ty {
            Type::BitVector(Width::Bits(width)) => {
                Ok(self.smt.bit_vec_sort(self.smt.numeral(width)))
            }
            Type::Int => Ok(self.smt.int_sort()),
            Type::Bool => Ok(self.smt.bool_sort()),
            Type::Unspecified => Ok(self.smt.atom(UNSPECIFIED_SORT)),
            Type::Unit => Ok(self.smt.atom(UNIT_SORT)),
            Type::Unknown | Type::BitVector(Width::Unknown) => {
                bail!("no smt2 sort for non-concrete type {ty}")
            }
        }
    }

    fn assign_expr(&mut self, x: ExprId, expr: &Expr) -> Result<()> {
        let lhs = self.smt.atom(self.expr_name(x));
        let rhs = self
            .expr_to_smt(expr)
            .map_err(|err| self.error(x, err.to_string()))?;
        Ok(self.smt.assert(
            self.smt
                .named(format!("expr{}", x.index()), self.smt.eq(lhs, rhs)),
        )?)
    }

    fn expr_to_smt(&mut self, expr: &Expr) -> Result<SExpr> {
        match *expr {
            Expr::Variable(_) => unreachable!("variables have no corresponding expression"),
            Expr::Const(ref c) => Ok(self.constant(c)),
            Expr::Not(x) => Ok(self.smt.not(self.expr_atom(x))),
            Expr::And(x, y) => Ok(self.smt.and(self.expr_atom(x), self.expr_atom(y))),
            Expr::Or(x, y) => Ok(self.smt.or(self.expr_atom(x), self.expr_atom(y))),
            Expr::Imp(x, y) => Ok(self.smt.imp(self.expr_atom(x), self.expr_atom(y))),
            Expr::Eq(x, y) => Ok(self.smt.eq(self.expr_atom(x), self.expr_atom(y))),
            Expr::Lt(x, y) => Ok(self.smt.lt(self.expr_atom(x), self.expr_atom(y))),
            Expr::Lte(x, y) => Ok(self.smt.lte(self.expr_atom(x), self.expr_atom(y))),
            Expr::BVUgt(x, y) => Ok(self.smt.bvugt(self.expr_atom(x), self.expr_atom(y))),
            Expr::BVUge(x, y) => Ok(self.smt.bvuge(self.expr_atom(x), self.expr_atom(y))),
            Expr::BVUlt(x, y) => Ok(self.smt.bvult(self.expr_atom(x), self.expr_atom(y))),
            Expr::BVUle(x, y) => Ok(self.smt.bvule(self.expr_atom(x), self.expr_atom(y))),
            Expr::BVSgt(x, y) => Ok(self.smt.bvsgt(self.expr_atom(x), self.expr_atom(y))),
            Expr::BVSge(x, y) => Ok(self.smt.bvsge(self.expr_atom(x), self.expr_atom(y))),
            Expr::BVSlt(x, y) => Ok(self.smt.bvslt(self.expr_atom(x), self.expr_atom(y))),
            Expr::BVSle(x, y) => Ok(self.smt.bvsle(self.expr_atom(x), self.expr_atom(y))),
            Expr::BVSaddo(x, y) => Ok(self.smt.list(vec![
                self.smt.atom("bvsaddo"),
                self.expr_atom(x),
                self.expr_atom(y),
            ])),
            Expr::BVNot(x) => Ok(self.smt.bvnot(self.expr_atom(x))),
            Expr::Cls(x) => {
                let width = self
                    .assignment
                    .try_bit_vector_width(x)
                    .context("cls semantics require known width")?;
                let xe = self.expr_atom(x);
                let id = x.index();
                match width {
                    8 => Ok(cls8(&mut self.smt, xe, id)),
                    16 => Ok(cls16(&mut self.smt, xe, id)),
                    32 => Ok(cls32(&mut self.smt, xe, id)),
                    64 => Ok(cls64(&mut self.smt, xe, id)),
                    _ => unimplemented!("unexpected CLS width"),
                }
            }
            Expr::Clz(x) => {
                let width = self
                    .assignment
                    .try_bit_vector_width(x)
                    .context("clz semantics require known width")?;
                let xe = self.expr_atom(x);
                let id: usize = x.index();
                match width {
                    1 => Ok(clz1(&mut self.smt, xe, id)),
                    8 => Ok(clz8(&mut self.smt, xe, id)),
                    16 => Ok(clz16(&mut self.smt, xe, id)),
                    32 => Ok(clz32(&mut self.smt, xe, id)),
                    64 => Ok(clz64(&mut self.smt, xe, id)),
                    _ => unimplemented!("unexpected CLZ width"),
                }
            }
            Expr::Rev(x) => {
                let width = self
                    .assignment
                    .try_bit_vector_width(x)
                    .context("cls semantics require known width")?;
                let xe = self.expr_atom(x);
                let id: usize = x.index();
                match width {
                    1 => Ok(rev1(&mut self.smt, xe, id)),
                    8 => Ok(rev8(&mut self.smt, xe, id)),
                    16 => Ok(rev16(&mut self.smt, xe, id)),
                    32 => Ok(rev32(&mut self.smt, xe, id)),
                    64 => Ok(rev64(&mut self.smt, xe, id)),
                    _ => unimplemented!("unexpected CLS width"),
                }
            }
            Expr::Popcnt(x) => {
                let width = self
                    .assignment
                    .try_bit_vector_width(x)
                    .context("popcnt semantics require known width")?;
                let xe = self.expr_atom(x);
                let id = x.index();
                match width {
                    8 | 16 | 32 | 64 => Ok(popcnt(&mut self.smt, width, xe, id)),
                    _ => unimplemented!("unexpected Popcnt width"),
                }
            }

            Expr::Add(x, y) => Ok(self.smt.plus(self.expr_atom(x), self.expr_atom(y))),
            Expr::Sub(x, y) => Ok(self.smt.sub(self.expr_atom(x), self.expr_atom(y))),
            Expr::Mul(x, y) => Ok(self.smt.times(self.expr_atom(x), self.expr_atom(y))),

            Expr::BVNeg(x) => Ok(self.smt.bvneg(self.expr_atom(x))),
            Expr::BVAdd(x, y) => Ok(self.smt.bvadd(self.expr_atom(x), self.expr_atom(y))),
            Expr::BVOr(x, y) => Ok(self.smt.bvor(self.expr_atom(x), self.expr_atom(y))),
            Expr::BVXor(x, y) => Ok(self.smt.bvxor(self.expr_atom(x), self.expr_atom(y))),
            Expr::BVSub(x, y) => Ok(self.smt.bvsub(self.expr_atom(x), self.expr_atom(y))),
            Expr::BVMul(x, y) => Ok(self.smt.bvmul(self.expr_atom(x), self.expr_atom(y))),
            Expr::BVSDiv(x, y) => Ok(self.smt.list(vec![
                self.smt.atom("bvsdiv"),
                self.expr_atom(x),
                self.expr_atom(y),
            ])),
            Expr::BVUDiv(x, y) => Ok(self.smt.bvudiv(self.expr_atom(x), self.expr_atom(y))),
            Expr::BVSRem(x, y) => Ok(self.smt.bvsrem(self.expr_atom(x), self.expr_atom(y))),
            Expr::BVURem(x, y) => Ok(self.smt.bvurem(self.expr_atom(x), self.expr_atom(y))),
            Expr::BVAnd(x, y) => Ok(self.smt.bvand(self.expr_atom(x), self.expr_atom(y))),
            Expr::BVShl(x, y) => Ok(self.smt.bvshl(self.expr_atom(x), self.expr_atom(y))),
            Expr::BVLShr(x, y) => Ok(self.smt.bvlshr(self.expr_atom(x), self.expr_atom(y))),
            Expr::BVAShr(x, y) => Ok(self.smt.bvashr(self.expr_atom(x), self.expr_atom(y))),
            Expr::BVRotl(x, y) => {
                let width = self
                    .assignment
                    .try_bit_vector_width(x)
                    .context("target of rotl expression should be a bit-vector of known width")?;
                let xs = self.expr_atom(x);
                let ys = self.expr_atom(y);
                Ok(self.encode_rotate(RotationDirection::Left, xs, ys, width))
            }
            Expr::BVRotr(x, y) => {
                let width = self
                    .assignment
                    .try_bit_vector_width(x)
                    .context("target of rotr expression should be a bit-vector of known width")?;
                let xs = self.expr_atom(x);
                let ys = self.expr_atom(y);
                Ok(self.encode_rotate(RotationDirection::Right, xs, ys, width))
            }
            Expr::Conditional(c, t, e) => {
                Ok(self
                    .smt
                    .ite(self.expr_atom(c), self.expr_atom(t), self.expr_atom(e)))
            }
            Expr::BVZeroExt(w, x) => self.bv_zero_ext(w, x),
            Expr::BVSignExt(w, x) => self.bv_sign_ext(w, x),
            Expr::BVConvTo(w, x) => self.bv_conv_to(w, x),
            Expr::BVExtract(h, l, x) => Ok(self.extract(h, l, self.expr_atom(x))),
            Expr::BVConcat(x, y) => Ok(self.smt.concat(self.expr_atom(x), self.expr_atom(y))),
            Expr::Int2BV(w, x) => self.int_to_bv(w, x),
            Expr::BV2Nat(x) => Ok(self
                .smt
                .list(vec![self.smt.atom("bv2nat"), self.expr_atom(x)])),
            Expr::ToFP(w, x) => self.fp_from_expr(w, x, true),
            Expr::ToFPUnsigned(w, x) => self.fp_from_expr(w, x, false),
            Expr::ToFPFromFP(w, x) => self.fp_from_fp(w, x),
            Expr::FPToUBV(w, x) => self.fp_to_bv(w, x, false),
            Expr::FPToSBV(w, x) => self.fp_to_bv(w, x, true),
            Expr::WidthOf(x) => self.width_of(x),
            Expr::FPPositiveInfinity(x) => Ok(self.fp_value("+oo", x)?),
            Expr::FPNegativeInfinity(x) => Ok(self.fp_value("-oo", x)?),
            Expr::FPPositiveZero(x) => Ok(self.fp_value("+zero", x)?),
            Expr::FPNegativeZero(x) => Ok(self.fp_value("-zero", x)?),
            Expr::FPNaN(x) => Ok(self.fp_value("NaN", x)?),
            Expr::FPEq(x, y) => Ok(self.fp_test("fp.eq", x, y)?),
            Expr::FPNe(x, y) => {
                let test_eq = self.fp_test("fp.eq", x, y)?;
                Ok(self.smt.not(test_eq))
            }
            Expr::FPLt(x, y) => Ok(self.fp_test("fp.lt", x, y)?),
            Expr::FPGt(x, y) => Ok(self.fp_test("fp.gt", x, y)?),
            Expr::FPLe(x, y) => Ok(self.fp_test("fp.leq", x, y)?),
            Expr::FPGe(x, y) => Ok(self.fp_test("fp.geq", x, y)?),
            Expr::FPAdd(x, y) => Ok(self.fp_rounding_binary("fp.add", x, y)?),
            Expr::FPSub(x, y) => Ok(self.fp_rounding_binary("fp.sub", x, y)?),
            Expr::FPMul(x, y) => Ok(self.fp_rounding_binary("fp.mul", x, y)?),
            Expr::FPDiv(x, y) => Ok(self.fp_rounding_binary("fp.div", x, y)?),
            Expr::FPMin(x, y) => Ok(self.fp_binary("fp.min", x, y)?),
            Expr::FPMax(x, y) => Ok(self.fp_binary("fp.max", x, y)?),
            Expr::FPNeg(x) => Ok(self.fp_unary("fp.neg", x)?),
            Expr::FPCeil(x) => {
                Ok(self.fp_rounding_unary("fp.roundToIntegral", ROUND_TOWARD_POSITIVE, x)?)
            }
            Expr::FPFloor(x) => {
                Ok(self.fp_rounding_unary("fp.roundToIntegral", ROUND_TOWARD_NEGATIVE, x)?)
            }
            Expr::FPSqrt(x) => Ok(self.fp_sqrt(x)?),
            Expr::FPTrunc(x) => {
                Ok(self.fp_rounding_unary("fp.roundToIntegral", ROUND_TOWARD_ZERO, x)?)
            }
            Expr::FPNearest(x) => {
                Ok(self.fp_rounding_unary("fp.roundToIntegral", ROUND_NEAREST_TIES_TO_EVEN, x)?)
            }
            Expr::FPIsZero(x) => Ok(self.fp_unary_predicate("fp.isZero", x)?),
            Expr::FPIsInfinite(x) => Ok(self.fp_unary_predicate("fp.isInfinite", x)?),
            Expr::FPIsNaN(x) => Ok(self.fp_unary_predicate("fp.isNaN", x)?),
            Expr::FPIsNegative(x) => Ok(self.fp_unary_predicate("fp.isNegative", x)?),
            Expr::FPIsPositive(x) => Ok(self.fp_unary_predicate("fp.isPositive", x)?),
        }
    }

    fn constant(&self, constant: &Const) -> SExpr {
        match *constant {
            Const::Bool(true) => self.smt.true_(),
            Const::Bool(false) => self.smt.false_(),
            Const::Int(v) => self.smt.numeral(v),
            Const::BitVector(w, ref v) => self.smt.atom(format!("#b{v:0>w$b}")),
            Const::Unspecified => unimplemented!("constant of unspecified type"),
        }
    }

    fn bv_zero_ext(&self, w: ExprId, x: ExprId) -> Result<SExpr> {
        // TODO(mbm): dedupe logic with bv_sign_ext and bv_conv_to?

        // Destination width expression should have known integer value.
        let dst: usize = self
            .assignment
            .try_int_value(w)
            .context("destination width of zero_ext expression should have known integer value")?
            .try_into()
            .expect("width should be representable as usize");

        // Expression type should be a bit-vector of known width.
        let src = self
            .assignment
            .try_bit_vector_width(x)
            .context("source of zero_ext expression should be a bit-vector of known width")?;

        // Build zero_extend expression.
        let padding = dst
            .checked_sub(src)
            .ok_or(format_err!("cannot zero extend to smaller width"))?;
        Ok(self.zero_extend(padding, self.expr_atom(x)))
    }

    fn bv_sign_ext(&self, w: ExprId, x: ExprId) -> Result<SExpr> {
        // TODO(mbm): dedupe logic with bv_conv_to?

        // Destination width expression should have known integer value.
        let dst: usize = self
            .assignment
            .try_int_value(w)
            .context("destination width of sign_ext expression should have known integer value")?
            .try_into()
            .expect("width should be representable as usize");

        // Expression type should be a bit-vector of known width.
        let src = self
            .assignment
            .try_bit_vector_width(x)
            .context("source of sign_ext expression should be a bit-vector of known width")?;

        // Build sign_extend expression.
        let padding = dst
            .checked_sub(src)
            .ok_or(format_err!("cannot sign extend to smaller width"))?;
        Ok(self.sign_extend(padding, self.expr_atom(x)))
    }

    fn bv_conv_to(&mut self, w: ExprId, x: ExprId) -> Result<SExpr> {
        // Destination width expression should have known integer value.
        let dst: usize = self
            .assignment
            .try_int_value(w)
            .context("destination width of conv_to expression should have known integer value")?
            .try_into()
            .expect("width should be representable as usize");

        // Expression type should be a bit-vector of known width.
        let src = self
            .assignment
            .try_bit_vector_width(x)
            .context("source of conv_to expression should be a bit-vector of known width")?;

        // Handle depending on source and destination widths.
        match dst.cmp(&src) {
            Ordering::Greater => {
                let padding = self.declare_bit_vec("conv_to_padding", dst - src)?;
                Ok(self.smt.concat(padding, self.expr_atom(x)))
            }
            Ordering::Less => {
                let high_bit = dst.checked_sub(1).unwrap();
                Ok(self.extract(high_bit, 0, self.expr_atom(x)))
            }
            Ordering::Equal => Ok(self.expr_atom(x)),
        }
    }

    fn int_to_bv(&self, w: ExprId, x: ExprId) -> Result<SExpr> {
        // Destination width expression should have known integer value.
        let width: usize = self
            .assignment
            .try_int_value(w)
            .context("destination width of int2bv expression should have known integer value")?
            .try_into()
            .expect("width should be representable as usize");

        // Build int2bv expression.
        Ok(self.int2bv(width, self.expr_atom(x)))
    }

    fn width_of(&self, x: ExprId) -> Result<SExpr> {
        // Expression type should be a bit-vector of known width.
        let width = self
            .assignment
            .try_bit_vector_width(x)
            .context("target of width_of expression should be a bit-vector of known width")?;

        // Substitute known constant width.
        Ok(self.smt.numeral(width))
    }

    fn verification_condition(&mut self) -> Result<()> {
        // (not (<assumptions> => <assertions>))
        let assumptions = self.all(&self.conditions.assumptions);
        let assertions = self.all(&self.conditions.assertions);
        let vc = self.smt.imp(assumptions, assertions);
        self.smt.assert(self.smt.not(vc))?;
        Ok(())
    }

    /// Zero-extend an SMT bit vector to a wider bit vector by adding `padding`
    /// zeroes to the front.
    fn zero_extend(&self, padding: usize, v: SExpr) -> SExpr {
        if padding == 0 {
            return v;
        }
        self.smt.list(vec![
            self.smt.list(vec![
                self.smt.atoms().und,
                self.smt.atom("zero_extend"),
                self.smt.numeral(padding),
            ]),
            v,
        ])
    }

    /// Sign-extend an SMT bit vector to a wider bit vector.
    fn sign_extend(&self, padding: usize, v: SExpr) -> SExpr {
        if padding == 0 {
            return v;
        }
        self.smt.list(vec![
            self.smt.list(vec![
                self.smt.atoms().und,
                self.smt.atom("sign_extend"),
                self.smt.numeral(padding),
            ]),
            v,
        ])
    }

    fn extract(&self, high_bit: usize, low_bit: usize, v: SExpr) -> SExpr {
        assert!(low_bit <= high_bit);
        self.smt
            .extract(high_bit.try_into().unwrap(), low_bit.try_into().unwrap(), v)
    }

    /// Convert an SMT integer to a bit vector of a given width.
    fn int2bv(&self, width: usize, value: SExpr) -> SExpr {
        self.smt.list(vec![
            self.smt.list(vec![
                self.smt.atoms().und,
                self.smt.atom("int2bv"),
                self.smt.numeral(width),
            ]),
            value,
        ])
    }

    /// Floating point special values.
    fn fp_value(&mut self, op: &str, w: ExprId) -> Result<SExpr> {
        let width = self
            .assignment
            .try_int_value(w)
            .context("floating point constant width should have known integer value")?
            .try_into()?;
        let (eb, sb) = Self::fp_exponent_significand_bits(width)?;
        let result_fp = self.smt.list(vec![
            self.smt.atoms().und,
            self.smt.atom(op),
            self.smt.numeral(eb),
            self.smt.numeral(sb),
        ]);

        // Return bit-vector that's equal to the expression as a floating point.
        let result = self.declare_bit_vec(op, width)?;
        let result_as_fp = self.to_fp(result, width)?;
        self.smt.assert(self.smt.eq(result_as_fp, result_fp))?;

        Ok(result)
    }

    /// Floating point unary operand without rounding.
    fn fp_unary(&mut self, op: &str, x: ExprId) -> Result<SExpr> {
        // Convert to floating point.
        let width = self
            .assignment
            .try_bit_vector_width(x)
            .context("floating point expression must be a bit-vector of known width")?;

        let x = self.to_fp(self.expr_atom(x), width)?;

        // Unary expression.
        let result_fp = self.smt.list(vec![self.smt.atom(op), x]);

        // Return bit-vector that's equal to the expression as a floating point.
        let result = self.declare_bit_vec(op, width)?;
        let result_as_fp = self.to_fp(result, width)?;
        self.smt.assert(self.smt.eq(result_as_fp, result_fp))?;

        Ok(result)
    }

    /// Floating point test operation without rounding, to boolean.
    fn fp_test(&mut self, op: &str, x: ExprId, y: ExprId) -> Result<SExpr> {
        // Convert to floating point.
        let width = self
            .assignment
            .try_bit_vector_width(x)
            .context("floating point expression must be a bit-vector of known width")?;

        let x = self.to_fp(self.expr_atom(x), width)?;
        let y = self.to_fp(self.expr_atom(y), width)?;

        // Binary result, no conversion needed after test
        Ok(self.smt.list(vec![self.smt.atom(op), x, y]))
    }

    /// Floating point binary operand without rounding.
    fn fp_binary(&mut self, op: &str, x: ExprId, y: ExprId) -> Result<SExpr> {
        // Convert to floating point.
        let width = self
            .assignment
            .try_bit_vector_width(x)
            .context("floating point expression must be a bit-vector of known width")?;

        let x = self.to_fp(self.expr_atom(x), width)?;
        let y = self.to_fp(self.expr_atom(y), width)?;

        // Binary expression.
        let result_fp = self.smt.list(vec![self.smt.atom(op), x, y]);

        // Return bit-vector that's equal to the expression as a floating point.
        let result = self.declare_bit_vec(op, width)?;
        let result_as_fp = self.to_fp(result, width)?;
        self.smt.assert(self.smt.eq(result_as_fp, result_fp))?;

        Ok(result)
    }

    /// Floating point unary operand with rounding.
    fn fp_rounding_unary(&mut self, op: &str, rounding_mode: &str, x: ExprId) -> Result<SExpr> {
        // Convert to floating point.
        let width = self
            .assignment
            .try_bit_vector_width(x)
            .context("floating point expression must be a bit-vector of known width")?;

        let x = self.to_fp(self.expr_atom(x), width)?;

        // Unary expression.
        let result_fp = self
            .smt
            .list(vec![self.smt.atom(op), self.smt.atom(rounding_mode), x]);

        // Return bit-vector that's equal to the expression as a floating point.
        let result = self.declare_bit_vec(op, width)?;
        let result_as_fp = self.to_fp(result, width)?;
        self.smt.assert(self.smt.eq(result_as_fp, result_fp))?;

        Ok(result)
    }

    /// Floating point binary operand with rounding.
    fn fp_rounding_binary(&mut self, op: &str, x: ExprId, y: ExprId) -> Result<SExpr> {
        // Convert to floating point.
        let width = self
            .assignment
            .try_bit_vector_width(x)
            .context("floating point expression must be a bit-vector of known width")?;

        let x = self.to_fp(self.expr_atom(x), width)?;
        let y = self.to_fp(self.expr_atom(y), width)?;

        // Binary expression.
        let result_fp = self
            .smt
            .list(vec![self.smt.atom(op), self.smt.atom(ROUNDING_MODE), x, y]);

        // Return bit-vector that's equal to the expression as a floating point.
        let result = self.declare_bit_vec(op, width)?;
        let result_as_fp = self.to_fp(result, width)?;
        self.smt.assert(self.smt.eq(result_as_fp, result_fp))?;

        Ok(result)
    }

    /// Floating point square root, modeled as a deterministic uninterpreted
    /// function over the input bits rather than the bit-exact `fp.sqrt`.
    ///
    /// `fp.sqrt` is the one floating-point operation that current SMT solvers
    /// (both Z3 and CVC5) cannot decide in a reasonable time in this encoding:
    /// the bit-vector/`to_fp` round-trip around `fp.sqrt` makes even the
    /// applicability precheck time out. The verification only relies on sqrt
    /// being a *deterministic* function of its input -- both the instruction
    /// spec and the lowering apply the same sqrt to the same value, and both
    /// handle NaN/zero/infinity/negative inputs explicitly before ever
    /// reaching `fp.sqrt`, so its bit-exact value is never relied upon. Modeling
    /// it as an uninterpreted function is therefore sound for proving
    /// lowering/spec equivalence (congruence forces equal inputs to equal
    /// outputs, and unequal inputs remain free to differ) while keeping the
    /// queries decidable. This mirrors the custom encodings used for other
    /// solver-hostile operations (`cls`, `clz`, `popcnt`, `rev`).
    fn fp_sqrt(&mut self, x: ExprId) -> Result<SExpr> {
        let width = self
            .assignment
            .try_bit_vector_width(x)
            .context("floating point expression must be a bit-vector of known width")?;

        // Declare the per-width uninterpreted sqrt function once, then share the
        // same symbol across every sqrt occurrence so that equal inputs are
        // forced (by congruence) to produce equal outputs.
        let func = format!("fp.sqrt_uf_{width}");
        if self.sqrt_uf_widths.insert(width) {
            let bv_sort = self.smt.bit_vec_sort(self.smt.numeral(width));
            self.smt.declare_fun(&func, vec![bv_sort], bv_sort)?;
        }

        Ok(self.smt.list(vec![self.smt.atom(func), self.expr_atom(x)]))
    }

    /// Floating point unary predicate.
    fn fp_unary_predicate(&mut self, op: &str, x: ExprId) -> Result<SExpr> {
        // Convert operand to floating point.
        let width = self
            .assignment
            .try_bit_vector_width(x)
            .context("floating point expression must be a bit-vector of known width")?;

        let x = self.to_fp(self.expr_atom(x), width)?;

        // Emit expression.
        Ok(self.smt.list(vec![self.smt.atom(op), x]))
    }

    /// Represent an expression in SMT-LIB floating-point type.
    fn to_fp(&self, x: SExpr, width: usize) -> Result<SExpr> {
        let (eb, sb) = Self::fp_exponent_significand_bits(width)?;
        Ok(self.smt.list(vec![
            self.smt.list(vec![
                self.smt.atoms().und,
                self.smt.atom("to_fp"),
                self.smt.numeral(eb),
                self.smt.numeral(sb),
            ]),
            x,
        ]))
    }

    fn fp_from_expr(&mut self, w: ExprId, xid: ExprId, signed: bool) -> Result<SExpr> {
        // Destination width expression should have known integer value.
        let width: usize = self
            .assignment
            .try_int_value(w)
            .context("destination width of to_fp expression should have known integer value")?
            .try_into()
            .expect("width should be representable as usize");

        let x = self.expr_atom(xid);
        let (eb, sb) = Self::fp_exponent_significand_bits(width)?;
        let fp = self.smt.list(vec![
            self.smt.list(vec![
                self.smt.atoms().und,
                self.smt
                    .atom(if signed { "to_fp" } else { "to_fp_unsigned" }),
                self.smt.numeral(eb),
                self.smt.numeral(sb),
            ]),
            self.smt.atom(ROUNDING_MODE),
            x,
        ]);
        // Return bit-vector that's equal to the expression as a floating point.
        let result = self.declare_bit_vec("conv", width)?;
        let result_as_fp = self.to_fp(result, width)?;
        self.smt.assert(self.smt.eq(result_as_fp, fp))?;

        Ok(result)
    }

    fn fp_to_bv(&mut self, w: ExprId, x: ExprId, signed: bool) -> Result<SExpr> {
        // Destination width expression should have known integer value.
        let width: usize = self
            .assignment
            .try_int_value(w)
            .context("destination width of fp_to_bv expression should have known integer value")?
            .try_into()
            .expect("width should be representable as usize");

        let x = self.to_fp(self.expr_atom(x), width)?;

        let fp: SExpr = self.smt.list(vec![
            self.smt.list(vec![
                self.smt.atoms().und,
                self.smt
                    .atom(if signed { "fp.to_sbv" } else { "fp.to_ubv" }),
                self.smt.numeral(width),
            ]),
            self.smt.atom(ROUNDING_MODE),
            x,
        ]);

        Ok(fp)
    }

    fn fp_from_fp(&mut self, w: ExprId, xid: ExprId) -> Result<SExpr> {
        // Destination width expression should have known integer value.
        let new_width: usize = self
            .assignment
            .try_int_value(w)
            .context(
                "destination width of to_fp_from_fp expression should have known integer value",
            )?
            .try_into()
            .expect("width should be representable as usize");

        // Convert operand to floating point.
        let width = self
            .assignment
            .try_bit_vector_width(xid)
            .context("floating point expression must be a bit-vector of known width")?;
        let x = self.to_fp(self.expr_atom(xid), width)?;

        let (eb, sb) = Self::fp_exponent_significand_bits(new_width)?;
        let fp = self.smt.list(vec![
            self.smt.list(vec![
                self.smt.atoms().und,
                self.smt.atom("to_fp"),
                self.smt.numeral(eb),
                self.smt.numeral(sb),
            ]),
            self.smt.atom(ROUNDING_MODE),
            x,
        ]);
        // Return bit-vector that's equal to the expression as a floating point.
        let result = self.declare_bit_vec("conv", new_width)?;
        let result_as_fp = self.to_fp(result, new_width)?;
        self.smt.assert(self.smt.eq(result_as_fp, fp))?;

        Ok(result)
    }

    fn fp_exponent_significand_bits(width: usize) -> Result<(usize, usize)> {
        Ok(match width {
            32 => (8, 24),
            64 => (11, 53),
            _ => bail!("unsupported floating-point width"),
        })
    }

    /// Parse a constant SMT expression.
    fn const_from_sexpr(&self, sexpr: SExpr) -> Result<Const> {
        match self.smt.get(sexpr) {
            SExprData::Atom(a) => Self::const_from_literal(a),
            SExprData::List(exprs) => self.const_from_qualified_abstract_value(exprs),
            SExprData::String(s) => bail!("unsupported smt const: {s}"),
        }
    }

    /// Parse a constant from an SMT literal.
    fn const_from_literal(atom: &str) -> Result<Const> {
        if atom == "true" {
            Ok(Const::Bool(true))
        } else if atom == "false" {
            Ok(Const::Bool(false))
        } else if let Some(x) = atom.strip_prefix("#x") {
            Ok(Const::BitVector(
                x.len() * 4,
                BigUint::from_str_radix(x, 16)?,
            ))
        } else if let Some(x) = atom.strip_prefix("#b") {
            Ok(Const::BitVector(x.len(), BigUint::from_str_radix(x, 2)?))
        } else if atom.starts_with(|c: char| c.is_ascii_digit()) {
            Ok(Const::Int(atom.parse()?))
        } else {
            bail!("unsupported smt literal: {atom}")
        }
    }

    /// Parse a constant value of a declared sort from an SMT qualified abstract value.
    fn const_from_qualified_abstract_value(&self, exprs: &[SExpr]) -> Result<Const> {
        // This logic is specific to CVC5's representation of declared sort
        // abstract values. Z3 uses a different format.  This function is
        // therefore careful to check for the exact format it expects from CVC5.

        // Expect a list of atoms.
        let atoms = exprs
            .iter()
            .map(|e| match self.smt.get(*e) {
                SExprData::Atom(a) => Ok(a),
                _ => bail!("expected atom in qualified identifier"),
            })
            .collect::<Result<Vec<_>>>()?;

        // Expect the list to be of the form (as @<abstract_value> <sort>).
        let ["as", value, sort] = atoms.as_slice() else {
            bail!("unsupported qualified identifier: {atoms:?}");
        };

        // Expect an abstract value.
        if !value.starts_with('@') {
            bail!("expected qualified identifier constant to have abstract value");
        }

        // Construct constant based on the sort.
        if sort == &UNSPECIFIED_SORT {
            Ok(Const::Unspecified)
        } else if sort == &UNIT_SORT {
            todo!("unit sort")
        } else {
            bail!("unknown sort: '{sort}'");
        }
    }

    fn all(&self, xs: &[ExprId]) -> SExpr {
        self.smt.and_many(xs.iter().map(|x| self.expr_atom(*x)))
    }

    fn expr_atom(&self, x: ExprId) -> SExpr {
        self.smt.atom(self.expr_name(x))
    }

    fn expr_name(&self, x: ExprId) -> String {
        let expr = &self.conditions.exprs[x.index()];
        if let Expr::Variable(v) = expr {
            format!(
                "{}_{}",
                self.conditions.variables[v.index()].name,
                x.index()
            )
        } else {
            format!("e{}", x.index())
        }
    }

    fn declare_bit_vec(&mut self, name: &str, n: usize) -> Result<SExpr> {
        let name = format!("${name}{}", self.tmp_idx);
        self.tmp_idx += 1;
        let sort = self.smt.bit_vec_sort(self.smt.numeral(n));
        self.smt.declare_const(&name, sort)?;
        Ok(self.smt.atom(name))
    }

    fn encode_rotate(
        &self,
        op: RotationDirection,
        source: SExpr,
        amount: SExpr,
        width: usize,
    ) -> SExpr {
        // SMT bitvector rotate_left requires that the rotate amount be
        // statically specified. Instead, to use a dynamic amount, desugar
        // to shifts and bit arithmetic.
        let width_as_bv = self.smt.binary(width, width);
        let wrapped_amount = self.smt.bvurem(amount, width_as_bv);
        let wrapped_delta = self.smt.bvsub(width_as_bv, wrapped_amount);
        match op {
            RotationDirection::Left => self.smt.bvor(
                self.smt.bvshl(source, wrapped_amount),
                self.smt.bvlshr(source, wrapped_delta),
            ),
            RotationDirection::Right => self.smt.bvor(
                self.smt.bvshl(source, wrapped_delta),
                self.smt.bvlshr(source, wrapped_amount),
            ),
        }
    }

    fn error(&self, x: ExprId, msg: impl Into<String>) -> Error {
        self.conditions.error_at_expr(self.prog, x, msg)
    }
}
