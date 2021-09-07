use crate::lexer::Pos;

/// The parsed form of an ISLE file.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Defs {
    pub defs: Vec<Def>,
    pub filenames: Vec<String>,
}

/// One toplevel form in an ISLE file.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Def {
    Type(Type),
    Rule(Rule),
    Decl(Decl),
    Extern(Extern),
}

/// An identifier -- a variable, term symbol, or type.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ident(pub String);

/// A declaration of a type.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Type {
    pub name: Ident,
    pub is_extern: bool,
    pub ty: TypeValue,
    pub pos: Pos,
}

/// The actual type-value: a primitive or an enum with variants.
///
/// TODO: add structs as well?
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TypeValue {
    Primitive(Ident),
    Enum(Vec<Variant>),
}

/// One variant of an enum type.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Variant {
    pub name: Ident,
    pub fields: Vec<Field>,
}

/// One field of an enum variant.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Field {
    pub name: Ident,
    pub ty: Ident,
}

/// A declaration of a term with its argument and return types.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Decl {
    pub term: Ident,
    pub arg_tys: Vec<Ident>,
    pub ret_ty: Ident,
    pub pos: Pos,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Rule {
    pub pattern: Pattern,
    pub expr: Expr,
    pub pos: Pos,
    pub prio: Option<i64>,
}

/// A pattern: the left-hand side of a rule.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Pattern {
    /// An operator that binds a variable to a subterm and match the
    /// subpattern.
    BindPattern { var: Ident, subpat: Box<Pattern> },
    /// A variable that has already been bound (`=x` syntax).
    Var { var: Ident },
    /// An operator that matches a constant integer value.
    ConstInt { val: i64 },
    /// An application of a type variant or term.
    Term { sym: Ident, args: Vec<Pattern> },
    /// An operator that matches anything.
    Wildcard,
    /// N sub-patterns that must all match.
    And { subpats: Vec<Pattern> },
}

/// An expression: the right-hand side of a rule.
///
/// Note that this *almost* looks like a core Lisp or lambda calculus,
/// except that there is no abstraction (lambda). This first-order
/// limit is what makes it analyzable.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Expr {
    /// A term: `(sym args...)`.
    Term { sym: Ident, args: Vec<Expr> },
    /// A variable use.
    Var { name: Ident },
    /// A constant integer.
    ConstInt { val: i64 },
    /// The `(let ((var ty val)*) body)` form.
    Let { defs: Vec<LetDef>, body: Box<Expr> },
}

/// One variable locally bound in a `(let ...)` expression.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LetDef {
    pub var: Ident,
    pub ty: Ident,
    pub val: Box<Expr>,
}

/// An external binding: an extractor or constructor function attached
/// to a term.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Extern {
    /// An external extractor: `(extractor Term rustfunc)` form.
    Extractor {
        /// The term to which this external extractor is attached.
        term: Ident,
        /// The Rust function name.
        func: Ident,
        /// The position of this decl.
        pos: Pos,
        /// Whether this extractor is infallible (always matches).
        infallible: bool,
    },
    /// An external constructor: `(constructor Term rustfunc)` form.
    Constructor {
        /// The term to which this external constructor is attached.
        term: Ident,
        /// The Rust function name.
        func: Ident,
        /// The position of this decl.
        pos: Pos,
    },
}
