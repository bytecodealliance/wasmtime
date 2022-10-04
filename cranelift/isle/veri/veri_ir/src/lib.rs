//! Verification Intermediate Representation for relevant types, eventually to
//! be lowered to SMT. The goal is to leave some freedom to change term
//! encodings or the specific solver backend.
//!
//! Note: annotations use the higher-level IR in annotation_ir.rs.
pub mod annotation_ir;
pub mod isle_annotations;

use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeContext {
    pub tyvars: HashMap<Expr, u32>,
    pub tymap: HashMap<u32, Type>,
    // map of type var to set index
    pub bv_unknown_width_sets: HashMap<u32, u32>,
}

/// Packaged semantics for a single rule, included metadata on which terms
/// are not yet defined.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleSemantics {
    pub lhs: Expr,
    pub rhs: Expr,

    pub quantified_vars: Vec<BoundVar>,
    pub free_vars: Vec<BoundVar>,
    pub assumptions: Vec<Expr>,

    pub tyctx: TypeContext,

    //  TODO: remove
    pub lhs_undefined_terms: Vec<UndefinedTerm>,
    pub rhs_undefined_terms: Vec<UndefinedTerm>,
}
// TODO: can nuke this
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RulePath {
    pub rules: Vec<RuleSemantics>,
    pub undefined_term_pairs: Vec<(UndefinedTerm, UndefinedTerm)>,
}

/// A structure linking rules that share intermediate terms. A path from a root
/// RuleSemantics to a leaf of the tree represents a valid rewriting if all
/// assumptions along the path are feasible.
#[derive(Clone, Debug)]
pub struct RuleTree {
    pub value: RuleSemantics,
    // maybe want an RC cell instead of a Box
    pub children: HashMap<BoundVar, Vec<RuleTree>>,
    pub height: usize,
}

/// Verification IR annotations for an ISLE term consist of the function
/// signature and a list of assertions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VIRTermAnnotation {
    pub sig: VIRTermSignature,
    pub assertions: Vec<Expr>,
}

impl VIRTermAnnotation {
    /// New annotation, ensuring that each assertions is a bool.
    pub fn new(sig: VIRTermSignature, assertions: Vec<Expr>) -> Self {
        // assert!(assertions.iter().all(|a| a.ty().is_bool()));
        VIRTermAnnotation { sig, assertions }
    }

    pub fn func(&self) -> &VIRTermSignature {
        &self.sig
    }

    pub fn assertions(&self) -> &Vec<Expr> {
        &self.assertions
    }
}
/// A function signature annotation, including the bound variable names for all
/// arguments and the return value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VIRTermSignature {
    pub args: Vec<BoundVar>,
    pub ret: BoundVar,
}
/// A bound function with named arguments, the VIR type signature, and the body
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub ty: Type,
    pub args: Vec<BoundVar>,
    pub body: Box<Expr>,
}

/// Application of a function expression to arguments
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionApplication {
    pub ty: Type,
    pub func: Box<Expr>,
    pub args: Vec<Expr>,
}
/// A bound variable, including the VIR type
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BoundVar {
    pub name: String,
    pub tyvar: u32,
}

/// An ISLE term that does not yet have a defined semantics (that is, a
/// term that has no annotation).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UndefinedTerm {
    pub name: String,
    pub ret: BoundVar,
    pub args: Vec<Expr>,
}

/// Verification type
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    /// The expression is a bitvector, currently modeled in the
    /// logic QF_BV https://smtlib.cs.uiowa.edu/version1/logics/QF_BV.smt
    /// This corresponds to Cranelift's Isle type:
    /// (type Value (primitive Value))
    BitVector(Option<usize>),

    /// The expression is a boolean. This does not directly correspond
    /// to a specific Cranelift Isle type, rather, we use it for the
    /// language of assertions.
    Bool,

    /// The expression is an Isle type. This is separate from BitVector
    /// because it allows us to use a different solver type (e.h., Int)
    //. for assertions (e.g., fits_in_64).
    /// This corresponds to Cranelift's Isle type:
    /// (type Type (primitive Type))
    Int,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Terminal {
    Var(String),
    Const(i128),
    True,
    False,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    // Boolean operations
    Not,

    // Bitvector operations
    BVNeg,
    BVNot,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    // Boolean operations
    And,
    Or,
    Imp,
    Eq,
    Lte,

    // Bitvector operations
    BVAdd,
    BVSub,
    BVAnd,
    BVOr,
    BVRotl,
    BVShl,
    BVShr,
}

/// Expressions (combined across all types).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Expr {
    // Terminal nodes
    Terminal(Terminal),

    // Opcode nodes
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),

    // ITE
    Conditional(Box<Expr>, Box<Expr>, Box<Expr>),

    // Conversions
    // Extract specified bits
    BVExtract(usize, usize, Box<Expr>),

    // Convert integer to bitvector with that value
    BVIntToBV(usize, Box<Expr>),

    // Zero extend, with static or dynamic width
    BVZeroExtTo(usize, Box<Expr>),
    BVZeroExtToVarWidth(Box<Expr>, Box<Expr>),

    // Sign extend, with static or dynamic width
    BVSignExt(usize, Box<Expr>),
    BVSignExtToVarWidth(Box<Expr>, Box<Expr>),

    // Conversion to wider/narrower bits, without an explicit extend
    BVConvTo(Box<Expr>),
    BVConvToVarWidth(Box<Expr>, Box<Expr>),

    WidthOf(Box<Expr>),

    // Undefined terms
    UndefinedTerm(UndefinedTerm),
}

pub fn all_starting_bitvectors() -> Vec<usize> {
    vec![1, 8, 16, 32, 64]
}

impl BoundVar {
    pub fn as_expr(&self) -> Expr {
        Expr::Terminal(Terminal::Var(self.name.clone()))
    }
}

/// To-be-flushed-out verification counterexample for failures
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Counterexample {}

/// To-be-flushed-out verification result
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerificationResult {
    InapplicableRule,
    Success,
    Failure(Counterexample),
    Unknown,
}
