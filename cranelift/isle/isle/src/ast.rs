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
    Extractor(Extractor),
    Decl(Decl),
    Extern(Extern),
}

/// An identifier -- a variable, term symbol, or type.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ident(pub String, pub Pos);

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
    Primitive(Ident, Pos),
    Enum(Vec<Variant>, Pos),
}

/// One variant of an enum type.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Variant {
    pub name: Ident,
    pub fields: Vec<Field>,
    pub pos: Pos,
}

/// One field of an enum variant.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Field {
    pub name: Ident,
    pub ty: Ident,
    pub pos: Pos,
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

/// An extractor macro: (A x y) becomes (B x _ y ...). Expanded during
/// ast-to-sema pass.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Extractor {
    pub term: Ident,
    pub args: Vec<Ident>,
    pub template: Pattern,
    pub pos: Pos,
}

/// A pattern: the left-hand side of a rule.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Pattern {
    /// An operator that binds a variable to a subterm and match the
    /// subpattern.
    BindPattern {
        var: Ident,
        subpat: Box<Pattern>,
        pos: Pos,
    },
    /// A variable that has already been bound (`=x` syntax).
    Var { var: Ident, pos: Pos },
    /// An operator that matches a constant integer value.
    ConstInt { val: i64, pos: Pos },
    /// An operator that matches an external constant value.
    ConstPrim { val: Ident, pos: Pos },
    /// An application of a type variant or term.
    Term {
        sym: Ident,
        args: Vec<TermArgPattern>,
        pos: Pos,
    },
    /// An operator that matches anything.
    Wildcard { pos: Pos },
    /// N sub-patterns that must all match.
    And { subpats: Vec<Pattern>, pos: Pos },
    /// Internal use only: macro argument in a template.
    MacroArg { index: usize, pos: Pos },
}

impl Pattern {
    pub fn root_term(&self) -> Option<&Ident> {
        match self {
            &Pattern::BindPattern { ref subpat, .. } => subpat.root_term(),
            &Pattern::Term { ref sym, .. } => Some(sym),
            _ => None,
        }
    }

    pub fn make_macro_template(&self, macro_args: &[Ident]) -> Pattern {
        log::trace!("make_macro_template: {:?} with {:?}", self, macro_args);
        match self {
            &Pattern::BindPattern {
                ref var,
                ref subpat,
                pos,
                ..
            } if matches!(&**subpat, &Pattern::Wildcard { .. }) => {
                if let Some(i) = macro_args.iter().position(|arg| arg.0 == var.0) {
                    Pattern::MacroArg { index: i, pos }
                } else {
                    self.clone()
                }
            }
            &Pattern::BindPattern {
                ref var,
                ref subpat,
                pos,
            } => Pattern::BindPattern {
                var: var.clone(),
                subpat: Box::new(subpat.make_macro_template(macro_args)),
                pos,
            },
            &Pattern::And { ref subpats, pos } => {
                let subpats = subpats
                    .iter()
                    .map(|subpat| subpat.make_macro_template(macro_args))
                    .collect::<Vec<_>>();
                Pattern::And { subpats, pos }
            }
            &Pattern::Term {
                ref sym,
                ref args,
                pos,
            } => {
                let args = args
                    .iter()
                    .map(|arg| arg.make_macro_template(macro_args))
                    .collect::<Vec<_>>();
                Pattern::Term {
                    sym: sym.clone(),
                    args,
                    pos,
                }
            }

            &Pattern::Var { .. }
            | &Pattern::Wildcard { .. }
            | &Pattern::ConstInt { .. }
            | &Pattern::ConstPrim { .. } => self.clone(),
            &Pattern::MacroArg { .. } => unreachable!(),
        }
    }

    pub fn subst_macro_args(&self, macro_args: &[Pattern]) -> Pattern {
        log::trace!("subst_macro_args: {:?} with {:?}", self, macro_args);
        match self {
            &Pattern::BindPattern {
                ref var,
                ref subpat,
                pos,
            } => Pattern::BindPattern {
                var: var.clone(),
                subpat: Box::new(subpat.subst_macro_args(macro_args)),
                pos,
            },
            &Pattern::And { ref subpats, pos } => {
                let subpats = subpats
                    .iter()
                    .map(|subpat| subpat.subst_macro_args(macro_args))
                    .collect::<Vec<_>>();
                Pattern::And { subpats, pos }
            }
            &Pattern::Term {
                ref sym,
                ref args,
                pos,
            } => {
                let args = args
                    .iter()
                    .map(|arg| arg.subst_macro_args(macro_args))
                    .collect::<Vec<_>>();
                Pattern::Term {
                    sym: sym.clone(),
                    args,
                    pos,
                }
            }

            &Pattern::Var { .. }
            | &Pattern::Wildcard { .. }
            | &Pattern::ConstInt { .. }
            | &Pattern::ConstPrim { .. } => self.clone(),
            &Pattern::MacroArg { index, .. } => macro_args[index].clone(),
        }
    }

    pub fn pos(&self) -> Pos {
        match self {
            &Pattern::ConstInt { pos, .. }
            | &Pattern::ConstPrim { pos, .. }
            | &Pattern::And { pos, .. }
            | &Pattern::Term { pos, .. }
            | &Pattern::BindPattern { pos, .. }
            | &Pattern::Var { pos, .. }
            | &Pattern::Wildcard { pos, .. }
            | &Pattern::MacroArg { pos, .. } => pos,
        }
    }
}

/// A pattern in a term argument. Adds "evaluated expression" to kinds
/// of patterns in addition to all options in `Pattern`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TermArgPattern {
    /// A regular pattern that must match the existing value in the term's argument.
    Pattern(Pattern),
    /// An expression that is evaluated during the match phase and can
    /// be given into an extractor. This is essentially a limited form
    /// of unification or bidirectional argument flow (a la Prolog):
    /// we can pass an arg *into* an extractor rather than getting the
    /// arg *out of* it.
    Expr(Expr),
}

impl TermArgPattern {
    fn make_macro_template(&self, args: &[Ident]) -> TermArgPattern {
        log::trace!("repplace_macro_args: {:?} with {:?}", self, args);
        match self {
            &TermArgPattern::Pattern(ref pat) => {
                TermArgPattern::Pattern(pat.make_macro_template(args))
            }
            &TermArgPattern::Expr(_) => self.clone(),
        }
    }

    fn subst_macro_args(&self, args: &[Pattern]) -> TermArgPattern {
        match self {
            &TermArgPattern::Pattern(ref pat) => {
                TermArgPattern::Pattern(pat.subst_macro_args(args))
            }
            &TermArgPattern::Expr(_) => self.clone(),
        }
    }
}

/// An expression: the right-hand side of a rule.
///
/// Note that this *almost* looks like a core Lisp or lambda calculus,
/// except that there is no abstraction (lambda). This first-order
/// limit is what makes it analyzable.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Expr {
    /// A term: `(sym args...)`.
    Term {
        sym: Ident,
        args: Vec<Expr>,
        pos: Pos,
    },
    /// A variable use.
    Var { name: Ident, pos: Pos },
    /// A constant integer.
    ConstInt { val: i64, pos: Pos },
    /// A constant of some other primitive type.
    ConstPrim { val: Ident, pos: Pos },
    /// The `(let ((var ty val)*) body)` form.
    Let {
        defs: Vec<LetDef>,
        body: Box<Expr>,
        pos: Pos,
    },
}

impl Expr {
    pub fn pos(&self) -> Pos {
        match self {
            &Expr::Term { pos, .. }
            | &Expr::Var { pos, .. }
            | &Expr::ConstInt { pos, .. }
            | &Expr::ConstPrim { pos, .. }
            | &Expr::Let { pos, .. } => pos,
        }
    }
}

/// One variable locally bound in a `(let ...)` expression.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LetDef {
    pub var: Ident,
    pub ty: Ident,
    pub val: Box<Expr>,
    pub pos: Pos,
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
        /// Poliarity of args: whether values are inputs or outputs to
        /// the external extractor function. This is a sort of
        /// statically-defined approximation to Prolog-style
        /// unification; we allow for the same flexible directionality
        /// but fix it at DSL-definition time. By default, every arg
        /// is an *output* from the extractor (and the 'retval", or
        /// more precisely the term value that we are extracting, is
        /// an "input").
        arg_polarity: Option<Vec<ArgPolarity>>,
        /// Infallibility: if an external extractor returns `(T1, T2,
        /// ...)` rather than `Option<(T1, T2, ...)>`, and hence can
        /// never fail, it is declared as such and allows for slightly
        /// better code to be generated.
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
    /// An external constant: `(const $IDENT type)` form.
    Const { name: Ident, ty: Ident, pos: Pos },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgPolarity {
    /// An arg that must be given an Expr in the pattern and passes
    /// data *to* the extractor op.
    Input,
    /// An arg that must be given a regular pattern (not Expr) and
    /// receives data *from* the extractor op.
    Output,
}
