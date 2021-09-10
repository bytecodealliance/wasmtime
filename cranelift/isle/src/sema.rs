//! Semantic analysis.

use crate::ast;
use crate::error::*;
use crate::lexer::Pos;
use std::collections::HashMap;

pub type SemaResult<T> = std::result::Result<T, SemaError>;

#[macro_export]
macro_rules! declare_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub usize);
        impl $name {
            pub fn index(self) -> usize {
                self.0
            }
        }
    };
}

declare_id!(Sym);
declare_id!(TypeId);
declare_id!(VariantId);
declare_id!(FieldId);
declare_id!(TermId);
declare_id!(RuleId);
declare_id!(VarId);

#[derive(Clone, Debug)]
pub struct TypeEnv {
    pub filenames: Vec<String>,
    pub syms: Vec<String>,
    pub sym_map: HashMap<String, Sym>,
    pub types: Vec<Type>,
    pub type_map: HashMap<Sym, TypeId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Primitive(TypeId, Sym),
    Enum {
        name: Sym,
        id: TypeId,
        is_extern: bool,
        variants: Vec<Variant>,
        pos: Pos,
    },
}

impl Type {
    pub fn name<'a>(&self, tyenv: &'a TypeEnv) -> &'a str {
        match self {
            Self::Primitive(_, name) | Self::Enum { name, .. } => &tyenv.syms[name.index()],
        }
    }

    pub fn is_prim(&self) -> bool {
        match self {
            &Type::Primitive(..) => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    pub name: Sym,
    pub fullname: Sym,
    pub id: VariantId,
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub name: Sym,
    pub id: FieldId,
    pub ty: TypeId,
}

#[derive(Clone, Debug)]
pub struct TermEnv {
    pub terms: Vec<Term>,
    pub term_map: HashMap<Sym, TermId>,
    pub rules: Vec<Rule>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Term {
    pub id: TermId,
    pub name: Sym,
    pub arg_tys: Vec<TypeId>,
    pub ret_ty: TypeId,
    pub kind: TermKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TermKind {
    EnumVariant {
        /// Which variant of the enum: e.g. for enum type `A` if a
        /// term is `(A.A1 ...)` then the variant ID corresponds to
        /// `A1`.
        variant: VariantId,
    },
    /// A term with "internal" rules that work in the forward
    /// direction. Becomes a compiled Rust function in the generated
    /// code.
    InternalConstructor,
    /// A term that defines an "extractor macro" in the LHS of a
    /// pattern. Its arguments take patterns and are simply
    /// substituted with the given patterns when used.
    InternalExtractor { template: ast::Pattern },
    /// A term defined solely by an external extractor function.
    ExternalExtractor {
        /// Extractor func.
        name: Sym,
        /// Which arguments of the extractor are inputs and which are outputs?
        arg_polarity: Vec<ArgPolarity>,
        /// Is the external extractor infallible?
        infallible: bool,
    },
    /// A term defined solely by an external constructor function.
    ExternalConstructor {
        /// Constructor func.
        name: Sym,
    },
    /// Declared but no body or externs associated (yet).
    Declared,
}

pub use crate::ast::ArgPolarity;

#[derive(Clone, Debug)]
pub struct ExternalSig {
    pub func_name: String,
    pub full_name: String,
    pub arg_tys: Vec<TypeId>,
    pub ret_tys: Vec<TypeId>,
    pub infallible: bool,
}

impl Term {
    pub fn ty(&self) -> TypeId {
        self.ret_ty
    }

    pub fn to_variant(&self) -> Option<VariantId> {
        match &self.kind {
            &TermKind::EnumVariant { variant } => Some(variant),
            _ => None,
        }
    }

    pub fn is_constructor(&self) -> bool {
        match &self.kind {
            &TermKind::InternalConstructor { .. } | &TermKind::ExternalConstructor { .. } => true,
            _ => false,
        }
    }

    pub fn is_extractor(&self) -> bool {
        match &self.kind {
            &TermKind::InternalExtractor { .. } | &TermKind::ExternalExtractor { .. } => true,
            _ => false,
        }
    }

    pub fn is_external(&self) -> bool {
        match &self.kind {
            &TermKind::ExternalExtractor { .. } | &TermKind::ExternalConstructor { .. } => true,
            _ => false,
        }
    }

    pub fn to_sig(&self, tyenv: &TypeEnv) -> Option<ExternalSig> {
        match &self.kind {
            &TermKind::ExternalConstructor { name } => Some(ExternalSig {
                func_name: tyenv.syms[name.index()].clone(),
                full_name: format!("C::{}", tyenv.syms[name.index()]),
                arg_tys: self.arg_tys.clone(),
                ret_tys: vec![self.ret_ty],
                infallible: true,
            }),
            &TermKind::ExternalExtractor {
                name,
                ref arg_polarity,
                infallible,
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
                    arg_tys,
                    ret_tys,
                    infallible,
                })
            }
            &TermKind::InternalConstructor { .. } => {
                let name = format!("constructor_{}", tyenv.syms[self.name.index()]);
                Some(ExternalSig {
                    func_name: name.clone(),
                    full_name: name,
                    arg_tys: self.arg_tys.clone(),
                    ret_tys: vec![self.ret_ty],
                    infallible: false,
                })
            }
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Rule {
    pub id: RuleId,
    pub lhs: Pattern,
    pub rhs: Expr,
    pub prio: Option<i64>,
    pub pos: Pos,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pattern {
    BindPattern(TypeId, VarId, Box<Pattern>),
    Var(TypeId, VarId),
    ConstInt(TypeId, i64),
    Term(TypeId, TermId, Vec<TermArgPattern>),
    Wildcard(TypeId),
    And(TypeId, Vec<Pattern>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TermArgPattern {
    Pattern(Pattern),
    Expr(Expr),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Term(TypeId, TermId, Vec<Expr>),
    Var(TypeId, VarId),
    ConstInt(TypeId, i64),
    Let(TypeId, Vec<(VarId, TypeId, Box<Expr>)>, Box<Expr>),
}

impl Pattern {
    pub fn ty(&self) -> TypeId {
        match self {
            &Self::BindPattern(t, ..) => t,
            &Self::Var(t, ..) => t,
            &Self::ConstInt(t, ..) => t,
            &Self::Term(t, ..) => t,
            &Self::Wildcard(t, ..) => t,
            &Self::And(t, ..) => t,
        }
    }

    pub fn root_term(&self) -> Option<TermId> {
        match self {
            &Pattern::Term(_, term, _) => Some(term),
            &Pattern::BindPattern(_, _, ref subpat) => subpat.root_term(),
            _ => None,
        }
    }
}

impl Expr {
    pub fn ty(&self) -> TypeId {
        match self {
            &Self::Term(t, ..) => t,
            &Self::Var(t, ..) => t,
            &Self::ConstInt(t, ..) => t,
            &Self::Let(t, ..) => t,
        }
    }
}

impl TypeEnv {
    pub fn from_ast(defs: &ast::Defs) -> SemaResult<TypeEnv> {
        let mut tyenv = TypeEnv {
            filenames: defs.filenames.clone(),
            syms: vec![],
            sym_map: HashMap::new(),
            types: vec![],
            type_map: HashMap::new(),
        };

        // Traverse defs, assigning type IDs to type names. We'll fill
        // in types on a second pass.
        for def in &defs.defs {
            match def {
                &ast::Def::Type(ref td) => {
                    let tid = TypeId(tyenv.type_map.len());
                    let name = tyenv.intern_mut(&td.name);
                    if tyenv.type_map.contains_key(&name) {
                        return Err(tyenv.error(
                            td.pos,
                            format!("Type name defined more than once: '{}'", td.name.0),
                        ));
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
                    let ty = tyenv.type_from_ast(TypeId(tid), td)?;
                    tyenv.types.push(ty);
                    tid += 1;
                }
                _ => {}
            }
        }

        Ok(tyenv)
    }

    fn type_from_ast(&mut self, tid: TypeId, ty: &ast::Type) -> SemaResult<Type> {
        let name = self.intern(&ty.name).unwrap();
        match &ty.ty {
            &ast::TypeValue::Primitive(ref id) => Ok(Type::Primitive(tid, self.intern_mut(id))),
            &ast::TypeValue::Enum(ref ty_variants) => {
                let mut variants = vec![];
                for variant in ty_variants {
                    let combined_ident = ast::Ident(format!("{}.{}", ty.name.0, variant.name.0));
                    let fullname = self.intern_mut(&combined_ident);
                    let name = self.intern_mut(&variant.name);
                    let id = VariantId(variants.len());
                    if variants.iter().any(|v: &Variant| v.name == name) {
                        return Err(self.error(
                            ty.pos,
                            format!("Duplicate variant name in type: '{}'", variant.name.0),
                        ));
                    }
                    let mut fields = vec![];
                    for field in &variant.fields {
                        let field_name = self.intern_mut(&field.name);
                        if fields.iter().any(|f: &Field| f.name == field_name) {
                            return Err(self.error(
                                ty.pos,
                                format!(
                                    "Duplicate field name '{}' in variant '{}' of type",
                                    field.name.0, variant.name.0
                                ),
                            ));
                        }
                        let field_ty = self.intern_mut(&field.ty);
                        let field_tid = match self.type_map.get(&field_ty) {
                            Some(tid) => *tid,
                            None => {
                                return Err(self.error(
                                    ty.pos,
                                    format!(
                                        "Unknown type '{}' for field '{}' in variant '{}'",
                                        field.ty.0, field.name.0, variant.name.0
                                    ),
                                ));
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
                Ok(Type::Enum {
                    name,
                    id: tid,
                    is_extern: ty.is_extern,
                    variants,
                    pos: ty.pos,
                })
            }
        }
    }

    fn error(&self, pos: Pos, msg: String) -> SemaError {
        SemaError {
            filename: self.filenames[pos.file].clone(),
            pos,
            msg,
        }
    }

    pub fn intern_mut(&mut self, ident: &ast::Ident) -> Sym {
        if let Some(s) = self.sym_map.get(&ident.0).cloned() {
            s
        } else {
            let s = Sym(self.syms.len());
            self.syms.push(ident.0.clone());
            self.sym_map.insert(ident.0.clone(), s);
            s
        }
    }

    pub fn intern(&self, ident: &ast::Ident) -> Option<Sym> {
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
    pub fn from_ast(tyenv: &mut TypeEnv, defs: &ast::Defs) -> SemaResult<TermEnv> {
        let mut env = TermEnv {
            terms: vec![],
            term_map: HashMap::new(),
            rules: vec![],
        };

        env.collect_term_sigs(tyenv, defs)?;
        env.collect_enum_variant_terms(tyenv)?;
        env.collect_constructors(tyenv, defs)?;
        env.collect_extractor_templates(tyenv, defs)?;
        env.collect_rules(tyenv, defs)?;

        Ok(env)
    }

    fn collect_term_sigs(&mut self, tyenv: &mut TypeEnv, defs: &ast::Defs) -> SemaResult<()> {
        for def in &defs.defs {
            match def {
                &ast::Def::Decl(ref decl) => {
                    let tid = TermId(self.terms.len());
                    let name = tyenv.intern_mut(&decl.term);
                    if self.term_map.contains_key(&name) {
                        return Err(
                            tyenv.error(decl.pos, format!("Duplicate decl for '{}'", decl.term.0))
                        );
                    }
                    self.term_map.insert(name, tid);

                    let arg_tys = decl
                        .arg_tys
                        .iter()
                        .map(|id| {
                            let sym = tyenv.intern_mut(id);
                            tyenv.type_map.get(&sym).cloned().ok_or_else(|| {
                                tyenv.error(decl.pos, format!("Unknown arg type: '{}'", id.0))
                            })
                        })
                        .collect::<SemaResult<Vec<TypeId>>>()?;
                    let ret_ty = {
                        let sym = tyenv.intern_mut(&decl.ret_ty);
                        tyenv.type_map.get(&sym).cloned().ok_or_else(|| {
                            tyenv.error(
                                decl.pos,
                                format!("Unknown return type: '{}'", decl.ret_ty.0),
                            )
                        })?
                    };

                    self.terms.push(Term {
                        id: tid,
                        name,
                        arg_tys,
                        ret_ty,
                        kind: TermKind::Declared,
                    });
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn collect_enum_variant_terms(&mut self, tyenv: &mut TypeEnv) -> SemaResult<()> {
        for ty in &tyenv.types {
            match ty {
                &Type::Enum {
                    pos,
                    id,
                    ref variants,
                    ..
                } => {
                    for variant in variants {
                        if self.term_map.contains_key(&variant.fullname) {
                            return Err(tyenv.error(
                                pos,
                                format!(
                                    "Duplicate enum variant constructor: '{}'",
                                    tyenv.syms[variant.fullname.index()]
                                ),
                            ));
                        }
                        let tid = TermId(self.terms.len());
                        let arg_tys = variant.fields.iter().map(|fld| fld.ty).collect::<Vec<_>>();
                        let ret_ty = id;
                        self.terms.push(Term {
                            id: tid,
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

        Ok(())
    }

    fn collect_constructors(&mut self, tyenv: &mut TypeEnv, defs: &ast::Defs) -> SemaResult<()> {
        for def in &defs.defs {
            match def {
                &ast::Def::Rule(ref rule) => {
                    let pos = rule.pos;
                    let term = match rule.pattern.root_term() {
                        Some(t) => t,
                        None => {
                            return Err(tyenv.error(
                                pos,
                                "Rule does not have a term at the LHS root".to_string(),
                            ));
                        }
                    };
                    let sym = tyenv.intern_mut(&term);
                    let term = match self.term_map.get(&sym) {
                        Some(&tid) => tid,
                        None => {
                            return Err(
                                tyenv.error(pos, "Rule LHS root term is not defined".to_string())
                            );
                        }
                    };
                    let termdata = &mut self.terms[term.index()];
                    match &termdata.kind {
                        &TermKind::Declared => {
                            termdata.kind = TermKind::InternalConstructor;
                        }
                        &TermKind::InternalConstructor => {
                            // OK, no error; multiple rules can apply to one internal constructor term.
                        }
                        _ => {
                            return Err(tyenv.error(pos, "Rule LHS root term is incorrect kind; cannot be internal constructor".to_string()));
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn collect_extractor_templates(
        &mut self,
        tyenv: &mut TypeEnv,
        defs: &ast::Defs,
    ) -> SemaResult<()> {
        for def in &defs.defs {
            match def {
                &ast::Def::Extractor(ref ext) => {
                    let sym = tyenv.intern_mut(&ext.term);
                    let term = self.term_map.get(&sym).ok_or_else(|| {
                        tyenv.error(
                            ext.pos,
                            "Extractor macro body definition on a non-existent term".to_string(),
                        )
                    })?;
                    let termdata = &mut self.terms[term.index()];
                    let template = ext.template.make_macro_template(&ext.args[..]);
                    log::trace!("extractor def: {:?} becomes template {:?}", def, template);
                    match &termdata.kind {
                        &TermKind::Declared => {
                            termdata.kind = TermKind::InternalExtractor { template };
                        }
                        _ => {
                            return Err(tyenv.error(
                                ext.pos,
                                "Extractor macro body defined on term of incorrect kind"
                                    .to_string(),
                            ));
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn collect_rules(&mut self, tyenv: &mut TypeEnv, defs: &ast::Defs) -> SemaResult<()> {
        for def in &defs.defs {
            match def {
                &ast::Def::Rule(ref rule) => {
                    let pos = rule.pos;
                    let mut bindings = Bindings {
                        next_var: 0,
                        vars: vec![],
                    };

                    let (lhs, ty) = self.translate_pattern(
                        tyenv,
                        rule.pos,
                        &rule.pattern,
                        None,
                        &mut bindings,
                    )?;
                    let rhs =
                        self.translate_expr(tyenv, rule.pos, &rule.expr, ty, &mut bindings)?;

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
                            return Err(tyenv.error(
                                pos,
                                format!("Constructor declared on undefined term '{}'", term.0),
                            ))
                        }
                    };
                    let termdata = &mut self.terms[term_id.index()];
                    match &termdata.kind {
                        &TermKind::Declared => {
                            termdata.kind = TermKind::ExternalConstructor { name: func_sym };
                        }
                        _ => {
                            return Err(tyenv.error(
                                pos,
                                format!(
                                    "Constructor defined on term of improper type '{}'",
                                    term.0
                                ),
                            ));
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
                            return Err(tyenv.error(
                                pos,
                                format!("Extractor declared on undefined term '{}'", term.0),
                            ))
                        }
                    };

                    let termdata = &mut self.terms[term_id.index()];

                    let arg_polarity = if let Some(pol) = arg_polarity.as_ref() {
                        if pol.len() != termdata.arg_tys.len() {
                            return Err(tyenv.error(pos, "Incorrect number of argument-polarity directions in extractor definition".to_string()));
                        }
                        pol.clone()
                    } else {
                        vec![ArgPolarity::Output; termdata.arg_tys.len()]
                    };

                    match &termdata.kind {
                        &TermKind::Declared => {
                            termdata.kind = TermKind::ExternalExtractor {
                                name: func_sym,
                                arg_polarity,
                                infallible,
                            };
                        }
                        _ => {
                            return Err(tyenv.error(
                                pos,
                                format!("Extractor defined on term of improper type '{}'", term.0),
                            ));
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn translate_pattern(
        &self,
        tyenv: &mut TypeEnv,
        pos: Pos,
        pat: &ast::Pattern,
        expected_ty: Option<TypeId>,
        bindings: &mut Bindings,
    ) -> SemaResult<(Pattern, TypeId)> {
        log::trace!("translate_pattern: {:?}", pat);
        log::trace!("translate_pattern: bindings = {:?}", bindings);
        match pat {
            // TODO: flag on primitive type decl indicating it's an integer type?
            &ast::Pattern::ConstInt { val } => {
                let ty = expected_ty.ok_or_else(|| {
                    tyenv.error(pos, "Need an implied type for an integer constant".into())
                })?;
                Ok((Pattern::ConstInt(ty, val), ty))
            }
            &ast::Pattern::Wildcard => {
                let ty = expected_ty.ok_or_else(|| {
                    tyenv.error(pos, "Need an implied type for a wildcard".into())
                })?;
                Ok((Pattern::Wildcard(ty), ty))
            }
            &ast::Pattern::And { ref subpats } => {
                let mut expected_ty = expected_ty;
                let mut children = vec![];
                for subpat in subpats {
                    let (subpat, ty) =
                        self.translate_pattern(tyenv, pos, &*subpat, expected_ty, bindings)?;
                    expected_ty = expected_ty.or(Some(ty));
                    children.push(subpat);
                }
                if expected_ty.is_none() {
                    return Err(tyenv.error(pos, "No type for (and ...) form.".to_string()));
                }
                let ty = expected_ty.unwrap();
                Ok((Pattern::And(ty, children), ty))
            }
            &ast::Pattern::BindPattern {
                ref var,
                ref subpat,
            } => {
                // Do the subpattern first so we can resolve the type for sure.
                let (subpat, ty) =
                    self.translate_pattern(tyenv, pos, &*subpat, expected_ty, bindings)?;

                let name = tyenv.intern_mut(var);
                if bindings.vars.iter().any(|bv| bv.name == name) {
                    return Err(tyenv.error(
                        pos,
                        format!("Rebound variable name in LHS pattern: '{}'", var.0),
                    ));
                }
                let id = VarId(bindings.next_var);
                bindings.next_var += 1;
                log::trace!("binding var {:?}", var.0);
                bindings.vars.push(BoundVar { name, id, ty });

                Ok((Pattern::BindPattern(ty, id, Box::new(subpat)), ty))
            }
            &ast::Pattern::Var { ref var } => {
                // Look up the variable; it must already have been bound.
                let name = tyenv.intern_mut(var);
                let bv = match bindings.vars.iter().rev().find(|bv| bv.name == name) {
                    None => {
                        return Err(tyenv.error(
                            pos,
                            format!(
                                "Unknown variable '{}' in bound-var pattern '={}'",
                                var.0, var.0
                            ),
                        ))
                    }
                    Some(bv) => bv,
                };
                let ty = match expected_ty {
                    None => bv.ty,
                    Some(expected_ty) if expected_ty == bv.ty => bv.ty,
                    Some(expected_ty) => {
                        return Err(tyenv.error(pos, format!("Mismatched types: pattern expects type '{}' but already-bound var '{}' has type '{}'", tyenv.types[expected_ty.index()].name(tyenv), var.0, tyenv.types[bv.ty.index()].name(tyenv))));
                    }
                };
                Ok((Pattern::Var(ty, bv.id), ty))
            }
            &ast::Pattern::Term { ref sym, ref args } => {
                let name = tyenv.intern_mut(&sym);
                // Look up the term.
                let tid = self.term_map.get(&name).ok_or_else(|| {
                    tyenv.error(pos, format!("Unknown term in pattern: '{}'", sym.0))
                })?;

                // Get the return type and arg types. Verify the
                // expected type of this pattern, if any, against the
                // return type of the term.
                let ret_ty = self.terms[tid.index()].ret_ty;
                let ty = match expected_ty {
                    None => ret_ty,
                    Some(expected_ty) if expected_ty == ret_ty => ret_ty,
                    Some(expected_ty) => {
                        return Err(tyenv.error(pos, format!("Mismatched types: pattern expects type '{}' but term has return type '{}'", tyenv.types[expected_ty.index()].name(tyenv), tyenv.types[ret_ty.index()].name(tyenv))));
                    }
                };

                // Check that we have the correct argument count.
                if self.terms[tid.index()].arg_tys.len() != args.len() {
                    return Err(tyenv.error(
                        pos,
                        format!(
                            "Incorrect argument count for term '{}': got {}, expect {}",
                            sym.0,
                            args.len(),
                            self.terms[tid.index()].arg_tys.len()
                        ),
                    ));
                }

                let termdata = &self.terms[tid.index()];

                match &termdata.kind {
                    &TermKind::EnumVariant { .. } => {
                        for arg in args {
                            if let &ast::TermArgPattern::Expr(..) = arg {
                                return Err(tyenv.error(pos, format!("Term in pattern '{}' cannot have an injected expr, because it is an enum variant", sym.0)));
                            }
                        }
                    }
                    &TermKind::ExternalExtractor {
                        ref arg_polarity, ..
                    } => {
                        for (arg, pol) in args.iter().zip(arg_polarity.iter()) {
                            match (arg, pol) {
                                (&ast::TermArgPattern::Expr(..), &ArgPolarity::Input) => {}
                                (&ast::TermArgPattern::Expr(..), &ArgPolarity::Output) => {
                                    return Err(tyenv.error(
                                        pos,
                                        "Expression used for output-polarity extractor arg"
                                            .to_string(),
                                    ));
                                }
                                (_, &ArgPolarity::Output) => {}
                                (_, &ArgPolarity::Input) => {
                                    return Err(tyenv.error(pos, "Non-expression used in pattern but expression required for input-polarity extractor arg".to_string()));
                                }
                            }
                        }
                    }
                    &TermKind::InternalExtractor { ref template } => {
                        // Expand the extractor macro! We create a map
                        // from macro args to AST pattern trees and
                        // then evaluate the template with these
                        // substitutions.
                        let mut macro_args: Vec<ast::Pattern> = vec![];
                        for template_arg in args {
                            let sub_ast = match template_arg {
                                &ast::TermArgPattern::Pattern(ref pat) => pat.clone(),
                                &ast::TermArgPattern::Expr(_) => {
                                    return Err(tyenv.error(pos, "Cannot expand an extractor macro with an expression in a macro argument".to_string()));
                                }
                            };
                            macro_args.push(sub_ast.clone());
                        }
                        log::trace!("internal extractor macro args = {:?}", args);
                        let pat = template.subst_macro_args(&macro_args[..]);
                        return self.translate_pattern(tyenv, pos, &pat, expected_ty, bindings);
                    }
                    &TermKind::ExternalConstructor { .. } | &TermKind::InternalConstructor => {
                        // OK.
                    }
                    &TermKind::Declared => {
                        return Err(tyenv
                            .error(pos, format!("Declared but undefined term '{}' used", sym.0)));
                    }
                }

                // Resolve subpatterns.
                let mut subpats = vec![];
                for (i, arg) in args.iter().enumerate() {
                    let arg_ty = self.terms[tid.index()].arg_tys[i];
                    let (subpat, _) =
                        self.translate_pattern_term_arg(tyenv, pos, arg, Some(arg_ty), bindings)?;
                    subpats.push(subpat);
                }

                Ok((Pattern::Term(ty, *tid, subpats), ty))
            }
            &ast::Pattern::MacroArg { .. } => unreachable!(),
        }
    }

    fn translate_pattern_term_arg(
        &self,
        tyenv: &mut TypeEnv,
        pos: Pos,
        pat: &ast::TermArgPattern,
        expected_ty: Option<TypeId>,
        bindings: &mut Bindings,
    ) -> SemaResult<(TermArgPattern, TypeId)> {
        match pat {
            &ast::TermArgPattern::Pattern(ref pat) => {
                let (subpat, ty) =
                    self.translate_pattern(tyenv, pos, pat, expected_ty, bindings)?;
                Ok((TermArgPattern::Pattern(subpat), ty))
            }
            &ast::TermArgPattern::Expr(ref expr) => {
                if expected_ty.is_none() {
                    return Err(tyenv.error(
                        pos,
                        "Expression in pattern must have expected type".to_string(),
                    ));
                }
                let ty = expected_ty.unwrap();
                let expr = self.translate_expr(tyenv, pos, expr, expected_ty.unwrap(), bindings)?;
                Ok((TermArgPattern::Expr(expr), ty))
            }
        }
    }

    fn translate_expr(
        &self,
        tyenv: &mut TypeEnv,
        pos: Pos,
        expr: &ast::Expr,
        ty: TypeId,
        bindings: &mut Bindings,
    ) -> SemaResult<Expr> {
        log::trace!("translate_expr: {:?}", expr);
        match expr {
            &ast::Expr::Term { ref sym, ref args } => {
                // Look up the term.
                let name = tyenv.intern_mut(&sym);
                // Look up the term.
                let tid = self.term_map.get(&name).ok_or_else(|| {
                    tyenv.error(pos, format!("Unknown term in pattern: '{}'", sym.0))
                })?;

                // Get the return type and arg types. Verify the
                // expected type of this pattern, if any, against the
                // return type of the term.
                let ret_ty = self.terms[tid.index()].ret_ty;
                if ret_ty != ty {
                    return Err(tyenv.error(pos, format!("Mismatched types: expression expects type '{}' but term has return type '{}'", tyenv.types[ty.index()].name(tyenv), tyenv.types[ret_ty.index()].name(tyenv))));
                }

                // Check that we have the correct argument count.
                if self.terms[tid.index()].arg_tys.len() != args.len() {
                    return Err(tyenv.error(
                        pos,
                        format!(
                            "Incorrect argument count for term '{}': got {}, expect {}",
                            sym.0,
                            args.len(),
                            self.terms[tid.index()].arg_tys.len()
                        ),
                    ));
                }

                // Resolve subexpressions.
                let mut subexprs = vec![];
                for (i, arg) in args.iter().enumerate() {
                    let arg_ty = self.terms[tid.index()].arg_tys[i];
                    let subexpr = self.translate_expr(tyenv, pos, arg, arg_ty, bindings)?;
                    subexprs.push(subexpr);
                }

                Ok(Expr::Term(ty, *tid, subexprs))
            }
            &ast::Expr::Var { ref name } => {
                let sym = tyenv.intern_mut(name);
                // Look through bindings, innermost (most recent) first.
                let bv = match bindings.vars.iter().rev().find(|b| b.name == sym) {
                    None => {
                        return Err(tyenv.error(pos, format!("Unknown variable '{}'", name.0)));
                    }
                    Some(bv) => bv,
                };

                // Verify type.
                if bv.ty != ty {
                    return Err(tyenv.error(
                        pos,
                        format!(
                            "Variable '{}' has type {} but we need {} in context",
                            name.0,
                            tyenv.types[bv.ty.index()].name(tyenv),
                            tyenv.types[ty.index()].name(tyenv)
                        ),
                    ));
                }

                Ok(Expr::Var(bv.ty, bv.id))
            }
            &ast::Expr::ConstInt { val } => Ok(Expr::ConstInt(ty, val)),
            &ast::Expr::Let { ref defs, ref body } => {
                let orig_binding_len = bindings.vars.len();

                // For each new binding...
                let mut let_defs = vec![];
                for def in defs {
                    // Check that the given variable name does not already exist.
                    let name = tyenv.intern_mut(&def.var);
                    if bindings.vars.iter().any(|bv| bv.name == name) {
                        return Err(
                            tyenv.error(pos, format!("Variable '{}' already bound", def.var.0))
                        );
                    }

                    // Look up the type.
                    let tysym = match tyenv.intern(&def.ty) {
                        Some(ty) => ty,
                        None => {
                            return Err(tyenv.error(
                                pos,
                                format!("Unknown type {} for variable '{}'", def.ty.0, def.var.0),
                            ))
                        }
                    };
                    let tid = match tyenv.type_map.get(&tysym) {
                        Some(tid) => *tid,
                        None => {
                            return Err(tyenv.error(
                                pos,
                                format!("Unknown type {} for variable '{}'", def.ty.0, def.var.0),
                            ))
                        }
                    };

                    // Evaluate the variable's value.
                    let val = Box::new(self.translate_expr(tyenv, pos, &def.val, ty, bindings)?);

                    // Bind the var with the given type.
                    let id = VarId(bindings.next_var);
                    bindings.next_var += 1;
                    bindings.vars.push(BoundVar { name, id, ty: tid });

                    let_defs.push((id, ty, val));
                }

                // Evaluate the body, expecting the type of the overall let-expr.
                let body = Box::new(self.translate_expr(tyenv, pos, body, ty, bindings)?);
                let body_ty = body.ty();

                // Pop the bindings.
                bindings.vars.truncate(orig_binding_len);

                Ok(Expr::Let(body_ty, let_defs, body))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ast::Ident;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    #[test]
    fn build_type_env() {
        let text = r"
            (type u32 (primitive u32))
            (type A extern (enum (B (f1 u32) (f2 u32)) (C (f1 u32))))
        ";
        let ast = Parser::new(Lexer::from_str(text, "file.isle"))
            .parse_defs()
            .expect("should parse");
        let tyenv = TypeEnv::from_ast(&ast).expect("should not have type-definition errors");

        let sym_a = tyenv.intern(&Ident("A".to_string())).unwrap();
        let sym_b = tyenv.intern(&Ident("B".to_string())).unwrap();
        let sym_c = tyenv.intern(&Ident("C".to_string())).unwrap();
        let sym_a_b = tyenv.intern(&Ident("A.B".to_string())).unwrap();
        let sym_a_c = tyenv.intern(&Ident("A.C".to_string())).unwrap();
        let sym_u32 = tyenv.intern(&Ident("u32".to_string())).unwrap();
        let sym_f1 = tyenv.intern(&Ident("f1".to_string())).unwrap();
        let sym_f2 = tyenv.intern(&Ident("f2".to_string())).unwrap();

        assert_eq!(tyenv.type_map.get(&sym_u32).unwrap(), &TypeId(0));
        assert_eq!(tyenv.type_map.get(&sym_a).unwrap(), &TypeId(1));

        assert_eq!(
            tyenv.types,
            vec![
                Type::Primitive(TypeId(0), sym_u32),
                Type::Enum {
                    name: sym_a,
                    id: TypeId(1),
                    is_extern: true,
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
                            },],
                        },
                    ],
                    pos: Pos {
                        file: 0,
                        offset: 58,
                        line: 3,
                        col: 18,
                    },
                },
            ]
        );
    }
}
