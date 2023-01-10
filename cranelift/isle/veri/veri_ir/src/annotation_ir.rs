/// A higher-level annotation IR that does not specify bitvector widths.
/// This allows annotations to be generic over possible types, which
/// corresponds to how ISLE rewrites are written.

/// A bound variable, including the VIR type
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundVar {
    pub name: String,
    pub ty: Option<Type>,
}

impl BoundVar {
    // TODO: special case this for function bound vars?
    /// Construct a new bound variable, cloning from references
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
        Expr::Var(self.name.clone(), 0)
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
    pub assertions: Vec<Box<Expr>>,
}

impl TermAnnotation {
    /// New annotation
    pub fn new(sig: TermSignature, assertions: Vec<Expr>) -> Self {
        TermAnnotation {
            sig,
            assertions: assertions.iter().map(|x| Box::new(x.clone())).collect(),
        }
    }

    pub fn sig(&self) -> &TermSignature {
        &self.sig
    }

    pub fn assertions(&self) -> Vec<Expr> {
        self.assertions.iter().map(|x| *x.clone()).collect()
    }
}

/// Function type with argument and return types.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct FunctionType {
    pub args: Vec<Type>,
    pub ret: Box<Type>,
}

/// Higher-level type, not including bitwidths.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Type {
    /// Internal type used solely for type inference
    Poly(u32),

    /// The expression is a bitvector, currently modeled in the
    /// logic QF_BV https://smtlib.cs.uiowa.edu/version1/logics/QF_BV.smt
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
}

impl Type {
    pub fn is_poly(&self) -> bool {
        match self {
            Type::Poly(_) => true,
            _ => false,
        }
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
    pub func: Box<Expr>,
    // Note: extra Box for now for ease of parsing
    #[allow(clippy::vec_box)]
    pub args: Vec<Box<Expr>>,
}

/// Typed expressions (u32 is the type var)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    // Terminal nodes
    Var(String, u32),
    Const(Const, u32),
    True(u32),
    False(u32),

    // Special terminal node: the current width
    TyWidth(u32),

    // Get the width of a bitvector
    WidthOf(Box<Expr>, u32),

    // Boolean operations
    Not(Box<Expr>, u32),
    And(Box<Expr>, Box<Expr>, u32),
    Or(Box<Expr>, Box<Expr>, u32),
    Imp(Box<Expr>, Box<Expr>, u32),
    Eq(Box<Expr>, Box<Expr>, u32),
    Lte(Box<Expr>, Box<Expr>, u32),

    // Bitvector operations
    //      Note: these follow the naming conventions of the SMT theory of bitvectors:
    //      https://smtlib.cs.uiowa.edu/version1/logics/QF_BV.smt
    // Unary operators
    BVNeg(Box<Expr>, u32),
    BVNot(Box<Expr>, u32),
    CLZ(Box<Expr>, u32),
    A64CLZ(Box<Expr>, Box<Expr>, u32),

    // Binary operators
    BVMul(Box<Expr>, Box<Expr>, u32),
    BVAdd(Box<Expr>, Box<Expr>, u32),
    BVSub(Box<Expr>, Box<Expr>, u32),
    BVAnd(Box<Expr>, Box<Expr>, u32),
    BVOr(Box<Expr>, Box<Expr>, u32),
    BVXor(Box<Expr>, Box<Expr>, u32),
    BVRotl(Box<Expr>, Box<Expr>, u32),
    BVRotr(Box<Expr>, Box<Expr>, u32),
    BVShl(Box<Expr>, Box<Expr>, u32),
    BVShr(Box<Expr>, Box<Expr>, u32),

    // Conversions
    // Zero extend, static and dynamic width
    BVZeroExtTo(Box<Width>, Box<Expr>, u32),
    BVZeroExtToVarWidth(Box<Expr>, Box<Expr>, u32),

    // Sign extend, static and dynamic width
    BVSignExtTo(Box<Width>, Box<Expr>, u32),
    BVSignExtToVarWidth(Box<Expr>, Box<Expr>, u32),

    // Extract specified bits
    BVExtract(usize, usize, Box<Expr>, u32),

    // Convert integer to bitvector
    BVIntToBv(usize, Box<Expr>, u32),

    // Convert bitvector to integer
    BVToInt(Box<Expr>, u32),

    // Conversion to wider/narrower bits, without an explicit extend
    BVConvTo(Box<Width>, Box<Expr>, u32),
    // Allow the destination width to be symbolic.
    BVConvToVarWidth(Box<Expr>, Box<Expr>, u32),

    // Conditional if-then-else
    Conditional(Box<Expr>, Box<Expr>, Box<Expr>, u32),
}

impl Expr {
    pub fn var(s: &str) -> Expr {
        Expr::Var(s.to_string(), 0)
    }

    pub fn unary<F: Fn(Box<Expr>, u32) -> Expr>(f: F, x: Expr) -> Expr {
        f(Box::new(x), 0)
    }

    pub fn binary<F: Fn(Box<Expr>, Box<Expr>, u32) -> Expr>(f: F, x: Expr, y: Expr) -> Expr {
        f(Box::new(x), Box::new(y), 0)
    }

    pub fn get_type_var(x: &Expr) -> u32 {
        match x {
            Expr::True(t)
            | Expr::False(t)
            | Expr::TyWidth(t)
            | Expr::Var(_, t)
            | Expr::Const(_, t)
            | Expr::WidthOf(_, t)
            | Expr::Not(_, t)
            | Expr::BVNeg(_, t)
            | Expr::BVNot(_, t)
            | Expr::CLZ(_, t)
            | Expr::A64CLZ(_, _, t)
            | Expr::And(_, _, t)
            | Expr::Or(_, _, t)
            | Expr::Imp(_, _, t)
            | Expr::Eq(_, _, t)
            | Expr::Lte(_, _, t)
            | Expr::BVMul(_, _, t)
            | Expr::BVAdd(_, _, t)
            | Expr::BVSub(_, _, t)
            | Expr::BVAnd(_, _, t)
            | Expr::BVOr(_, _, t)
            | Expr::BVXor(_, _, t)
            | Expr::BVRotl(_, _, t)
            | Expr::BVRotr(_, _, t)
            | Expr::BVShl(_, _, t)
            | Expr::BVShr(_, _, t)
            | Expr::BVZeroExtTo(_, _, t)
            | Expr::BVZeroExtToVarWidth(_, _, t)
            | Expr::BVSignExtTo(_, _, t)
            | Expr::BVSignExtToVarWidth(_, _, t)
            | Expr::BVIntToBv(_, _, t)
            | Expr::BVToInt(_, t)
            | Expr::BVConvTo(_, _, t)
            | Expr::BVConvToVarWidth(_, _, t)
            | Expr::Conditional(_, _, _, t)
            | Expr::BVExtract(_, _, _, t) => *t,
        }
    }
}
