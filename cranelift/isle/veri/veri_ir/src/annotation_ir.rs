/// A higher-level annotation IR that does not specify bitvector widths.
/// This allows annotations to be generic over possible types, which
/// corresponds to how ISLE rewrites are written.
use std::fmt;
/// A bound variable, including the VIR type
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundVar {
    pub name: String,
    pub ty: Option<Type>,
}

impl BoundVar {
    /// Construct a new bound variable
    pub fn new_with_ty(name: &str, ty: &Type) -> Self {
        BoundVar {
            name: name.to_string(),
            ty: Some(ty.clone()),
        }
    }

    /// Construct a new bound variable, cloning from references
    pub fn new(name: &str) -> Self {
        BoundVar {
            name: name.to_string(),
            ty: None,
        }
    }

    /// An expression with the bound variable's name
    pub fn as_expr(&self) -> Expr {
        Expr::Var(self.name.clone())
    }
}

/// A function signature annotation, including the bound variable names for all
/// arguments and the return value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TermSignature {
    pub args: Vec<BoundVar>,
    pub ret: BoundVar,
}

/// Verification IR annotations for an ISLE term consist of the function
/// signature and a list of assertions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TermAnnotation {
    pub sig: TermSignature,
    // Note: extra Box for now for ease of parsing
    #[allow(clippy::vec_box)]
    pub assumptions: Vec<Box<Expr>>,

    #[allow(clippy::vec_box)]
    pub assertions: Vec<Box<Expr>>,
}

impl TermAnnotation {
    /// New annotation
    pub fn new(sig: TermSignature, assumptions: Vec<Expr>, assertions: Vec<Expr>) -> Self {
        TermAnnotation {
            sig,
            assumptions: assumptions.iter().map(|x| Box::new(x.clone())).collect(),
            assertions: assertions.iter().map(|x| Box::new(x.clone())).collect(),
        }
    }

    pub fn sig(&self) -> &TermSignature {
        &self.sig
    }

    pub fn assertions(&self) -> Vec<Expr> {
        self.assumptions.iter().map(|x| *x.clone()).collect()
    }
}

/// Higher-level type, not including bitwidths.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Type {
    /// Internal type used solely for type inference
    Poly(u32),

    /// The expression is a bitvector, currently modeled in the
    /// logic QF_BV https://SMT-LIB.cs.uiowa.edu/version1/logics/QF_BV.smt
    /// This corresponds to Cranelift's Isle type:
    /// (type Value (primitive Value))
    BitVector,

    /// Use if the width is known
    BitVectorWithWidth(usize),

    // Use if the width is unknown after inference, indexed by a
    // cannonical type variable
    BitVectorUnknown(u32),

    /// The expression is an integer (currently used for ISLE type,
    /// representing bitwidth)
    Int,

    /// The expression is a boolean.
    Bool,

    /// Unit, removed before SMT-Lib
    Unit,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Poly(_) => write!(f, "poly"),
            Type::BitVector => write!(f, "bv"),
            Type::BitVectorWithWidth(w) => write!(f, "bv{}", *w),
            Type::BitVectorUnknown(_) => write!(f, "bv"),
            Type::Int => write!(f, "Int"),
            Type::Bool => write!(f, "Bool"),
            Type::Unit => write!(f, "Unit"),
        }
    }
}

impl Type {
    pub fn is_poly(&self) -> bool {
        matches!(self, Type::Poly(_))
    }
}

/// Type-specified constants
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Const {
    pub ty: Type,
    pub value: i128,
    pub width: usize,
}

/// Width arguments
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Width {
    Const(usize),
    RegWidth,
}

/// Typed expressions (u32 is the type var)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    // Terminal nodes
    Var(String),
    Const(Const),
    True,
    False,

    // Get the width of a bitvector
    WidthOf(Box<Expr>),

    // Boolean operations
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Imp(Box<Expr>, Box<Expr>),
    Eq(Box<Expr>, Box<Expr>),
    Lte(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),

    BVSgt(Box<Expr>, Box<Expr>),
    BVSgte(Box<Expr>, Box<Expr>),
    BVSlt(Box<Expr>, Box<Expr>),
    BVSlte(Box<Expr>, Box<Expr>),
    BVUgt(Box<Expr>, Box<Expr>),
    BVUgte(Box<Expr>, Box<Expr>),
    BVUlt(Box<Expr>, Box<Expr>),
    BVUlte(Box<Expr>, Box<Expr>),

    BVSaddo(Box<Expr>, Box<Expr>),

    // Bitvector operations
    //      Note: these follow the naming conventions of the SMT theory of bitvectors:
    //      https://SMT-LIB.cs.uiowa.edu/version1/logics/QF_BV.smt
    // Unary operators
    BVNeg(Box<Expr>),
    BVNot(Box<Expr>),
    CLZ(Box<Expr>),
    CLS(Box<Expr>),
    Rev(Box<Expr>),
    BVPopcnt(Box<Expr>),

    // Binary operators
    BVMul(Box<Expr>, Box<Expr>),
    BVUDiv(Box<Expr>, Box<Expr>),
    BVSDiv(Box<Expr>, Box<Expr>),
    BVAdd(Box<Expr>, Box<Expr>),
    BVSub(Box<Expr>, Box<Expr>),
    BVUrem(Box<Expr>, Box<Expr>),
    BVSrem(Box<Expr>, Box<Expr>),
    BVAnd(Box<Expr>, Box<Expr>),
    BVOr(Box<Expr>, Box<Expr>),
    BVXor(Box<Expr>, Box<Expr>),
    BVRotl(Box<Expr>, Box<Expr>),
    BVRotr(Box<Expr>, Box<Expr>),
    BVShl(Box<Expr>, Box<Expr>),
    BVShr(Box<Expr>, Box<Expr>),
    BVAShr(Box<Expr>, Box<Expr>),

    // Includes type
    BVSubs(Box<Expr>, Box<Expr>, Box<Expr>),

    // Conversions
    // Zero extend, static and dynamic width
    BVZeroExtTo(Box<Width>, Box<Expr>),
    BVZeroExtToVarWidth(Box<Expr>, Box<Expr>),

    // Sign extend, static and dynamic width
    BVSignExtTo(Box<Width>, Box<Expr>),
    BVSignExtToVarWidth(Box<Expr>, Box<Expr>),

    // Extract specified bits
    BVExtract(usize, usize, Box<Expr>),

    // Concat two bitvectors
    BVConcat(Vec<Expr>),

    // Convert integer to bitvector
    BVIntToBv(usize, Box<Expr>),

    // Convert bitvector to integer
    BVToInt(Box<Expr>),

    // Conversion to wider/narrower bits, without an explicit extend
    // Allow the destination width to be symbolic.
    BVConvTo(Box<Expr>, Box<Expr>),

    // Conditional if-then-else
    Conditional(Box<Expr>, Box<Expr>, Box<Expr>),

    // Switch
    Switch(Box<Expr>, Vec<(Expr, Expr)>),

    LoadEffect(Box<Expr>, Box<Expr>, Box<Expr>),

    StoreEffect(Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>),
}

impl Expr {
    pub fn var(s: &str) -> Expr {
        Expr::Var(s.to_string())
    }

    pub fn unary<F: Fn(Box<Expr>) -> Expr>(f: F, x: Expr) -> Expr {
        f(Box::new(x))
    }

    pub fn binary<F: Fn(Box<Expr>, Box<Expr>) -> Expr>(f: F, x: Expr, y: Expr) -> Expr {
        f(Box::new(x), Box::new(y))
    }
}
