use anyhow::{Ok, Result, bail, format_err};
use cranelift_isle::{
    ast::{self, AttrKind, AttrTarget, Def, Ident, Model, ModelType, Modifies, SpecOp},
    lexer::Pos,
    sema::{ReturnKind, RuleId, Sym, Term, TermEnv, TermId, TypeEnv, TypeId},
};
use std::{
    collections::{HashMap, HashSet, hash_map::Entry},
    fmt::Debug,
};

use crate::types::{Compound, Const};

/// Positioned attaches positional information to a wrapped object.
#[derive(Clone)]
pub struct Positioned<X> {
    pub pos: Pos,
    pub x: X,
}

impl<X> Positioned<X> {
    fn new(pos: Pos, x: X) -> Box<Self> {
        Box::new(Self { pos, x })
    }
}

impl<X: Debug> Debug for Positioned<X> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Positioned")
            .field(&self.pos)
            .field(&self.x)
            .finish()
    }
}

pub type Expr = Box<Positioned<ExprKind>>;

/// Spec expression.
#[derive(Debug, Clone)]
pub enum ExprKind {
    // TODO(mbm): plumb positional information through spec expressions

    // Terminal nodes
    Var(Ident),
    Const(Const),
    As(Expr, Compound),
    Constructor(Constructor),
    Field(Ident, Expr),
    Discriminator(Ident, Expr),

    // Get the width of a bitvector
    WidthOf(Expr),

    // Boolean operations
    Not(Expr),
    And(Vec<Expr>),
    Or(Vec<Expr>),
    Imp(Expr, Expr),
    Eq(Expr, Expr),
    Lt(Expr, Expr),
    Lte(Expr, Expr),
    Gt(Expr, Expr),
    Gte(Expr, Expr),

    BVSgt(Expr, Expr),
    BVSge(Expr, Expr),
    BVSlt(Expr, Expr),
    BVSle(Expr, Expr),
    BVUgt(Expr, Expr),
    BVUge(Expr, Expr),
    BVUlt(Expr, Expr),
    BVUle(Expr, Expr),

    BVSaddo(Expr, Expr),

    // Integer arithmetic
    Add(Expr, Expr),
    Sub(Expr, Expr),
    Mul(Expr, Expr),

    // Bitvector operations

    // Unary operators
    BVNeg(Expr),
    BVNot(Expr),
    Cls(Expr),
    Clz(Expr),
    Rev(Expr),
    Popcnt(Expr),

    // Binary operators
    BVAdd(Expr, Expr),
    BVSub(Expr, Expr),
    BVMul(Expr, Expr),
    BVUDiv(Expr, Expr),
    BVSDiv(Expr, Expr),
    BVURem(Expr, Expr),
    BVSRem(Expr, Expr),
    BVAnd(Expr, Expr),
    BVOr(Expr, Expr),
    BVXor(Expr, Expr),
    BVRotl(Expr, Expr),
    BVRotr(Expr, Expr),
    BVShl(Expr, Expr),
    BVLShr(Expr, Expr),
    BVAShr(Expr, Expr),

    // Conversions
    BVZeroExt(Expr, Expr),
    BVSignExt(Expr, Expr),
    // Conversion to wider/narrower bits, without an explicit extend.
    BVConvTo(Expr, Expr),
    ToFP(Expr, Expr),
    ToFPUnsigned(Expr, Expr),
    ToFPFromFP(Expr, Expr),
    FPToUBV(Expr, Expr),
    FPToSBV(Expr, Expr),

    // Extract specified bits
    BVExtract(usize, usize, Expr),

    // Concatenate bitvectors.
    BVConcat(Vec<Expr>),
    BVReplicate(Expr, usize),

    // Convert between integers and bitvector.
    Int2BV(Expr, Expr),
    BV2Nat(Expr),

    // Floating point.
    FPPositiveInfinity(Expr),
    FPNegativeInfinity(Expr),
    FPPositiveZero(Expr),
    FPNegativeZero(Expr),
    FPNaN(Expr),
    FPEq(Expr, Expr),
    FPNe(Expr, Expr),
    FPLt(Expr, Expr),
    FPGt(Expr, Expr),
    FPLe(Expr, Expr),
    FPGe(Expr, Expr),
    FPAdd(Expr, Expr),
    FPSub(Expr, Expr),
    FPMul(Expr, Expr),
    FPDiv(Expr, Expr),
    FPMin(Expr, Expr),
    FPMax(Expr, Expr),
    FPNeg(Expr),
    FPCeil(Expr),
    FPFloor(Expr),
    FPSqrt(Expr),
    FPTrunc(Expr),
    FPNearest(Expr),
    FPIsZero(Expr),
    FPIsInfinite(Expr),
    FPIsNaN(Expr),
    FPIsNegative(Expr),
    FPIsPositive(Expr),

    // Conditional if-then-else
    Conditional(Expr, Expr, Expr),
    // Switch
    Switch(Expr, Vec<(Expr, Expr)>),
    // Match
    Match(Expr, Vec<Arm>),
    // Let bindings
    Let(Vec<(Ident, Expr)>, Expr),
    // With scope.
    With(Vec<Ident>, Expr),
    // Macro definition.
    Macro(Vec<Ident>, Expr),
    // Macro expansion.
    Expand(Ident, Vec<Expr>),
}

macro_rules! unary_expr {
    ($expr:path, $args:ident, $pos:ident) => {{
        // TODO(mbm): return error instead of assert
        assert_eq!(
            $args.len(),
            1,
            "Unexpected number of args for unary operator at {:?}",
            $pos
        );
        $expr(expr_from_ast(&$args[0]))
    }};
}

macro_rules! binary_expr {
    ($expr:path, $args:ident, $pos:ident) => {{
        // TODO(mbm): return error instead of assert
        assert_eq!(
            $args.len(),
            2,
            "Unexpected number of args for binary operator at {:?}",
            $pos
        );
        $expr(expr_from_ast(&$args[0]), expr_from_ast(&$args[1]))
    }};
}

macro_rules! ternary_expr {
    ($expr:path, $args:ident, $pos:ident) => {{
        // TODO(mbm): return error instead of assert
        assert_eq!(
            $args.len(),
            3,
            "Unexpected number of args for ternary operator at {:?}",
            $pos
        );
        $expr(
            expr_from_ast(&$args[0]),
            expr_from_ast(&$args[1]),
            expr_from_ast(&$args[2]),
        )
    }};
}

macro_rules! variadic_expr {
    ($expr:path, $args:ident, $pos:ident) => {{
        // TODO(mbm): return error instead of assert
        assert!(
            $args.len() >= 1,
            "Unexpected number of args for variadic binary operator {:?}",
            $pos
        );
        $expr(exprs_from_ast($args))
    }};
}

fn expr_from_ast(expr: &ast::SpecExpr) -> Expr {
    Positioned::new(expr.pos(), ExprKind::from_ast(expr))
}

fn exprs_from_ast(exprs: &[ast::SpecExpr]) -> Vec<Expr> {
    exprs.iter().map(expr_from_ast).collect()
}

fn var_from_ident(ident: Ident) -> Expr {
    Positioned::new(ident.1, ExprKind::Var(ident))
}

impl ExprKind {
    fn from_ast(expr: &ast::SpecExpr) -> ExprKind {
        match expr {
            ast::SpecExpr::ConstInt { val, pos: _ } => ExprKind::Const(Const::Int(*val)),
            ast::SpecExpr::ConstBool { val, pos: _ } => ExprKind::Const(Const::Bool(*val)),
            ast::SpecExpr::ConstBitVec { val, width, pos: _ } => {
                ExprKind::Const(Const::BitVector(*width, (*val).into()))
            }
            ast::SpecExpr::Var { var, pos: _ } => ExprKind::Var(var.clone()),
            ast::SpecExpr::As { x, ty, pos: _ } => {
                ExprKind::As(expr_from_ast(x), Compound::from_ast(ty))
            }
            ast::SpecExpr::Field { field, x, pos: _ } => {
                ExprKind::Field(field.clone(), expr_from_ast(x))
            }
            ast::SpecExpr::Discriminator { variant, x, pos: _ } => {
                ExprKind::Discriminator(variant.clone(), expr_from_ast(x))
            }
            ast::SpecExpr::Op { op, args, pos } => match op {
                // Unary
                SpecOp::Not => unary_expr!(ExprKind::Not, args, pos),
                SpecOp::BVNot => unary_expr!(ExprKind::BVNot, args, pos),
                SpecOp::BVNeg => unary_expr!(ExprKind::BVNeg, args, pos),
                SpecOp::Cls => unary_expr!(ExprKind::Cls, args, pos),
                SpecOp::Rev => unary_expr!(ExprKind::Rev, args, pos),
                SpecOp::Clz => unary_expr!(ExprKind::Clz, args, pos),
                SpecOp::Popcnt => unary_expr!(ExprKind::Popcnt, args, pos),

                // Variadic binops
                SpecOp::And => variadic_expr!(ExprKind::And, args, pos),
                SpecOp::Or => variadic_expr!(ExprKind::Or, args, pos),

                // Binary
                SpecOp::Eq => binary_expr!(ExprKind::Eq, args, pos),
                SpecOp::Lt => binary_expr!(ExprKind::Lt, args, pos),
                SpecOp::Lte => binary_expr!(ExprKind::Lte, args, pos),
                SpecOp::Gt => binary_expr!(ExprKind::Gt, args, pos),
                SpecOp::Gte => binary_expr!(ExprKind::Gte, args, pos),
                SpecOp::Imp => binary_expr!(ExprKind::Imp, args, pos),
                SpecOp::Add => binary_expr!(ExprKind::Add, args, pos),
                SpecOp::Sub => binary_expr!(ExprKind::Sub, args, pos),
                SpecOp::Mul => binary_expr!(ExprKind::Mul, args, pos),
                SpecOp::BVAnd => binary_expr!(ExprKind::BVAnd, args, pos),
                SpecOp::BVOr => binary_expr!(ExprKind::BVOr, args, pos),
                SpecOp::BVXor => binary_expr!(ExprKind::BVXor, args, pos),
                SpecOp::BVAdd => binary_expr!(ExprKind::BVAdd, args, pos),
                SpecOp::BVSub => binary_expr!(ExprKind::BVSub, args, pos),
                SpecOp::BVMul => binary_expr!(ExprKind::BVMul, args, pos),
                SpecOp::BVSdiv => binary_expr!(ExprKind::BVSDiv, args, pos),
                SpecOp::BVUdiv => binary_expr!(ExprKind::BVUDiv, args, pos),
                SpecOp::BVUrem => binary_expr!(ExprKind::BVURem, args, pos),
                SpecOp::BVSrem => binary_expr!(ExprKind::BVSRem, args, pos),
                SpecOp::BVShl => binary_expr!(ExprKind::BVShl, args, pos),
                SpecOp::BVLshr => binary_expr!(ExprKind::BVLShr, args, pos),
                SpecOp::BVAshr => binary_expr!(ExprKind::BVAShr, args, pos),
                SpecOp::BVUle => binary_expr!(ExprKind::BVUle, args, pos),
                SpecOp::BVUlt => binary_expr!(ExprKind::BVUlt, args, pos),
                SpecOp::BVUgt => binary_expr!(ExprKind::BVUgt, args, pos),
                SpecOp::BVUge => binary_expr!(ExprKind::BVUge, args, pos),
                SpecOp::BVSlt => binary_expr!(ExprKind::BVSlt, args, pos),
                SpecOp::BVSle => binary_expr!(ExprKind::BVSle, args, pos),
                SpecOp::BVSgt => binary_expr!(ExprKind::BVSgt, args, pos),
                SpecOp::BVSge => binary_expr!(ExprKind::BVSge, args, pos),
                SpecOp::BVSaddo => binary_expr!(ExprKind::BVSaddo, args, pos),
                SpecOp::Rotr => binary_expr!(ExprKind::BVRotr, args, pos),
                SpecOp::Rotl => binary_expr!(ExprKind::BVRotl, args, pos),

                // Conversions
                SpecOp::ZeroExt => binary_expr!(ExprKind::BVZeroExt, args, pos),
                SpecOp::SignExt => binary_expr!(ExprKind::BVSignExt, args, pos),
                SpecOp::ConvTo => binary_expr!(ExprKind::BVConvTo, args, pos),
                SpecOp::Concat => variadic_expr!(ExprKind::BVConcat, args, pos),
                SpecOp::Replicate => {
                    // TODO(mbm): return error instead of assert
                    assert_eq!(
                        args.len(),
                        2,
                        "Unexpected number of args for extract operator at {pos:?}",
                    );
                    let repeat = spec_expr_to_usize(&args[1]).unwrap();
                    assert!(
                        repeat > 0,
                        "Unexpected repeat count for replicate operator at {pos:?}",
                    );
                    ExprKind::BVReplicate(expr_from_ast(&args[0]), repeat)
                }
                SpecOp::Extract => {
                    // TODO(mbm): return error instead of assert
                    assert_eq!(
                        args.len(),
                        3,
                        "Unexpected number of args for extract operator at {pos:?}",
                    );
                    ExprKind::BVExtract(
                        spec_expr_to_usize(&args[0]).unwrap(),
                        spec_expr_to_usize(&args[1]).unwrap(),
                        expr_from_ast(&args[2]),
                    )
                }
                SpecOp::Int2BV => binary_expr!(ExprKind::Int2BV, args, pos),
                SpecOp::BV2Nat => unary_expr!(ExprKind::BV2Nat, args, pos),
                SpecOp::WidthOf => unary_expr!(ExprKind::WidthOf, args, pos),
                SpecOp::ToFP => binary_expr!(ExprKind::ToFP, args, pos),
                SpecOp::ToFPUnsigned => binary_expr!(ExprKind::ToFPUnsigned, args, pos),
                SpecOp::ToFPFromFP => binary_expr!(ExprKind::ToFPFromFP, args, pos),
                SpecOp::FPToUBV => binary_expr!(ExprKind::FPToUBV, args, pos),
                SpecOp::FPToSBV => binary_expr!(ExprKind::FPToSBV, args, pos),

                // Floating point (IEEE)
                SpecOp::FPPositiveInfinity => unary_expr!(ExprKind::FPPositiveInfinity, args, pos),
                SpecOp::FPNegativeInfinity => unary_expr!(ExprKind::FPNegativeInfinity, args, pos),
                SpecOp::FPPositiveZero => unary_expr!(ExprKind::FPPositiveZero, args, pos),
                SpecOp::FPNegativeZero => unary_expr!(ExprKind::FPNegativeZero, args, pos),
                SpecOp::FPNaN => unary_expr!(ExprKind::FPNaN, args, pos),
                SpecOp::FPEq => binary_expr!(ExprKind::FPEq, args, pos),
                SpecOp::FPNe => binary_expr!(ExprKind::FPNe, args, pos),
                SpecOp::FPLt => binary_expr!(ExprKind::FPLt, args, pos),
                SpecOp::FPGt => binary_expr!(ExprKind::FPGt, args, pos),
                SpecOp::FPLe => binary_expr!(ExprKind::FPLe, args, pos),
                SpecOp::FPGe => binary_expr!(ExprKind::FPGe, args, pos),
                SpecOp::FPAdd => binary_expr!(ExprKind::FPAdd, args, pos),
                SpecOp::FPSub => binary_expr!(ExprKind::FPSub, args, pos),
                SpecOp::FPMul => binary_expr!(ExprKind::FPMul, args, pos),
                SpecOp::FPDiv => binary_expr!(ExprKind::FPDiv, args, pos),
                SpecOp::FPMin => binary_expr!(ExprKind::FPMin, args, pos),
                SpecOp::FPMax => binary_expr!(ExprKind::FPMax, args, pos),
                SpecOp::FPNeg => unary_expr!(ExprKind::FPNeg, args, pos),
                SpecOp::FPCeil => unary_expr!(ExprKind::FPCeil, args, pos),
                SpecOp::FPFloor => unary_expr!(ExprKind::FPFloor, args, pos),
                SpecOp::FPSqrt => unary_expr!(ExprKind::FPSqrt, args, pos),
                SpecOp::FPTrunc => unary_expr!(ExprKind::FPTrunc, args, pos),
                SpecOp::FPNearest => unary_expr!(ExprKind::FPNearest, args, pos),
                SpecOp::FPIsZero => unary_expr!(ExprKind::FPIsZero, args, pos),
                SpecOp::FPIsInfinite => unary_expr!(ExprKind::FPIsInfinite, args, pos),
                SpecOp::FPIsNaN => unary_expr!(ExprKind::FPIsNaN, args, pos),
                SpecOp::FPIsNegative => unary_expr!(ExprKind::FPIsNegative, args, pos),
                SpecOp::FPIsPositive => unary_expr!(ExprKind::FPIsPositive, args, pos),

                // Conditionals
                SpecOp::If => ternary_expr!(ExprKind::Conditional, args, pos),
                SpecOp::Switch => {
                    assert!(
                        args.len() > 1,
                        "Unexpected number of args for switch operator {pos:?}",
                    );
                    let on = expr_from_ast(&args[0]);
                    let arms: Vec<_> = args[1..]
                        .iter()
                        .map(|p| match p {
                            ast::SpecExpr::Pair { l, r, pos: _ } => {
                                (expr_from_ast(l), expr_from_ast(r))
                            }
                            // TODO(mbm): error rather than panic for non-pair in switch, since it's not actually unreachable
                            _ => unreachable!("switch expression arguments must be pairs"),
                        })
                        .collect();
                    ExprKind::Switch(on, arms)
                }
            },
            ast::SpecExpr::Match { x, arms, pos: _ } => {
                let x = expr_from_ast(x);
                let arms = arms
                    .iter()
                    .map(|arm| Arm {
                        variant: arm.variant.clone(),
                        args: arm.args.clone(),
                        body: expr_from_ast(&arm.body),
                    })
                    .collect();
                ExprKind::Match(x, arms)
            }
            ast::SpecExpr::Let { defs, body, pos: _ } => {
                let defs = defs
                    .iter()
                    .map(|(ident, x)| (ident.clone(), expr_from_ast(x)))
                    .collect();
                let body = expr_from_ast(body);
                ExprKind::Let(defs, body)
            }
            ast::SpecExpr::With {
                decls,
                body,
                pos: _,
            } => {
                let decls = decls.clone();
                let body = expr_from_ast(body);
                ExprKind::With(decls, body)
            }
            ast::SpecExpr::Pair { l, r, pos: _ } => {
                unreachable!(
                    "pairs must only occur in switch expressions, {:?} {:?}",
                    l, r
                )
            }
            ast::SpecExpr::Enum {
                name,
                variant,
                args,
                pos: _,
            } => ExprKind::Constructor(Constructor::Enum {
                name: name.clone(),
                variant: variant.clone(),
                args: exprs_from_ast(args),
            }),
            ast::SpecExpr::Struct { fields, pos: _ } => {
                ExprKind::Constructor(Constructor::Struct {
                    fields: fields.iter().map(FieldInit::from_ast).collect(),
                })
            }
            ast::SpecExpr::Macro {
                params,
                body,
                pos: _,
            } => ExprKind::Macro(params.clone(), expr_from_ast(body)),
            ast::SpecExpr::Expand { name, args, pos: _ } => {
                ExprKind::Expand(name.clone(), exprs_from_ast(args))
            }
        }
    }
}

fn spec_expr_to_usize(expr: &ast::SpecExpr) -> Option<usize> {
    match expr {
        &ast::SpecExpr::ConstInt { val, pos: _ } => {
            // TODO(mbm): return error rather than unwrap
            Some(val.try_into().expect("constant should be unsigned size"))
        }
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub enum Constructor {
    Enum {
        // TODO(mbm): Enum identifiers should be mapped to TermId?
        name: Ident,
        variant: Ident,
        args: Vec<Expr>,
    },
    Struct {
        fields: Vec<FieldInit>,
    },
}

#[derive(Debug, Clone)]
pub struct Arm {
    pub variant: Ident,
    pub args: Vec<Ident>,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub struct FieldInit {
    pub name: Ident,
    pub value: Expr,
}

impl FieldInit {
    fn from_ast(field: &ast::FieldInit) -> Self {
        Self {
            name: field.name.clone(),
            value: expr_from_ast(&field.value),
        }
    }
}

static RESULT: &str = "result";

pub struct Spec {
    pub args: Vec<Ident>,
    pub ret: Ident,
    pub provides: Vec<Expr>,
    pub requires: Vec<Expr>,
    pub matches: Vec<Expr>,
    pub modifies: Vec<Modifies>,
    pub pos: Pos,
}

impl Spec {
    fn new() -> Self {
        Self {
            args: Vec::new(),
            ret: Self::result_ident(),
            provides: Vec::new(),
            requires: Vec::new(),
            matches: Vec::new(),
            modifies: Vec::new(),
            pos: Pos::default(),
        }
    }

    fn from_ast(spec: &ast::Spec) -> Self {
        Self {
            args: spec.args.clone(),
            ret: Self::result_ident(),
            provides: exprs_from_ast(&spec.provides),
            requires: exprs_from_ast(&spec.requires),
            matches: exprs_from_ast(&spec.matches),
            modifies: spec.modifies.clone(),
            pos: spec.pos,
        }
    }

    fn result_ident() -> Ident {
        Ident(RESULT.to_string(), Pos::default())
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub name: Ident,
    pub ty: Compound,
    pub default: Expr,
}

#[derive(Debug, Clone)]
pub struct Signature {
    pub args: Vec<Compound>,
    pub ret: Compound,
}

impl Signature {
    fn from_ast(sig: &ast::Signature) -> Self {
        Self {
            args: sig.args.iter().map(Compound::from_ast).collect(),
            ret: Compound::from_ast(&sig.ret),
        }
    }
}

impl std::fmt::Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({args}) -> {ret}",
            args = self
                .args
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            ret = self.ret
        )
    }
}

pub struct Macro {
    pub name: Ident,
    pub params: Vec<Ident>,
    pub body: Expr,
}

pub struct SpecEnv {
    /// Specification for the given term.
    pub term_spec: HashMap<TermId, Spec>,

    /// State elements.
    pub state: Vec<State>,

    /// Terms that should be chained.
    pub chain: HashSet<TermId>,

    /// Tags applied to each term.
    pub term_tags: HashMap<TermId, HashSet<String>>,

    // Type instantiations for the given term.
    pub term_instantiations: HashMap<TermId, Vec<Signature>>,

    /// Rules for which priority is significant.
    pub priority: HashSet<RuleId>,

    /// Tags applied to each rule.
    pub rule_tags: HashMap<RuleId, HashSet<String>>,

    /// Model for the given type.
    pub type_model: HashMap<TypeId, Compound>,

    /// Value for the given constant.
    pub const_value: HashMap<Sym, Expr>,

    /// Macro definitions.
    pub macros: HashMap<String, Macro>,
}

impl SpecEnv {
    pub fn from_ast(defs: &[Def], termenv: &TermEnv, tyenv: &TypeEnv) -> Result<Self> {
        let mut env = Self {
            term_spec: HashMap::new(),
            state: Vec::new(),
            chain: HashSet::new(),
            term_tags: HashMap::new(),
            term_instantiations: HashMap::new(),
            priority: HashSet::new(),
            rule_tags: HashMap::new(),
            type_model: HashMap::new(),
            const_value: HashMap::new(),
            macros: HashMap::new(),
        };

        env.collect_models(defs, tyenv);
        env.derive_type_models(tyenv)?;
        env.derive_enum_variant_specs(termenv, tyenv)?;
        env.collect_state(defs)?;
        env.collect_instantiations(defs, termenv, tyenv);
        env.collect_specs(defs, termenv, tyenv)?;
        env.collect_attrs(defs, termenv, tyenv)?;
        env.collect_macros(defs);
        env.check_option_return_term_specs_uses_matches(termenv, tyenv)?;
        env.check_for_chained_terms_with_spec();

        Ok(env)
    }

    fn collect_models(&mut self, defs: &[Def], tyenv: &TypeEnv) {
        for def in defs {
            if let ast::Def::Model(Model { name, val }) = def {
                match val {
                    ast::ModelValue::TypeValue(model_type) => {
                        self.set_model_type(name, model_type, tyenv);
                    }
                    ast::ModelValue::ConstValue(val) => {
                        // TODO(mbm): error on missing constant name rather than panic
                        let sym = tyenv.intern(name).expect("constant name should be defined");
                        // TODO(mbm): enforce that the expression is constant.
                        // TODO(mbm): ensure the type of the expression matches the type of the
                        self.const_value.insert(sym, expr_from_ast(val));
                    }
                }
            }
        }
    }

    fn derive_type_models(&mut self, tyenv: &TypeEnv) -> Result<()> {
        for ty in &tyenv.types {
            // Has an explicit model already been specified?
            if self.has_model(ty.id()) {
                continue;
            }

            // Derive a model from ISLE type, if possible.
            let Some(derived_type) = Compound::from_isle(ty, tyenv) else {
                continue;
            };

            // Register derived.
            self.type_model.insert(ty.id(), derived_type);
        }
        Ok(())
    }

    fn derive_enum_variant_specs(&mut self, termenv: &TermEnv, tyenv: &TypeEnv) -> Result<()> {
        for model in self.type_model.values() {
            if let Compound::Enum(e) = model {
                for variant in &e.variants {
                    // Lookup the corresponding term.
                    let full_name = ast::Variant::full_name(&e.name, &variant.name);
                    let term_id =
                        termenv
                            .get_term_by_name(tyenv, &full_name)
                            .ok_or(format_err!(
                                "could not find variant term {name}",
                                name = full_name.0
                            ))?;

                    // Synthesize spec.
                    let pos = variant.name.1;
                    let args: Vec<Ident> = variant.fields.iter().map(|f| f.name.clone()).collect();
                    let constructor = Positioned::new(
                        pos,
                        ExprKind::Constructor(Constructor::Enum {
                            name: e.name.clone(),
                            variant: variant.name.clone(),
                            args: args.iter().cloned().map(var_from_ident).collect(),
                        }),
                    );

                    let mut spec = Spec::new();
                    spec.args = args;
                    let ret = var_from_ident(spec.ret.clone());
                    spec.provides
                        .push(Positioned::new(pos, ExprKind::Eq(ret, constructor)));
                    self.term_spec.insert(term_id, spec);
                    self.term_tags
                        .entry(term_id)
                        .or_default()
                        .insert("internal_derived_spec".to_string());
                }
            }
        }
        Ok(())
    }

    fn set_model_type(&mut self, name: &Ident, model_type: &ModelType, tyenv: &TypeEnv) {
        // TODO(mbm): error on missing type rather than panic
        let type_id = tyenv
            .get_type_by_name(name)
            .expect("type name should be defined");
        // TODO(mbm): error on duplicate model
        assert!(
            !self.type_model.contains_key(&type_id),
            "duplicate type model: {name}",
            name = name.0
        );
        self.type_model
            .insert(type_id, Compound::from_ast(model_type));
    }

    fn collect_state(&mut self, defs: &[Def]) -> Result<()> {
        // Collect states.
        for def in defs {
            if let ast::Def::State(ast::State {
                name,
                ty,
                default,
                pos: _,
            }) = def
            {
                let ty = Compound::from_ast(ty);
                let default = expr_from_ast(default);
                self.state.push(State {
                    name: name.clone(),
                    ty,
                    default,
                });
            }
        }

        // Check for duplicates.
        let mut names = HashSet::new();
        for state in &self.state {
            let name = &state.name.0;
            if names.contains(name) {
                bail!("duplicate state {name}");
            }
            names.insert(name);
        }

        Ok(())
    }

    fn collect_instantiations(&mut self, defs: &[Def], termenv: &TermEnv, tyenv: &TypeEnv) {
        // Collect form signatures first, as they may be referenced by instantiations.
        let mut form_signature = HashMap::new();
        for def in defs {
            if let ast::Def::Form(form) = def {
                let signatures: Vec<_> = form.signatures.iter().map(Signature::from_ast).collect();
                form_signature.insert(form.name.0.clone(), signatures);
            }
        }

        // Collect instantiations.
        for def in defs {
            if let ast::Def::Instantiation(inst) = def {
                let term_id = termenv.get_term_by_name(tyenv, &inst.term).unwrap();
                let sigs = match &inst.form {
                    Some(form) => form_signature[&form.0].clone(),
                    None => inst.signatures.iter().map(Signature::from_ast).collect(),
                };
                self.term_instantiations.insert(term_id, sigs);
            }
        }
    }

    fn collect_specs(&mut self, defs: &[Def], termenv: &TermEnv, tyenv: &TypeEnv) -> Result<()> {
        for def in defs {
            if let ast::Def::Spec(spec) = def {
                let term_id = termenv
                    .get_term_by_name(tyenv, &spec.term)
                    .ok_or(format_err!(
                        "spec for unknown term {name}",
                        name = spec.term.0
                    ))?;
                match self.term_spec.entry(term_id) {
                    Entry::Occupied(_) => {
                        bail!("duplicate spec for term {name}", name = spec.term.0)
                    }
                    Entry::Vacant(e) => {
                        e.insert(Spec::from_ast(spec));
                    }
                }
            }
        }
        Ok(())
    }

    fn collect_attrs(&mut self, defs: &[Def], termenv: &TermEnv, tyenv: &TypeEnv) -> Result<()> {
        for def in defs {
            if let ast::Def::Attr(attr) = def {
                match &attr.target {
                    AttrTarget::Term(name) => {
                        let term_id = termenv.get_term_by_name(tyenv, name).ok_or(format_err!(
                            "attr term '{name}' should exist",
                            name = name.0
                        ))?;
                        for kind in &attr.kinds {
                            match kind {
                                AttrKind::Chain => {
                                    self.chain.insert(term_id);
                                }
                                AttrKind::Tag(tag) => {
                                    self.term_tags
                                        .entry(term_id)
                                        .or_default()
                                        .insert(tag.0.clone());
                                }
                                AttrKind::Priority => {
                                    bail!("priority attribute cannot be applied to terms");
                                }
                            }
                        }
                    }
                    AttrTarget::Rule(name) => {
                        let rule_id = termenv
                            .get_rule_by_name(tyenv, name)
                            .ok_or(format_err!("attr rule '{}' does not exist", name.0))?;
                        for kind in &attr.kinds {
                            match kind {
                                AttrKind::Priority => {
                                    self.priority.insert(rule_id);
                                }
                                AttrKind::Tag(tag) => {
                                    self.rule_tags
                                        .entry(rule_id)
                                        .or_default()
                                        .insert(tag.0.clone());
                                }
                                AttrKind::Chain => {
                                    bail!("chain attribute cannot be applied to rule");
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn collect_macros(&mut self, defs: &[Def]) {
        for def in defs {
            if let ast::Def::SpecMacro(spec_macro) = def {
                let body = expr_from_ast(&spec_macro.body);
                self.macros.insert(
                    spec_macro.name.0.clone(),
                    Macro {
                        name: spec_macro.name.clone(),
                        params: spec_macro.params.clone(),
                        body,
                    },
                );
            }
        }
    }

    fn check_option_return_term_specs_uses_matches(
        &self,
        termenv: &TermEnv,
        tyenv: &TypeEnv,
    ) -> Result<()> {
        for (term_id, spec) in &self.term_spec {
            let term = &termenv.terms[term_id.index()];
            if !Self::term_returns_option(term, tyenv) {
                continue;
            }
            if !spec.requires.is_empty() {
                bail!(
                    "term '{name}' requires should be match",
                    name = tyenv.syms[term.name.index()],
                );
            }
        }
        Ok(())
    }

    fn term_returns_option(term: &Term, tyenv: &TypeEnv) -> bool {
        // Constructor
        if term.has_constructor() {
            return term.is_partial();
        }

        // External extractor
        if let Some(sig) = term.extractor_sig(tyenv) {
            return sig.ret_kind == ReturnKind::Option;
        }

        // Extractor
        if term.has_extractor() {
            return true;
        }

        false
    }

    fn check_for_chained_terms_with_spec(&self) {
        for term_id in &self.chain {
            // TODO(mbm): error rather than panic
            assert!(
                !self.term_spec.contains_key(term_id),
                "chained term should not have spec"
            );
        }
    }

    /// Resolve any named types in the given compound type.
    pub fn resolve_type(&self, ty: &Compound, tyenv: &TypeEnv) -> Result<Compound> {
        ty.resolve(&mut |name| {
            let type_id = tyenv
                .get_type_by_name(name)
                .ok_or(format_err!("unknown type {}", name.0))?;
            let ty = self.type_model.get(&type_id).ok_or(format_err!(
                "unspecified model for type `{}`: a spec references this type (directly, or \
                 via a term signature or another model), but it has no `(model ...)` \
                 declaration. Add a `(model {} ...)` form in a spec file describing its \
                 representation.",
                name.0,
                name.0
            ))?;
            Ok(ty.clone())
        })
    }

    /// Resolve any named types in the given term signature.
    pub fn resolve_signature(&self, sig: &Signature, tyenv: &TypeEnv) -> Result<Signature> {
        Ok(Signature {
            args: sig
                .args
                .iter()
                .map(|arg| self.resolve_type(arg, tyenv))
                .collect::<Result<_>>()?,
            ret: self.resolve_type(&sig.ret, tyenv)?,
        })
    }

    /// Lookup instantiations for the given term, with any named types resolved.
    pub fn resolve_term_instantiations(
        &self,
        term_id: &TermId,
        tyenv: &TypeEnv,
    ) -> Result<Vec<Signature>> {
        let Some(sigs) = self.term_instantiations.get(term_id) else {
            return Ok(Vec::new());
        };

        sigs.iter()
            .map(|sig| self.resolve_signature(sig, tyenv))
            .collect::<Result<_>>()
    }

    /// Report whether the given term has a specification.
    pub fn has_spec(&self, term_id: TermId) -> bool {
        self.term_spec.contains_key(&term_id)
    }

    pub fn has_model(&self, type_id: TypeId) -> bool {
        self.type_model.contains_key(&type_id)
    }
}
