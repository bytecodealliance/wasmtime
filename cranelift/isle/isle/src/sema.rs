//! Semantic analysis.
//!
//! This module primarily contains the type environment and term environment.
//!
//! The type environment is constructed by analyzing an input AST. The type
//! environment records the types used in the input source and the types of our
//! various rules and symbols. ISLE's type system is intentionally easy to
//! check, only requires a single pass over the AST, and doesn't require any
//! unification or anything like that.
//!
//! The term environment is constructed from both the AST and type
//! envionment. It is sort of a typed and reorganized AST that more directly
//! reflects ISLE semantics than the input ISLE source code (where as the AST is
//! the opposite).

use crate::ast;
use crate::error::*;
use crate::lexer::Pos;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;

declare_id!(
    /// The id of an interned symbol.
    Sym
);
declare_id!(
    /// The id of an interned type inside the `TypeEnv`.
    TypeId
);
declare_id!(
    /// The id of a variant inside an enum.
    VariantId
);
declare_id!(
    /// The id of a field inside a variant.
    FieldId
);
declare_id!(
    /// The id of an interned term inside the `TermEnv`.
    TermId
);
declare_id!(
    /// The id of an interned rule inside the `TermEnv`.
    RuleId
);
declare_id!(
    /// The id of a bound variable inside a `Bindings`.
    VarId
);

/// The type environment.
///
/// Keeps track of which symbols and rules have which types.
#[derive(Clone, Debug)]
pub struct TypeEnv {
    /// Arena of input ISLE source filenames.
    ///
    /// We refer to these indirectly through the `Pos::file` indices.
    pub filenames: Vec<Arc<str>>,

    /// Arena of input ISLE source contents.
    ///
    /// We refer to these indirectly through the `Pos::file` indices.
    pub file_texts: Vec<Arc<str>>,

    /// Arena of interned symbol names.
    ///
    /// Referred to indirectly via `Sym` indices.
    pub syms: Vec<String>,

    /// Map of already-interned symbol names to their `Sym` ids.
    pub sym_map: BTreeMap<String, Sym>,

    /// Arena of type definitions.
    ///
    /// Referred to indirectly via `TypeId`s.
    pub types: Vec<Type>,

    /// A map from a type name symbol to its `TypeId`.
    pub type_map: BTreeMap<Sym, TypeId>,

    /// The types of constant symbols.
    pub const_types: BTreeMap<Sym, TypeId>,

    /// Type errors that we've found so far during type checking.
    pub errors: Vec<Error>,
}

/// A type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    /// A primitive, `Copy` type.
    ///
    /// These are always defined externally, and we allow literals of these
    /// types to pass through from ISLE source code to the emitted Rust code.
    Primitive(TypeId, Sym, Pos),

    /// A sum type.
    ///
    /// Note that enums with only one variant are equivalent to a "struct".
    Enum {
        /// The name of this enum.
        name: Sym,
        /// This `enum`'s type id.
        id: TypeId,
        /// Is this `enum` defined in external Rust code?
        ///
        /// If so, ISLE will not emit a definition for it. If not, then it will
        /// emit a Rust definition for it.
        is_extern: bool,
        /// Whether this type should *not* derive `Debug`.
        ///
        /// Incompatible with `is_extern`.
        is_nodebug: bool,
        /// The different variants for this enum.
        variants: Vec<Variant>,
        /// The ISLE source position where this `enum` is defined.
        pos: Pos,
    },
}

impl Type {
    /// Get the name of this `Type`.
    pub fn name<'a>(&self, tyenv: &'a TypeEnv) -> &'a str {
        match self {
            Self::Primitive(_, name, _) | Self::Enum { name, .. } => &tyenv.syms[name.index()],
        }
    }

    /// Get the position where this type was defined.
    pub fn pos(&self) -> Pos {
        match self {
            Self::Primitive(_, _, pos) | Self::Enum { pos, .. } => *pos,
        }
    }

    /// Is this a primitive type?
    pub fn is_prim(&self) -> bool {
        matches!(self, Type::Primitive(..))
    }
}

/// A variant of an enum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    /// The name of this variant.
    pub name: Sym,

    /// The full, prefixed-with-the-enum's-name name of this variant.
    ///
    /// E.g. if the enum is `Foo` and this variant is `Bar`, then the
    /// `fullname` is `Foo.Bar`.
    pub fullname: Sym,

    /// The id of this variant, i.e. the index of this variant within its
    /// enum's `Type::Enum::variants`.
    pub id: VariantId,

    /// The data fields of this enum variant.
    pub fields: Vec<Field>,
}

/// A field of a `Variant`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    /// The name of this field.
    pub name: Sym,
    /// This field's id.
    pub id: FieldId,
    /// The type of this field.
    pub ty: TypeId,
}

/// The term environment.
///
/// This is sort of a typed and reorganized AST that more directly reflects ISLE
/// semantics than the input ISLE source code (where as the AST is the
/// opposite).
#[derive(Clone, Debug)]
pub struct TermEnv {
    /// Arena of interned terms defined in this ISLE program.
    ///
    /// This is indexed by `TermId`.
    pub terms: Vec<Term>,

    /// A map from am interned `Term`'s name to its `TermId`.
    pub term_map: BTreeMap<Sym, TermId>,

    /// Arena of interned rules defined in this ISLE program.
    ///
    /// This is indexed by `RuleId`.
    pub rules: Vec<Rule>,
}

/// A term.
///
/// Maps parameter types to result types if this is a constructor term, or
/// result types to parameter types if this is an extractor term. Or both if
/// this term can be either a constructor or an extractor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Term {
    /// This term's id.
    pub id: TermId,
    /// The source position where this term was declared.
    pub decl_pos: Pos,
    /// The name of this term.
    pub name: Sym,
    /// The parameter types to this term.
    pub arg_tys: Vec<TypeId>,
    /// The result types of this term.
    pub ret_ty: TypeId,
    /// The kind of this term.
    pub kind: TermKind,
}

/// The kind of a term.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TermKind {
    /// An enum variant constructor or extractor.
    EnumVariant {
        /// Which variant of the enum: e.g. for enum type `A` if a term is
        /// `(A.A1 ...)` then the variant ID corresponds to `A1`.
        variant: VariantId,
    },
    /// A term declared via a `(decl ...)` form.
    Decl {
        /// The kind of this term's constructor, if any.
        constructor_kind: Option<ConstructorKind>,
        /// The kind of this term's extractor, if any.
        extractor_kind: Option<ExtractorKind>,
    },
}

/// The kind of a constructor for a term.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstructorKind {
    /// A term with "internal" rules that work in the forward direction. Becomes
    /// a compiled Rust function in the generated code.
    InternalConstructor,
    /// A term defined solely by an external constructor function.
    ExternalConstructor {
        /// The external name of the constructor function.
        name: Sym,
    },
}

/// The kind of an extractor for a term.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtractorKind {
    /// A term that defines an "extractor macro" in the LHS of a pattern. Its
    /// arguments take patterns and are simply substituted with the given
    /// patterns when used.
    InternalExtractor {
        /// This extractor's pattern.
        template: ast::Pattern,
    },
    /// A term defined solely by an external extractor function.
    ExternalExtractor {
        /// The external name of the extractor function.
        name: Sym,
        /// Which arguments of the extractor are inputs and which are outputs?
        arg_polarity: Vec<ArgPolarity>,
        /// Is the external extractor infallible?
        infallible: bool,
        /// The position where this external extractor was declared.
        pos: Pos,
    },
}

pub use crate::ast::ArgPolarity;

/// An external function signature.
#[derive(Clone, Debug)]
pub struct ExternalSig {
    /// The name of the external function.
    pub func_name: String,
    /// The name of the external function, prefixed with the context trait.
    pub full_name: String,
    /// The types of this function signature's parameters.
    pub param_tys: Vec<TypeId>,
    /// The types of this function signature's results.
    pub ret_tys: Vec<TypeId>,
    /// Whether this signature is infallible or not.
    pub infallible: bool,
}

impl Term {
    /// Get this term's type.
    pub fn ty(&self) -> TypeId {
        self.ret_ty
    }

    /// Is this term an enum variant?
    pub fn is_enum_variant(&self) -> bool {
        matches!(self.kind, TermKind::EnumVariant { .. })
    }

    /// Does this term have a constructor?
    pub fn has_constructor(&self) -> bool {
        matches!(
            self.kind,
            TermKind::EnumVariant { .. }
                | TermKind::Decl {
                    constructor_kind: Some(_),
                    ..
                }
        )
    }

    /// Does this term have an extractor?
    pub fn has_extractor(&self) -> bool {
        matches!(
            self.kind,
            TermKind::EnumVariant { .. }
                | TermKind::Decl {
                    extractor_kind: Some(_),
                    ..
                }
        )
    }

    /// Is this term's extractor external?
    pub fn has_external_extractor(&self) -> bool {
        matches!(
            self.kind,
            TermKind::Decl {
                extractor_kind: Some(ExtractorKind::ExternalExtractor { .. }),
                ..
            }
        )
    }

    /// Is this term's constructor external?
    pub fn has_external_constructor(&self) -> bool {
        matches!(
            self.kind,
            TermKind::Decl {
                constructor_kind: Some(ConstructorKind::ExternalConstructor { .. }),
                ..
            }
        )
    }

    /// Get this term's extractor's external function signature, if any.
    pub fn extractor_sig(&self, tyenv: &TypeEnv) -> Option<ExternalSig> {
        match &self.kind {
            TermKind::Decl {
                extractor_kind:
                    Some(ExtractorKind::ExternalExtractor {
                        name,
                        ref arg_polarity,
                        infallible,
                        ..
                    }),
                ..
            } => {
                let mut arg_tys = vec![];
                let mut ret_tys = vec![];
                arg_tys.push(self.ret_ty);
                for (&arg, polarity) in self.arg_tys.iter().zip(arg_polarity.iter()) {
                    match polarity {
                        &ArgPolarity::Input => {
                            arg_tys.push(arg);
                        }
                        &ArgPolarity::Output => {
                            ret_tys.push(arg);
                        }
                    }
                }
                Some(ExternalSig {
                    func_name: tyenv.syms[name.index()].clone(),
                    full_name: format!("C::{}", tyenv.syms[name.index()]),
                    param_tys: arg_tys,
                    ret_tys,
                    infallible: *infallible,
                })
            }
            _ => None,
        }
    }

    /// Get this term's constructor's external function signature, if any.
    pub fn constructor_sig(&self, tyenv: &TypeEnv) -> Option<ExternalSig> {
        match &self.kind {
            TermKind::Decl {
                constructor_kind: Some(ConstructorKind::ExternalConstructor { name }),
                ..
            } => Some(ExternalSig {
                func_name: tyenv.syms[name.index()].clone(),
                full_name: format!("C::{}", tyenv.syms[name.index()]),
                param_tys: self.arg_tys.clone(),
                ret_tys: vec![self.ret_ty],
                infallible: true,
            }),
            TermKind::Decl {
                constructor_kind: Some(ConstructorKind::InternalConstructor { .. }),
                ..
            } => {
                let name = format!("constructor_{}", tyenv.syms[self.name.index()]);
                Some(ExternalSig {
                    func_name: name.clone(),
                    full_name: name,
                    param_tys: self.arg_tys.clone(),
                    ret_tys: vec![self.ret_ty],
                    infallible: false,
                })
            }
            _ => None,
        }
    }
}

/// A term rewrite rule.
#[derive(Clone, Debug)]
pub struct Rule {
    /// This rule's id.
    pub id: RuleId,
    /// The left-hand side pattern that this rule matches.
    pub lhs: Pattern,
    /// The right-hand side expression that this rule evaluates upon successful
    /// match.
    pub rhs: Expr,
    /// The priority of this rule, if any.
    pub prio: Option<i64>,
    /// The source position where this rule is defined.
    pub pos: Pos,
}

/// A left-hand side pattern of some rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pattern {
    /// Bind a variable of the given type from the current value.
    ///
    /// Keep matching on the value with the subpattern.
    BindPattern(TypeId, VarId, Box<Pattern>),

    /// Match the current value against an already bound variable with the given
    /// type.
    Var(TypeId, VarId),

    /// Match the current value against a constant integer of the given integer
    /// type.
    ConstInt(TypeId, i64),

    /// Match the current value against a constant primitive value of the given
    /// primitive type.
    ConstPrim(TypeId, Sym),

    /// Match the current value against the given extractor term with the given
    /// arguments.
    Term(TypeId, TermId, Vec<TermArgPattern>),

    /// Match anything of the given type successfully.
    Wildcard(TypeId),

    /// Match all of the following patterns of the given type.
    And(TypeId, Vec<Pattern>),
}

/// Arguments to a term inside a pattern (i.e. an extractor).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TermArgPattern {
    /// A pattern to match sub-values (i.e. the extractor's results) against.
    Pattern(Pattern),
    /// An expression to generate a value that is passed into the extractor.
    Expr(Expr),
}

/// A right-hand side expression of some rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    /// Invoke this term constructor with the given arguments.
    Term(TypeId, TermId, Vec<Expr>),
    /// Get the value of a variable that was bound in the left-hand side.
    Var(TypeId, VarId),
    /// Get a constant integer.
    ConstInt(TypeId, i64),
    /// Get a constant primitive.
    ConstPrim(TypeId, Sym),
    /// Evaluate the nested expressions and bind their results to the given
    /// variables, then evaluate the body expression.
    Let {
        /// The type of the result of this let expression.
        ty: TypeId,
        /// The expressions that are evaluated and bound to the given variables.
        bindings: Vec<(VarId, TypeId, Box<Expr>)>,
        /// The body expression that is evaluated after the bindings.
        body: Box<Expr>,
    },
}

impl Pattern {
    /// Get this pattern's type.
    pub fn ty(&self) -> TypeId {
        match self {
            &Self::BindPattern(t, ..) => t,
            &Self::Var(t, ..) => t,
            &Self::ConstInt(t, ..) => t,
            &Self::ConstPrim(t, ..) => t,
            &Self::Term(t, ..) => t,
            &Self::Wildcard(t, ..) => t,
            &Self::And(t, ..) => t,
        }
    }

    /// Get the root term of this pattern, if any.
    pub fn root_term(&self) -> Option<TermId> {
        match self {
            &Pattern::Term(_, term, _) => Some(term),
            &Pattern::BindPattern(_, _, ref subpat) => subpat.root_term(),
            _ => None,
        }
    }
}

impl Expr {
    /// Get this expression's type.
    pub fn ty(&self) -> TypeId {
        match self {
            &Self::Term(t, ..) => t,
            &Self::Var(t, ..) => t,
            &Self::ConstInt(t, ..) => t,
            &Self::ConstPrim(t, ..) => t,
            &Self::Let { ty: t, .. } => t,
        }
    }
}

/// Given an `Option<T>`, unwrap the inner `T` value, or `continue` if it is
/// `None`.
///
/// Useful for when we encountered an error earlier in our analysis but kept
/// going to find more errors, and now we've run into some missing data that
/// would have been filled in if we didn't hit that original error, but we want
/// to keep going to find more errors.
macro_rules! unwrap_or_continue {
    ($e:expr) => {
        match $e {
            Some(x) => x,
            None => continue,
        }
    };
}

impl TypeEnv {
    /// Construct the type environment from the AST.
    pub fn from_ast(defs: &ast::Defs) -> Result<TypeEnv> {
        let mut tyenv = TypeEnv {
            filenames: defs.filenames.clone(),
            file_texts: defs.file_texts.clone(),
            syms: vec![],
            sym_map: BTreeMap::new(),
            types: vec![],
            type_map: BTreeMap::new(),
            const_types: BTreeMap::new(),
            errors: vec![],
        };

        // Traverse defs, assigning type IDs to type names. We'll fill
        // in types on a second pass.
        for def in &defs.defs {
            match def {
                &ast::Def::Type(ref td) => {
                    let tid = TypeId(tyenv.type_map.len());
                    let name = tyenv.intern_mut(&td.name);

                    if let Some(existing) = tyenv.type_map.get(&name).copied() {
                        tyenv.report_error(
                            td.pos,
                            format!("Type with name '{}' defined more than once", td.name.0),
                        );
                        let pos = unwrap_or_continue!(tyenv.types.get(existing.index())).pos();
                        tyenv.report_error(
                            pos,
                            format!("Type with name '{}' already defined here", td.name.0),
                        );
                        continue;
                    }

                    tyenv.type_map.insert(name, tid);
                }
                _ => {}
            }
        }

        // Now lower AST nodes to type definitions, raising errors
        // where typenames of fields are undefined or field names are
        // duplicated.
        let mut tid = 0;
        for def in &defs.defs {
            match def {
                &ast::Def::Type(ref td) => {
                    let ty = unwrap_or_continue!(tyenv.type_from_ast(TypeId(tid), td));
                    tyenv.types.push(ty);
                    tid += 1;
                }
                _ => {}
            }
        }

        // Now collect types for extern constants.
        for def in &defs.defs {
            match def {
                &ast::Def::Extern(ast::Extern::Const {
                    ref name,
                    ref ty,
                    pos,
                }) => {
                    let ty = tyenv.intern_mut(ty);
                    let ty = match tyenv.type_map.get(&ty) {
                        Some(ty) => *ty,
                        None => {
                            tyenv.report_error(pos, "Unknown type for constant");
                            continue;
                        }
                    };
                    let name = tyenv.intern_mut(name);
                    tyenv.const_types.insert(name, ty);
                }
                _ => {}
            }
        }

        tyenv.return_errors()?;

        Ok(tyenv)
    }

    fn return_errors(&mut self) -> Result<()> {
        match self.errors.len() {
            0 => Ok(()),
            1 => Err(self.errors.pop().unwrap()),
            _ => Err(Error::Errors(std::mem::take(&mut self.errors))),
        }
    }

    fn type_from_ast(&mut self, tid: TypeId, ty: &ast::Type) -> Option<Type> {
        let name = self.intern(&ty.name).unwrap();
        match &ty.ty {
            &ast::TypeValue::Primitive(ref id, ..) => {
                if ty.is_nodebug {
                    self.report_error(ty.pos, "primitive types cannot be marked `nodebug`");
                    return None;
                }
                if ty.is_extern {
                    self.report_error(ty.pos, "primitive types cannot be marked `extern`");
                    return None;
                }
                Some(Type::Primitive(tid, self.intern_mut(id), ty.pos))
            }
            &ast::TypeValue::Enum(ref ty_variants, ..) => {
                if ty.is_extern && ty.is_nodebug {
                    self.report_error(ty.pos, "external types cannot be marked `nodebug`");
                    return None;
                }

                let mut variants = vec![];
                for variant in ty_variants {
                    let combined_ident =
                        ast::Ident(format!("{}.{}", ty.name.0, variant.name.0), variant.name.1);
                    let fullname = self.intern_mut(&combined_ident);
                    let name = self.intern_mut(&variant.name);
                    let id = VariantId(variants.len());
                    if variants.iter().any(|v: &Variant| v.name == name) {
                        self.report_error(
                            variant.pos,
                            format!("Duplicate variant name in type: '{}'", variant.name.0),
                        );
                        return None;
                    }
                    let mut fields = vec![];
                    for field in &variant.fields {
                        let field_name = self.intern_mut(&field.name);
                        if fields.iter().any(|f: &Field| f.name == field_name) {
                            self.report_error(
                                field.pos,
                                format!(
                                    "Duplicate field name '{}' in variant '{}' of type",
                                    field.name.0, variant.name.0
                                ),
                            );
                            return None;
                        }
                        let field_ty = self.intern_mut(&field.ty);
                        let field_tid = match self.type_map.get(&field_ty) {
                            Some(tid) => *tid,
                            None => {
                                self.report_error(
                                    field.ty.1,
                                    format!(
                                        "Unknown type '{}' for field '{}' in variant '{}'",
                                        field.ty.0, field.name.0, variant.name.0
                                    ),
                                );
                                return None;
                            }
                        };
                        fields.push(Field {
                            name: field_name,
                            id: FieldId(fields.len()),
                            ty: field_tid,
                        });
                    }
                    variants.push(Variant {
                        name,
                        fullname,
                        id,
                        fields,
                    });
                }
                Some(Type::Enum {
                    name,
                    id: tid,
                    is_extern: ty.is_extern,
                    is_nodebug: ty.is_nodebug,
                    variants,
                    pos: ty.pos,
                })
            }
        }
    }

    fn error(&self, pos: Pos, msg: impl Into<String>) -> Error {
        let e = Error::TypeError {
            msg: msg.into(),
            src: Source::new(
                self.filenames[pos.file].clone(),
                self.file_texts[pos.file].clone(),
            ),
            span: miette::SourceSpan::from((pos.offset, 1)),
        };
        log::trace!("{}", e);
        e
    }

    fn report_error(&mut self, pos: Pos, msg: impl Into<String>) {
        let err = self.error(pos, msg);
        self.errors.push(err);
    }

    fn intern_mut(&mut self, ident: &ast::Ident) -> Sym {
        if let Some(s) = self.sym_map.get(&ident.0).copied() {
            s
        } else {
            let s = Sym(self.syms.len());
            self.syms.push(ident.0.clone());
            self.sym_map.insert(ident.0.clone(), s);
            s
        }
    }

    fn intern(&self, ident: &ast::Ident) -> Option<Sym> {
        self.sym_map.get(&ident.0).cloned()
    }
}

#[derive(Clone, Debug)]
struct Bindings {
    next_var: usize,
    vars: Vec<BoundVar>,
}

#[derive(Clone, Debug)]
struct BoundVar {
    name: Sym,
    id: VarId,
    ty: TypeId,
}

impl TermEnv {
    /// Construct the term environment from the AST and the type environment.
    pub fn from_ast(tyenv: &mut TypeEnv, defs: &ast::Defs) -> Result<TermEnv> {
        let mut env = TermEnv {
            terms: vec![],
            term_map: BTreeMap::new(),
            rules: vec![],
        };

        env.collect_term_sigs(tyenv, defs);
        env.collect_enum_variant_terms(tyenv);
        tyenv.return_errors()?;
        env.collect_constructors(tyenv, defs);
        env.collect_extractor_templates(tyenv, defs);
        tyenv.return_errors()?;
        env.collect_rules(tyenv, defs);
        env.check_for_undefined_decls(tyenv, defs);
        env.check_for_expr_terms_without_constructors(tyenv, defs);
        tyenv.return_errors()?;

        Ok(env)
    }

    fn collect_term_sigs(&mut self, tyenv: &mut TypeEnv, defs: &ast::Defs) {
        for def in &defs.defs {
            match def {
                &ast::Def::Decl(ref decl) => {
                    let tid = TermId(self.terms.len());
                    let name = tyenv.intern_mut(&decl.term);
                    if let Some(tid) = self.term_map.get(&name) {
                        tyenv.report_error(
                            decl.pos,
                            format!("Duplicate decl for '{}'", decl.term.0),
                        );
                        tyenv.report_error(
                            self.terms[tid.index()].decl_pos,
                            format!("Duplicate decl for '{}'", decl.term.0),
                        );
                    }
                    self.term_map.insert(name, tid);

                    let arg_tys = decl
                        .arg_tys
                        .iter()
                        .map(|id| {
                            let sym = tyenv.intern_mut(id);
                            tyenv.type_map.get(&sym).cloned().ok_or_else(|| {
                                tyenv.report_error(id.1, format!("Unknown arg type: '{}'", id.0));
                                ()
                            })
                        })
                        .collect::<std::result::Result<Vec<_>, _>>();
                    let arg_tys = match arg_tys {
                        Ok(a) => a,
                        Err(_) => {
                            continue;
                        }
                    };
                    let ret_ty = {
                        let sym = tyenv.intern_mut(&decl.ret_ty);
                        match tyenv.type_map.get(&sym).cloned() {
                            Some(t) => t,
                            None => {
                                tyenv.report_error(
                                    decl.ret_ty.1,
                                    format!("Unknown return type: '{}'", decl.ret_ty.0),
                                );
                                continue;
                            }
                        }
                    };

                    self.terms.push(Term {
                        id: tid,
                        decl_pos: decl.pos,
                        name,
                        arg_tys,
                        ret_ty,
                        kind: TermKind::Decl {
                            constructor_kind: None,
                            extractor_kind: None,
                        },
                    });
                }
                _ => {}
            }
        }
    }

    fn collect_enum_variant_terms(&mut self, tyenv: &mut TypeEnv) {
        'types: for i in 0..tyenv.types.len() {
            let ty = &tyenv.types[i];
            match ty {
                &Type::Enum {
                    pos,
                    id,
                    ref variants,
                    ..
                } => {
                    for variant in variants {
                        if self.term_map.contains_key(&variant.fullname) {
                            let variant_name = tyenv.syms[variant.fullname.index()].clone();
                            tyenv.report_error(
                                pos,
                                format!("Duplicate enum variant constructor: '{}'", variant_name,),
                            );
                            continue 'types;
                        }
                        let tid = TermId(self.terms.len());
                        let arg_tys = variant.fields.iter().map(|fld| fld.ty).collect::<Vec<_>>();
                        let ret_ty = id;
                        self.terms.push(Term {
                            id: tid,
                            decl_pos: pos,
                            name: variant.fullname,
                            arg_tys,
                            ret_ty,
                            kind: TermKind::EnumVariant {
                                variant: variant.id,
                            },
                        });
                        self.term_map.insert(variant.fullname, tid);
                    }
                }
                _ => {}
            }
        }
    }

    fn collect_constructors(&mut self, tyenv: &mut TypeEnv, defs: &ast::Defs) {
        for def in &defs.defs {
            log::debug!("collect_constructors from def: {:?}", def);
            match def {
                &ast::Def::Rule(ref rule) => {
                    let pos = rule.pos;
                    let term = match rule.pattern.root_term() {
                        Some(t) => t,
                        None => {
                            tyenv.report_error(
                                pos,
                                "Rule does not have a term at the LHS root".to_string(),
                            );
                            continue;
                        }
                    };
                    let sym = tyenv.intern_mut(&term);
                    let term = match self.term_map.get(&sym) {
                        Some(&tid) => tid,
                        None => {
                            tyenv
                                .report_error(pos, "Rule LHS root term is not defined".to_string());
                            continue;
                        }
                    };
                    let termdata = &mut self.terms[term.index()];
                    match &mut termdata.kind {
                        TermKind::Decl {
                            constructor_kind, ..
                        } => {
                            match constructor_kind {
                                None => {
                                    *constructor_kind = Some(ConstructorKind::InternalConstructor);
                                }
                                Some(ConstructorKind::InternalConstructor) => {
                                    // OK, no error; multiple rules can apply to
                                    // one internal constructor term.
                                }
                                Some(ConstructorKind::ExternalConstructor { .. }) => {
                                    tyenv.report_error(
                                        pos,
                                        "Rule LHS root term is incorrect kind; cannot \
                                         be external constructor"
                                            .to_string(),
                                    );
                                    continue;
                                }
                            }
                        }
                        TermKind::EnumVariant { .. } => {
                            tyenv.report_error(
                                pos,
                                "Rule LHS root term is incorrect kind; cannot be enum variant"
                                    .to_string(),
                            );
                            continue;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn collect_extractor_templates(&mut self, tyenv: &mut TypeEnv, defs: &ast::Defs) {
        let mut extractor_call_graph = BTreeMap::new();

        for def in &defs.defs {
            if let &ast::Def::Extractor(ref ext) = def {
                let sym = tyenv.intern_mut(&ext.term);
                let term = match self.term_map.get(&sym) {
                    Some(x) => x,
                    None => {
                        tyenv.report_error(
                            ext.pos,
                            "Extractor macro body definition on a non-existent term".to_string(),
                        );
                        return;
                    }
                };

                let template = ext.template.make_macro_template(&ext.args[..]);
                log::trace!("extractor def: {:?} becomes template {:?}", def, template);

                let mut callees = BTreeSet::new();
                template.terms(&mut |pos, t| {
                    let t = tyenv.intern_mut(t);
                    callees.insert(t);

                    if !self.term_map.contains_key(&t) {
                        tyenv.report_error(
                            pos,
                            format!(
                                "`{}` extractor definition references unknown term `{}`",
                                ext.term.0,
                                tyenv.syms[t.index()]
                            ),
                        );
                    }
                });
                extractor_call_graph.insert(sym, callees);

                let termdata = &mut self.terms[term.index()];
                match &mut termdata.kind {
                    TermKind::EnumVariant { .. } => {
                        tyenv.report_error(
                            ext.pos,
                            "Extractor macro body defined on term of incorrect kind; cannot be an \
                             enum variant",
                        );
                        continue;
                    }
                    TermKind::Decl { extractor_kind, .. } => match extractor_kind {
                        None => {
                            *extractor_kind = Some(ExtractorKind::InternalExtractor { template });
                        }
                        Some(ext_kind) => {
                            tyenv.report_error(
                                ext.pos,
                                "Duplicate extractor definition".to_string(),
                            );
                            let pos = match ext_kind {
                                ExtractorKind::InternalExtractor { template } => template.pos(),
                                ExtractorKind::ExternalExtractor { pos, .. } => *pos,
                            };
                            tyenv.report_error(
                                pos,
                                "Extractor was already defined here".to_string(),
                            );
                            continue;
                        }
                    },
                }
            }
        }

        // Check for cycles in the extractor call graph.
        let mut seen = BTreeSet::new();
        let mut stack = vec![];
        'outer: for root in extractor_call_graph.keys().copied() {
            seen.clear();
            stack.clear();
            stack.push((root, vec![root]));

            while let Some((caller, path)) = stack.pop() {
                let is_new = seen.insert(caller);
                if is_new {
                    if let Some(callees) = extractor_call_graph.get(&caller) {
                        stack.extend(callees.iter().map(|callee| {
                            let mut path = path.clone();
                            path.push(*callee);
                            (*callee, path)
                        }));
                    }
                } else {
                    let term = match self.term_map.get(&caller) {
                        Some(t) => t,
                        None => {
                            // Some other error must have already been recorded
                            // if we don't have the caller's term data.
                            assert!(!tyenv.errors.is_empty());
                            continue 'outer;
                        }
                    };
                    let pos = match &self.terms[term.index()].kind {
                        TermKind::Decl {
                            extractor_kind: Some(ExtractorKind::InternalExtractor { template }),
                            ..
                        } => template.pos(),
                        _ => {
                            // Again, there must have already been errors
                            // recorded.
                            assert!(!tyenv.errors.is_empty());
                            continue 'outer;
                        }
                    };

                    let path: Vec<_> = path
                        .iter()
                        .map(|sym| tyenv.syms[sym.index()].as_str())
                        .collect();
                    let msg = format!(
                        "`{}` extractor definition is recursive: {}",
                        tyenv.syms[root.index()],
                        path.join(" -> ")
                    );
                    tyenv.report_error(pos, msg);
                    continue 'outer;
                }
            }
        }
    }

    fn collect_rules(&mut self, tyenv: &mut TypeEnv, defs: &ast::Defs) {
        for def in &defs.defs {
            match def {
                &ast::Def::Rule(ref rule) => {
                    let pos = rule.pos;
                    let mut bindings = Bindings {
                        next_var: 0,
                        vars: vec![],
                    };

                    let rule_term = match rule.pattern.root_term() {
                        Some(name) => {
                            let sym = tyenv.intern_mut(name);
                            match self.term_map.get(&sym) {
                                Some(term) => *term,
                                None => {
                                    tyenv.report_error(
                                        pos,
                                        "Cannot define a rule for an unknown term".to_string(),
                                    );
                                    continue;
                                }
                            }
                        }
                        None => {
                            tyenv.report_error(
                                pos,
                                "Rule does not have a term at the root of its left-hand side"
                                    .to_string(),
                            );
                            continue;
                        }
                    };

                    let (lhs, ty) = unwrap_or_continue!(self.translate_pattern(
                        tyenv,
                        rule_term,
                        &rule.pattern,
                        None,
                        &mut bindings,
                        /* is_root = */ true,
                    ));
                    let rhs = unwrap_or_continue!(self.translate_expr(
                        tyenv,
                        &rule.expr,
                        ty,
                        &mut bindings
                    ));

                    let rid = RuleId(self.rules.len());
                    self.rules.push(Rule {
                        id: rid,
                        lhs,
                        rhs,
                        prio: rule.prio,
                        pos,
                    });
                }
                &ast::Def::Extern(ast::Extern::Constructor {
                    ref term,
                    ref func,
                    pos,
                }) => {
                    let term_sym = tyenv.intern_mut(term);
                    let func_sym = tyenv.intern_mut(func);
                    let term_id = match self.term_map.get(&term_sym) {
                        Some(term) => term,
                        None => {
                            tyenv.report_error(
                                pos,
                                format!("Constructor declared on undefined term '{}'", term.0),
                            );
                            continue;
                        }
                    };
                    let termdata = &mut self.terms[term_id.index()];
                    match &mut termdata.kind {
                        TermKind::Decl {
                            constructor_kind, ..
                        } => match constructor_kind {
                            None => {
                                *constructor_kind =
                                    Some(ConstructorKind::ExternalConstructor { name: func_sym });
                            }
                            Some(ConstructorKind::InternalConstructor) => {
                                tyenv.report_error(
                                    pos,
                                    format!(
                                        "External constructor declared on term that already has rules: {}",
                                        term.0,
                                    ),
                                );
                            }
                            Some(ConstructorKind::ExternalConstructor { .. }) => {
                                tyenv.report_error(
                                    pos,
                                    "Duplicate external constructor definition".to_string(),
                                );
                            }
                        },
                        TermKind::EnumVariant { .. } => {
                            tyenv.report_error(
                                pos,
                                format!(
                                    "External constructor cannot be defined on enum variant: {}",
                                    term.0,
                                ),
                            );
                        }
                    }
                }
                &ast::Def::Extern(ast::Extern::Extractor {
                    ref term,
                    ref func,
                    pos,
                    ref arg_polarity,
                    infallible,
                }) => {
                    let term_sym = tyenv.intern_mut(term);
                    let func_sym = tyenv.intern_mut(func);
                    let term_id = match self.term_map.get(&term_sym) {
                        Some(term) => term,
                        None => {
                            tyenv.report_error(
                                pos,
                                format!("Extractor declared on undefined term '{}'", term.0),
                            );
                            continue;
                        }
                    };

                    let termdata = &mut self.terms[term_id.index()];

                    let arg_polarity = if let Some(pol) = arg_polarity.as_ref() {
                        if pol.len() != termdata.arg_tys.len() {
                            tyenv.report_error(pos, "Incorrect number of argument-polarity directions in extractor definition".to_string());
                            continue;
                        }
                        pol.clone()
                    } else {
                        vec![ArgPolarity::Output; termdata.arg_tys.len()]
                    };

                    match &mut termdata.kind {
                        TermKind::Decl { extractor_kind, .. } => match extractor_kind {
                            None => {
                                *extractor_kind = Some(ExtractorKind::ExternalExtractor {
                                    name: func_sym,
                                    arg_polarity,
                                    infallible,
                                    pos,
                                });
                            }
                            Some(ExtractorKind::ExternalExtractor { pos: pos2, .. }) => {
                                tyenv.report_error(
                                    pos,
                                    "Duplicate external extractor definition".to_string(),
                                );
                                tyenv.report_error(
                                    *pos2,
                                    "External extractor already defined".to_string(),
                                );
                                continue;
                            }
                            Some(ExtractorKind::InternalExtractor { template }) => {
                                tyenv.report_error(
                                    pos,
                                    "Cannot define external extractor for term that already has an \
                                     internal extractor macro body defined"
                                        .to_string(),
                                );
                                tyenv.report_error(
                                    template.pos(),
                                    "Internal extractor macro body already defined".to_string(),
                                );
                                continue;
                            }
                        },
                        TermKind::EnumVariant { .. } => {
                            tyenv.report_error(
                                pos,
                                format!("Cannot define extractor for enum variant '{}'", term.0),
                            );
                            continue;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn check_for_undefined_decls(&self, tyenv: &mut TypeEnv, defs: &ast::Defs) {
        for def in &defs.defs {
            if let ast::Def::Decl(decl) = def {
                let sym = tyenv.intern_mut(&decl.term);
                let term = self.term_map[&sym];
                let term = &self.terms[term.index()];
                if !term.has_constructor() && !term.has_extractor() {
                    tyenv.report_error(
                        decl.pos,
                        format!(
                            "no rules, extractor, or external definition for declaration '{}'",
                            decl.term.0
                        ),
                    );
                }
            }
        }
    }

    fn check_for_expr_terms_without_constructors(&self, tyenv: &mut TypeEnv, defs: &ast::Defs) {
        for def in &defs.defs {
            if let ast::Def::Rule(rule) = def {
                rule.expr.terms(&mut |pos, ident| {
                    let sym = tyenv.intern_mut(ident);
                    let term = match self.term_map.get(&sym) {
                        None => {
                            debug_assert!(!tyenv.errors.is_empty());
                            return;
                        }
                        Some(t) => t,
                    };
                    let term = &self.terms[term.index()];
                    if !term.has_constructor() {
                        tyenv.report_error(
                            pos,
                            format!(
                                "term `{}` cannot be used in an expression because \
                                 it does not have a constructor",
                                ident.0
                            ),
                        )
                    }
                });
            }
        }
    }

    fn translate_pattern(
        &self,
        tyenv: &mut TypeEnv,
        rule_term: TermId,
        pat: &ast::Pattern,
        expected_ty: Option<TypeId>,
        bindings: &mut Bindings,
        is_root: bool,
    ) -> Option<(Pattern, TypeId)> {
        log::trace!("translate_pattern: {:?}", pat);
        log::trace!("translate_pattern: bindings = {:?}", bindings);
        match pat {
            // TODO: flag on primitive type decl indicating it's an integer type?
            &ast::Pattern::ConstInt { val, pos } => {
                let ty = match expected_ty {
                    Some(t) => t,
                    None => {
                        tyenv.report_error(pos, "Need an implied type for an integer constant");
                        return None;
                    }
                };
                if !tyenv.types[ty.index()].is_prim() {
                    tyenv.report_error(
                        pos,
                        format!(
                            "expected non-primitive type {}, but found integer literal '{}'",
                            tyenv.types[ty.index()].name(tyenv),
                            val,
                        ),
                    );
                }
                Some((Pattern::ConstInt(ty, val), ty))
            }
            &ast::Pattern::ConstPrim { ref val, pos } => {
                let val = tyenv.intern_mut(val);
                let const_ty = match tyenv.const_types.get(&val) {
                    Some(ty) => *ty,
                    None => {
                        tyenv.report_error(pos, "Unknown constant");
                        return None;
                    }
                };
                if expected_ty.is_some() && expected_ty != Some(const_ty) {
                    tyenv.report_error(pos, "Type mismatch for constant");
                }
                Some((Pattern::ConstPrim(const_ty, val), const_ty))
            }
            &ast::Pattern::Wildcard { pos } => {
                let ty = match expected_ty {
                    Some(t) => t,
                    None => {
                        tyenv.report_error(pos, "Need an implied type for a wildcard");
                        return None;
                    }
                };
                Some((Pattern::Wildcard(ty), ty))
            }
            &ast::Pattern::And { ref subpats, pos } => {
                let mut expected_ty = expected_ty;
                let mut children = vec![];
                for subpat in subpats {
                    let (subpat, ty) = unwrap_or_continue!(self.translate_pattern(
                        tyenv,
                        rule_term,
                        &*subpat,
                        expected_ty,
                        bindings,
                        /* is_root = */ false,
                    ));
                    expected_ty = expected_ty.or(Some(ty));
                    children.push(subpat);
                }
                if expected_ty.is_none() {
                    tyenv.report_error(pos, "No type for (and ...) form.".to_string());
                    return None;
                }
                let ty = expected_ty.unwrap();
                Some((Pattern::And(ty, children), ty))
            }
            &ast::Pattern::BindPattern {
                ref var,
                ref subpat,
                pos,
            } => {
                // Do the subpattern first so we can resolve the type for sure.
                let (subpat, ty) = self.translate_pattern(
                    tyenv,
                    rule_term,
                    &*subpat,
                    expected_ty,
                    bindings,
                    /* is_root = */ false,
                )?;

                let name = tyenv.intern_mut(var);
                if bindings.vars.iter().any(|bv| bv.name == name) {
                    tyenv.report_error(
                        pos,
                        format!("Re-bound variable name in LHS pattern: '{}'", var.0),
                    );
                    // Try to keep going.
                }
                let id = VarId(bindings.next_var);
                bindings.next_var += 1;
                log::trace!("binding var {:?}", var.0);
                bindings.vars.push(BoundVar { name, id, ty });

                Some((Pattern::BindPattern(ty, id, Box::new(subpat)), ty))
            }
            &ast::Pattern::Var { ref var, pos } => {
                // Look up the variable; it must already have been bound.
                let name = tyenv.intern_mut(var);
                let bv = match bindings.vars.iter().rev().find(|bv| bv.name == name) {
                    None => {
                        tyenv.report_error(
                            pos,
                            format!(
                                "Unknown variable '{}' in bound-var pattern '={}'",
                                var.0, var.0
                            ),
                        );
                        return None;
                    }
                    Some(bv) => bv,
                };
                let ty = match expected_ty {
                    None => bv.ty,
                    Some(expected_ty) if expected_ty == bv.ty => bv.ty,
                    Some(expected_ty) => {
                        tyenv.report_error(
                            pos,
                            format!(
                                "Mismatched types: pattern expects type '{}' but already-bound var '{}' has type '{}'",
                                tyenv.types[expected_ty.index()].name(tyenv),
                                var.0,
                                tyenv.types[bv.ty.index()].name(tyenv)));
                        bv.ty // Try to keep going for more errors.
                    }
                };
                Some((Pattern::Var(ty, bv.id), ty))
            }
            &ast::Pattern::Term {
                ref sym,
                ref args,
                pos,
            } => {
                let name = tyenv.intern_mut(&sym);
                // Look up the term.
                let tid = match self.term_map.get(&name) {
                    Some(t) => t,
                    None => {
                        tyenv.report_error(pos, format!("Unknown term in pattern: '{}'", sym.0));
                        return None;
                    }
                };

                // Get the return type and arg types. Verify the
                // expected type of this pattern, if any, against the
                // return type of the term.
                let ret_ty = self.terms[tid.index()].ret_ty;
                let ty = match expected_ty {
                    None => ret_ty,
                    Some(expected_ty) if expected_ty == ret_ty => ret_ty,
                    Some(expected_ty) => {
                        tyenv.report_error(
                            pos,
                            format!(
                                "Mismatched types: pattern expects type '{}' but term has return type '{}'",
                                tyenv.types[expected_ty.index()].name(tyenv),
                                tyenv.types[ret_ty.index()].name(tyenv)));
                        ret_ty // Try to keep going for more errors.
                    }
                };

                // Check that we have the correct argument count.
                if self.terms[tid.index()].arg_tys.len() != args.len() {
                    tyenv.report_error(
                        pos,
                        format!(
                            "Incorrect argument count for term '{}': got {}, expect {}",
                            sym.0,
                            args.len(),
                            self.terms[tid.index()].arg_tys.len()
                        ),
                    );
                }

                let termdata = &self.terms[tid.index()];

                match &termdata.kind {
                    TermKind::Decl {
                        constructor_kind: Some(ConstructorKind::InternalConstructor),
                        ..
                    } if is_root && *tid == rule_term => {
                        // This is just the `(foo ...)` pseudo-pattern inside a
                        // `(rule (foo ...) ...)` form. Just keep checking the
                        // sub-patterns.
                        for arg in args {
                            if let ast::TermArgPattern::Expr(e) = arg {
                                tyenv.report_error(
                                    e.pos(),
                                    "cannot use output-polarity expression with top-level rules"
                                        .to_string(),
                                );
                            }
                        }
                    }
                    TermKind::EnumVariant { .. } => {
                        for arg in args {
                            if let &ast::TermArgPattern::Expr(_) = arg {
                                tyenv.report_error(
                                    pos,
                                    format!(
                                        "Term in pattern '{}' cannot have an injected expr, because \
                                         it is an enum variant",
                                        sym.0
                                    )
                                );
                            }
                        }
                    }
                    TermKind::Decl {
                        extractor_kind:
                            Some(ExtractorKind::ExternalExtractor {
                                ref arg_polarity, ..
                            }),
                        ..
                    } => {
                        for (arg, pol) in args.iter().zip(arg_polarity.iter()) {
                            match (arg, pol) {
                                (&ast::TermArgPattern::Expr(..), &ArgPolarity::Input) => {}
                                (&ast::TermArgPattern::Expr(ref e), &ArgPolarity::Output) => {
                                    tyenv.report_error(
                                        e.pos(),
                                        "Expression used for output-polarity extractor arg"
                                            .to_string(),
                                    );
                                }
                                (_, &ArgPolarity::Output) => {}
                                (&ast::TermArgPattern::Pattern(ref p), &ArgPolarity::Input) => {
                                    tyenv.report_error(
                                        p.pos(),
                                        "Non-expression used in pattern but expression required for \
                                         input-polarity extractor arg"
                                            .to_string()
                                    );
                                }
                            }
                        }
                    }
                    TermKind::Decl {
                        extractor_kind: Some(ExtractorKind::InternalExtractor { ref template }),
                        ..
                    } => {
                        // Expand the extractor macro! We create a map
                        // from macro args to AST pattern trees and
                        // then evaluate the template with these
                        // substitutions.
                        let mut macro_args: Vec<ast::Pattern> = vec![];
                        for template_arg in args {
                            let sub_ast = match template_arg {
                                &ast::TermArgPattern::Pattern(ref pat) => pat.clone(),
                                &ast::TermArgPattern::Expr(_) => {
                                    tyenv.report_error(
                                        pos,
                                        "Cannot expand an extractor macro with an expression in a \
                                         macro argument"
                                            .to_string(),
                                    );
                                    return None;
                                }
                            };
                            macro_args.push(sub_ast.clone());
                        }
                        log::trace!("internal extractor macro args = {:?}", args);
                        let pat = template.subst_macro_args(&macro_args[..])?;
                        return self.translate_pattern(
                            tyenv,
                            rule_term,
                            &pat,
                            expected_ty,
                            bindings,
                            /* is_root = */ false,
                        );
                    }
                    TermKind::Decl {
                        extractor_kind: None,
                        ..
                    } => {
                        tyenv.report_error(
                            pos,
                            format!(
                                "Cannot use term '{}' that does not have a defined extractor in a \
                                 left-hand side pattern",
                                sym.0
                            ),
                        );
                    }
                }

                // Resolve subpatterns.
                let mut subpats = vec![];
                for (i, arg) in args.iter().enumerate() {
                    let term = unwrap_or_continue!(self.terms.get(tid.index()));
                    let arg_ty = unwrap_or_continue!(term.arg_tys.get(i).copied());
                    let (subpat, _) = unwrap_or_continue!(self.translate_pattern_term_arg(
                        tyenv,
                        rule_term,
                        pos,
                        arg,
                        Some(arg_ty),
                        bindings,
                    ));
                    subpats.push(subpat);
                }

                Some((Pattern::Term(ty, *tid, subpats), ty))
            }
            &ast::Pattern::MacroArg { .. } => unreachable!(),
        }
    }

    fn translate_pattern_term_arg(
        &self,
        tyenv: &mut TypeEnv,
        rule_term: TermId,
        pos: Pos,
        pat: &ast::TermArgPattern,
        expected_ty: Option<TypeId>,
        bindings: &mut Bindings,
    ) -> Option<(TermArgPattern, TypeId)> {
        match pat {
            &ast::TermArgPattern::Pattern(ref pat) => {
                let (subpat, ty) = self.translate_pattern(
                    tyenv,
                    rule_term,
                    pat,
                    expected_ty,
                    bindings,
                    /* is_root = */ false,
                )?;
                Some((TermArgPattern::Pattern(subpat), ty))
            }
            &ast::TermArgPattern::Expr(ref expr) => {
                if expected_ty.is_none() {
                    tyenv.report_error(
                        pos,
                        "Expression in pattern must have expected type".to_string(),
                    );
                    return None;
                }
                let ty = expected_ty.unwrap();
                let expr = self.translate_expr(tyenv, expr, expected_ty.unwrap(), bindings)?;
                Some((TermArgPattern::Expr(expr), ty))
            }
        }
    }

    fn translate_expr(
        &self,
        tyenv: &mut TypeEnv,
        expr: &ast::Expr,
        ty: TypeId,
        bindings: &mut Bindings,
    ) -> Option<Expr> {
        log::trace!("translate_expr: {:?}", expr);
        match expr {
            &ast::Expr::Term {
                ref sym,
                ref args,
                pos,
            } => {
                // Look up the term.
                let name = tyenv.intern_mut(&sym);
                // Look up the term.
                let tid = match self.term_map.get(&name) {
                    Some(t) => t,
                    None => {
                        tyenv.report_error(pos, format!("Unknown term in pattern: '{}'", sym.0));
                        return None;
                    }
                };

                // Get the return type and arg types. Verify the
                // expected type of this pattern, if any, against the
                // return type of the term.
                let ret_ty = self.terms[tid.index()].ret_ty;
                if ret_ty != ty {
                    tyenv.report_error(pos, format!("Mismatched types: expression expects type '{}' but term has return type '{}'", tyenv.types[ty.index()].name(tyenv), tyenv.types[ret_ty.index()].name(tyenv)));
                }

                // Check that we have the correct argument count.
                if self.terms[tid.index()].arg_tys.len() != args.len() {
                    tyenv.report_error(
                        pos,
                        format!(
                            "Incorrect argument count for term '{}': got {}, expect {}",
                            sym.0,
                            args.len(),
                            self.terms[tid.index()].arg_tys.len()
                        ),
                    );
                }

                // Resolve subexpressions.
                let mut subexprs = vec![];
                for (i, arg) in args.iter().enumerate() {
                    let term = unwrap_or_continue!(self.terms.get(tid.index()));
                    let arg_ty = unwrap_or_continue!(term.arg_tys.get(i).copied());
                    let subexpr =
                        unwrap_or_continue!(self.translate_expr(tyenv, arg, arg_ty, bindings));
                    subexprs.push(subexpr);
                }

                Some(Expr::Term(ty, *tid, subexprs))
            }
            &ast::Expr::Var { ref name, pos } => {
                let sym = tyenv.intern_mut(name);
                // Look through bindings, innermost (most recent) first.
                let bv = match bindings.vars.iter().rev().find(|b| b.name == sym) {
                    None => {
                        tyenv.report_error(pos, format!("Unknown variable '{}'", name.0));
                        return None;
                    }
                    Some(bv) => bv,
                };

                // Verify type.
                if bv.ty != ty {
                    tyenv.report_error(
                        pos,
                        format!(
                            "Variable '{}' has type {} but we need {} in context",
                            name.0,
                            tyenv.types[bv.ty.index()].name(tyenv),
                            tyenv.types[ty.index()].name(tyenv)
                        ),
                    );
                }

                Some(Expr::Var(bv.ty, bv.id))
            }
            &ast::Expr::ConstInt { val, pos } => {
                if !tyenv.types[ty.index()].is_prim() {
                    tyenv.report_error(
                        pos,
                        format!(
                            "expected non-primitive type {}, but found integer literal '{}'",
                            tyenv.types[ty.index()].name(tyenv),
                            val,
                        ),
                    );
                }
                Some(Expr::ConstInt(ty, val))
            }
            &ast::Expr::ConstPrim { ref val, pos } => {
                let val = tyenv.intern_mut(val);
                let const_ty = match tyenv.const_types.get(&val) {
                    Some(ty) => *ty,
                    None => {
                        tyenv.report_error(pos, "Unknown constant");
                        return None;
                    }
                };
                if const_ty != ty {
                    tyenv.report_error(
                        pos,
                        format!(
                            "Constant '{}' has wrong type: expected {}, but is actually {}",
                            tyenv.syms[val.index()],
                            tyenv.types[ty.index()].name(tyenv),
                            tyenv.types[const_ty.index()].name(tyenv)
                        ),
                    );
                    return None;
                }
                Some(Expr::ConstPrim(ty, val))
            }
            &ast::Expr::Let {
                ref defs,
                ref body,
                pos,
            } => {
                let orig_binding_len = bindings.vars.len();

                // For each new binding...
                let mut let_defs = vec![];
                for def in defs {
                    // Check that the given variable name does not already exist.
                    let name = tyenv.intern_mut(&def.var);
                    if bindings.vars.iter().any(|bv| bv.name == name) {
                        tyenv.report_error(pos, format!("Variable '{}' already bound", def.var.0));
                    }

                    // Look up the type.
                    let tysym = match tyenv.intern(&def.ty) {
                        Some(ty) => ty,
                        None => {
                            tyenv.report_error(
                                pos,
                                format!("Unknown type {} for variable '{}'", def.ty.0, def.var.0),
                            );
                            continue;
                        }
                    };
                    let tid = match tyenv.type_map.get(&tysym) {
                        Some(tid) => *tid,
                        None => {
                            tyenv.report_error(
                                pos,
                                format!("Unknown type {} for variable '{}'", def.ty.0, def.var.0),
                            );
                            continue;
                        }
                    };

                    // Evaluate the variable's value.
                    let val = Box::new(unwrap_or_continue!(
                        self.translate_expr(tyenv, &def.val, tid, bindings)
                    ));

                    // Bind the var with the given type.
                    let id = VarId(bindings.next_var);
                    bindings.next_var += 1;
                    bindings.vars.push(BoundVar { name, id, ty: tid });

                    let_defs.push((id, ty, val));
                }

                // Evaluate the body, expecting the type of the overall let-expr.
                let body = Box::new(self.translate_expr(tyenv, body, ty, bindings)?);
                let body_ty = body.ty();

                // Pop the bindings.
                bindings.vars.truncate(orig_binding_len);

                Some(Expr::Let {
                    ty: body_ty,
                    bindings: let_defs,
                    body,
                })
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ast::Ident;
    use crate::lexer::{Lexer, Pos};
    use crate::parser::parse;

    #[test]
    fn build_type_env() {
        let text = r"
            (type u32 (primitive u32))
            (type A extern (enum (B (f1 u32) (f2 u32)) (C (f1 u32))))
        ";
        let ast = parse(Lexer::from_str(text, "file.isle").unwrap()).expect("should parse");
        let tyenv = TypeEnv::from_ast(&ast).expect("should not have type-definition errors");

        let sym_a = tyenv
            .intern(&Ident("A".to_string(), Default::default()))
            .unwrap();
        let sym_b = tyenv
            .intern(&Ident("B".to_string(), Default::default()))
            .unwrap();
        let sym_c = tyenv
            .intern(&Ident("C".to_string(), Default::default()))
            .unwrap();
        let sym_a_b = tyenv
            .intern(&Ident("A.B".to_string(), Default::default()))
            .unwrap();
        let sym_a_c = tyenv
            .intern(&Ident("A.C".to_string(), Default::default()))
            .unwrap();
        let sym_u32 = tyenv
            .intern(&Ident("u32".to_string(), Default::default()))
            .unwrap();
        let sym_f1 = tyenv
            .intern(&Ident("f1".to_string(), Default::default()))
            .unwrap();
        let sym_f2 = tyenv
            .intern(&Ident("f2".to_string(), Default::default()))
            .unwrap();

        assert_eq!(tyenv.type_map.get(&sym_u32).unwrap(), &TypeId(0));
        assert_eq!(tyenv.type_map.get(&sym_a).unwrap(), &TypeId(1));

        let expected_types = vec![
            Type::Primitive(
                TypeId(0),
                sym_u32,
                Pos {
                    file: 0,
                    offset: 19,
                    line: 2,
                    col: 0,
                },
            ),
            Type::Enum {
                name: sym_a,
                id: TypeId(1),
                is_extern: true,
                is_nodebug: false,
                variants: vec![
                    Variant {
                        name: sym_b,
                        fullname: sym_a_b,
                        id: VariantId(0),
                        fields: vec![
                            Field {
                                name: sym_f1,
                                id: FieldId(0),
                                ty: TypeId(0),
                            },
                            Field {
                                name: sym_f2,
                                id: FieldId(1),
                                ty: TypeId(0),
                            },
                        ],
                    },
                    Variant {
                        name: sym_c,
                        fullname: sym_a_c,
                        id: VariantId(1),
                        fields: vec![Field {
                            name: sym_f1,
                            id: FieldId(0),
                            ty: TypeId(0),
                        }],
                    },
                ],
                pos: Pos {
                    file: 0,
                    offset: 58,
                    line: 3,
                    col: 0,
                },
            },
        ];

        assert_eq!(tyenv.types.len(), expected_types.len());
        for (i, (actual, expected)) in tyenv.types.iter().zip(&expected_types).enumerate() {
            assert_eq!(expected, actual, "`{}`th type is not equal!", i);
        }
    }
}
