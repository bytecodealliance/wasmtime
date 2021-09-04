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
    pub filename: String,
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
    Regular {
        // Producer and consumer rules are catalogued separately after
        // building Sequences. Here we just record whether an
        // extractor and/or constructor is known.
        /// Extractor func and `infallible` flag.
        extractor: Option<(Sym, bool)>,
        /// Constructor func.
        constructor: Option<Sym>,
    },
}

#[derive(Clone, Debug)]
pub struct Rule {
    pub id: RuleId,
    pub lhs: Pattern,
    pub rhs: Expr,
    pub prio: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pattern {
    BindPattern(TypeId, VarId, Box<Pattern>),
    Var(TypeId, VarId),
    ConstInt(TypeId, i64),
    Term(TypeId, TermId, Vec<Pattern>),
    Wildcard(TypeId),
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
            filename: defs.filename.clone(),
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
            filename: self.filename.clone(),
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

struct Bindings {
    next_var: usize,
    vars: Vec<BoundVar>,
}

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
                        kind: TermKind::Regular {
                            extractor: None,
                            constructor: None,
                        },
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

    fn collect_rules(&mut self, tyenv: &mut TypeEnv, defs: &ast::Defs) -> SemaResult<()> {
        for def in &defs.defs {
            match def {
                &ast::Def::Rule(ref rule) => {
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
                    match &mut self.terms[term_id.index()].kind {
                        &mut TermKind::EnumVariant { .. } => {
                            return Err(tyenv.error(
                                pos,
                                format!("Constructor defined on enum type '{}'", term.0),
                            ));
                        }
                        &mut TermKind::Regular {
                            ref mut constructor,
                            ..
                        } => {
                            if constructor.is_some() {
                                return Err(tyenv.error(
                                    pos,
                                    format!(
                                        "Constructor defined more than once on term '{}'",
                                        term.0
                                    ),
                                ));
                            }
                            *constructor = Some(func_sym);
                        }
                    }
                }
                &ast::Def::Extern(ast::Extern::Extractor {
                    ref term,
                    ref func,
                    pos,
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
                    match &mut self.terms[term_id.index()].kind {
                        &mut TermKind::EnumVariant { .. } => {
                            return Err(tyenv.error(
                                pos,
                                format!("Extractor defined on enum type '{}'", term.0),
                            ));
                        }
                        &mut TermKind::Regular {
                            ref mut extractor, ..
                        } => {
                            if extractor.is_some() {
                                return Err(tyenv.error(
                                    pos,
                                    format!(
                                        "Extractor defined more than once on term '{}'",
                                        term.0
                                    ),
                                ));
                            }
                            *extractor = Some((func_sym, infallible));
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

                // Resolve subpatterns.
                let mut subpats = vec![];
                for (i, arg) in args.iter().enumerate() {
                    let arg_ty = self.terms[tid.index()].arg_tys[i];
                    let (subpat, _) =
                        self.translate_pattern(tyenv, pos, arg, Some(arg_ty), bindings)?;
                    subpats.push(subpat);
                }

                Ok((Pattern::Term(ty, *tid, subpats), ty))
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
    use crate::parser::Parser;

    #[test]
    fn build_type_env() {
        let text = r"
            (type u32 (primitive u32))
            (type A extern (enum (B (f1 u32) (f2 u32)) (C (f1 u32))))
        ";
        let ast = Parser::new("file.isle", text)
            .parse_defs()
            .expect("should parse");
        let tyenv = TypeEnv::from_ast(&ast).expect("should not have type-definition errors");

        let sym_a = tyenv.intern(&Ident("A".to_string())).unwrap();
        let sym_b = tyenv.intern(&Ident("A.B".to_string())).unwrap();
        let sym_c = tyenv.intern(&Ident("A.C".to_string())).unwrap();
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
                            id: VariantId(1),
                            fields: vec![Field {
                                name: sym_f1,
                                id: FieldId(0),
                                ty: TypeId(0),
                            },],
                        },
                    ],
                    pos: Pos {
                        offset: 58,
                        line: 3,
                        col: 18,
                    },
                },
            ]
        );
    }

    #[test]
    fn build_rules() {
        let text = r"
            (type u32 (primitive u32))
            (type A extern (enum (B (f1 u32) (f2 u32)) (C (f1 u32))))

            (decl T1 (A) u32)
            (decl T2 (A A) A)
            (decl T3 (u32) A)

            (constructor T1 t1_ctor)
            (extractor T2 t2_etor)

            (rule
              (T1 _) 1)
            (rule
              (T2 x =x) (T3 42))
            (rule
              (T3 1) (A.C 2))
            (rule -1
              (T3 _) (A.C 3))
        ";
        let ast = Parser::new("file.isle", text)
            .parse_defs()
            .expect("should parse");
        let mut tyenv = TypeEnv::from_ast(&ast).expect("should not have type-definition errors");
        let _ = TermEnv::from_ast(&mut tyenv, &ast).expect("could not typecheck rules");
    }
}
