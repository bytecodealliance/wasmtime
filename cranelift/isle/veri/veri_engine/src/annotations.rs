use cranelift_isle::ast::{self, Signature};
use std::collections::HashMap;
use veri_ir::annotation_ir;

use cranelift_isle::ast::{Def, Ident, Model, ModelType, SpecExpr, SpecOp};
use cranelift_isle::lexer::Pos;
use cranelift_isle::sema::{TermEnv, TermId, TypeEnv, TypeId};
use veri_ir::annotation_ir::Width;
use veri_ir::annotation_ir::{BoundVar, Const, Expr, TermAnnotation, TermSignature, Type};
use veri_ir::TermSignature as TermTypeSignature;

static RESULT: &str = "result";

#[derive(Clone, Debug)]
pub struct ParsingEnv<'a> {
    pub typeenv: &'a TypeEnv,
    pub enums: HashMap<String, Expr>,
}

#[derive(Clone, Debug)]
pub struct AnnotationEnv {
    pub annotation_map: HashMap<TermId, TermAnnotation>,

    // Mapping from ISLE term to its signature instantiations.
    pub instantiations_map: HashMap<TermId, Vec<TermTypeSignature>>,

    // Mapping from ISLE type to its model (the annotation used to represent
    // it).
    pub model_map: HashMap<TypeId, annotation_ir::Type>,
}

impl AnnotationEnv {
    pub fn get_annotation_for_term(&self, term_id: &TermId) -> Option<TermAnnotation> {
        if self.annotation_map.contains_key(term_id) {
            return Some(self.annotation_map[term_id].clone());
        }
        None
    }

    pub fn get_term_signatures_by_name(
        &self,
        termenv: &TermEnv,
        typeenv: &TypeEnv,
    ) -> HashMap<String, Vec<TermTypeSignature>> {
        let mut term_signatures_by_name = HashMap::new();
        for (term_id, term_sigs) in &self.instantiations_map {
            let sym = termenv.terms[term_id.index()].name;
            let name = typeenv.syms[sym.index()].clone();
            term_signatures_by_name.insert(name, term_sigs.clone());
        }
        term_signatures_by_name
    }
}

pub fn spec_to_annotation_bound_var(i: &Ident) -> BoundVar {
    BoundVar {
        name: i.0.clone(),
        ty: None,
    }
}

fn spec_to_usize(s: &SpecExpr) -> Option<usize> {
    match s {
        SpecExpr::ConstInt { val, pos: _ } => Some(*val as usize),
        _ => None,
    }
}

fn spec_op_to_expr(s: &SpecOp, args: &[SpecExpr], pos: &Pos, env: &ParsingEnv) -> Expr {
    fn unop<F: Fn(Box<Expr>) -> Expr>(
        u: F,
        args: &[SpecExpr],
        pos: &Pos,
        env: &ParsingEnv,
    ) -> Expr {
        assert_eq!(
            args.len(),
            1,
            "Unexpected number of args for unary operator {:?}",
            pos
        );
        u(Box::new(spec_to_expr(&args[0], env)))
    }
    fn binop<F: Fn(Box<Expr>, Box<Expr>) -> Expr>(
        b: F,
        args: &[SpecExpr],
        _pos: &Pos,
        env: &ParsingEnv,
    ) -> Expr {
        assert_eq!(
            args.len(),
            2,
            "Unexpected number of args for binary operator {:?}",
            args
        );
        b(
            Box::new(spec_to_expr(&args[0], env)),
            Box::new(spec_to_expr(&args[1], env)),
        )
    }

    fn variadic_binop<F: Fn(Box<Expr>, Box<Expr>) -> Expr>(
        b: F,
        args: &[SpecExpr],
        pos: &Pos,
        env: &ParsingEnv,
    ) -> Expr {
        assert!(
            !args.is_empty(),
            "Unexpected number of args for variadic binary operator {:?}",
            pos
        );
        let mut expr_args: Vec<Expr> = args.iter().map(|a| spec_to_expr(a, env)).collect();
        let last = expr_args.remove(expr_args.len() - 1);

        // Reverse to keep the order of the original list
        expr_args
            .iter()
            .rev()
            .fold(last, |acc, a| b(Box::new(a.clone()), Box::new(acc)))
    }

    match s {
        // Unary
        SpecOp::Not => unop(Expr::Not, args, pos, env),
        SpecOp::BVNot => unop(Expr::BVNot, args, pos, env),
        SpecOp::BVNeg => unop(Expr::BVNeg, args, pos, env),
        SpecOp::Rev => unop(Expr::Rev, args, pos, env),
        SpecOp::Clz => unop(Expr::CLZ, args, pos, env),
        SpecOp::Cls => unop(Expr::CLS, args, pos, env),
        SpecOp::Popcnt => unop(Expr::BVPopcnt, args, pos, env),
        SpecOp::BV2Int => unop(Expr::BVToInt, args, pos, env),

        // Variadic binops
        SpecOp::And => variadic_binop(Expr::And, args, pos, env),
        SpecOp::Or => variadic_binop(Expr::Or, args, pos, env),

        // Binary
        SpecOp::Eq => binop(Expr::Eq, args, pos, env),
        SpecOp::Lt => binop(Expr::Lt, args, pos, env),
        SpecOp::Lte => binop(Expr::Lte, args, pos, env),
        SpecOp::Gt => binop(|x, y| Expr::Lt(y, x), args, pos, env),
        SpecOp::Gte => binop(|x, y| Expr::Lte(y, x), args, pos, env),
        SpecOp::Imp => binop(Expr::Imp, args, pos, env),
        SpecOp::BVAnd => binop(Expr::BVAnd, args, pos, env),
        SpecOp::BVOr => binop(Expr::BVOr, args, pos, env),
        SpecOp::BVXor => binop(Expr::BVXor, args, pos, env),
        SpecOp::BVAdd => binop(Expr::BVAdd, args, pos, env),
        SpecOp::BVSub => binop(Expr::BVSub, args, pos, env),
        SpecOp::BVMul => binop(Expr::BVMul, args, pos, env),
        SpecOp::BVUdiv => binop(Expr::BVUDiv, args, pos, env),
        SpecOp::BVUrem => binop(Expr::BVUrem, args, pos, env),
        SpecOp::BVSdiv => binop(Expr::BVSDiv, args, pos, env),
        SpecOp::BVSrem => binop(Expr::BVSrem, args, pos, env),
        SpecOp::BVShl => binop(Expr::BVShl, args, pos, env),
        SpecOp::BVLshr => binop(Expr::BVShr, args, pos, env),
        SpecOp::BVAshr => binop(Expr::BVAShr, args, pos, env),
        SpecOp::BVSaddo => binop(Expr::BVSaddo, args, pos, env),
        SpecOp::BVUle => binop(Expr::BVUlte, args, pos, env),
        SpecOp::BVUlt => binop(Expr::BVUlt, args, pos, env),
        SpecOp::BVUgt => binop(Expr::BVUgt, args, pos, env),
        SpecOp::BVUge => binop(Expr::BVUgte, args, pos, env),
        SpecOp::BVSlt => binop(Expr::BVSlt, args, pos, env),
        SpecOp::BVSle => binop(Expr::BVSlte, args, pos, env),
        SpecOp::BVSgt => binop(Expr::BVSgt, args, pos, env),
        SpecOp::BVSge => binop(Expr::BVSgte, args, pos, env),
        SpecOp::Rotr => binop(Expr::BVRotr, args, pos, env),
        SpecOp::Rotl => binop(Expr::BVRotl, args, pos, env),
        SpecOp::ZeroExt => match spec_to_usize(&args[0]) {
            Some(i) => Expr::BVZeroExtTo(
                Box::new(Width::Const(i)),
                Box::new(spec_to_expr(&args[1], env)),
            ),
            None => binop(Expr::BVZeroExtToVarWidth, args, pos, env),
        },
        SpecOp::SignExt => match spec_to_usize(&args[0]) {
            Some(i) => Expr::BVSignExtTo(
                Box::new(Width::Const(i)),
                Box::new(spec_to_expr(&args[1], env)),
            ),
            None => binop(Expr::BVSignExtToVarWidth, args, pos, env),
        },
        SpecOp::ConvTo => binop(Expr::BVConvTo, args, pos, env),
        SpecOp::Concat => {
            let cases: Vec<Expr> = args.iter().map(|a| spec_to_expr(a, env)).collect();
            Expr::BVConcat(cases)
        }
        SpecOp::Extract => {
            assert_eq!(
                args.len(),
                3,
                "Unexpected number of args for extract operator {:?}",
                pos
            );
            Expr::BVExtract(
                spec_to_usize(&args[0]).unwrap(),
                spec_to_usize(&args[1]).unwrap(),
                Box::new(spec_to_expr(&args[2], env)),
            )
        }
        SpecOp::Int2BV => {
            assert_eq!(
                args.len(),
                2,
                "Unexpected number of args for Int2BV operator {:?}",
                pos
            );
            Expr::BVIntToBv(
                spec_to_usize(&args[0]).unwrap(),
                Box::new(spec_to_expr(&args[1], env)),
            )
        }
        SpecOp::Subs => {
            assert_eq!(
                args.len(),
                3,
                "Unexpected number of args for subs operator {:?}",
                pos
            );
            Expr::BVSubs(
                Box::new(spec_to_expr(&args[0], env)),
                Box::new(spec_to_expr(&args[1], env)),
                Box::new(spec_to_expr(&args[2], env)),
            )
        }
        SpecOp::WidthOf => unop(Expr::WidthOf, args, pos, env),
        SpecOp::If => {
            assert_eq!(
                args.len(),
                3,
                "Unexpected number of args for extract operator {:?}",
                pos
            );
            Expr::Conditional(
                Box::new(spec_to_expr(&args[0], env)),
                Box::new(spec_to_expr(&args[1], env)),
                Box::new(spec_to_expr(&args[2], env)),
            )
        }
        SpecOp::Switch => {
            assert!(
                args.len() > 1,
                "Unexpected number of args for switch operator {:?}",
                pos
            );
            let swith_on = spec_to_expr(&args[0], env);
            let arms: Vec<(Expr, Expr)> = args[1..]
                .iter()
                .map(|a| match a {
                    SpecExpr::Pair { l, r } => {
                        let l_expr = spec_to_expr(l, env);
                        let r_expr = spec_to_expr(r, env);
                        (l_expr, r_expr)
                    }
                    _ => unreachable!(),
                })
                .collect();
            Expr::Switch(Box::new(swith_on), arms)
        }
        SpecOp::LoadEffect => {
            assert_eq!(
                args.len(),
                3,
                "Unexpected number of args for load operator {:?}",
                pos
            );
            Expr::LoadEffect(
                Box::new(spec_to_expr(&args[0], env)),
                Box::new(spec_to_expr(&args[1], env)),
                Box::new(spec_to_expr(&args[2], env)),
            )
        }
        SpecOp::StoreEffect => {
            assert_eq!(
                args.len(),
                4,
                "Unexpected number of args for store operator {:?}",
                pos
            );
            Expr::StoreEffect(
                Box::new(spec_to_expr(&args[0], env)),
                Box::new(spec_to_expr(&args[1], env)),
                Box::new(spec_to_expr(&args[2], env)),
                Box::new(spec_to_expr(&args[3], env)),
            )
        }
    }
}

fn spec_to_expr(s: &SpecExpr, env: &ParsingEnv) -> Expr {
    match s {
        SpecExpr::ConstUnit { pos: _ } => Expr::Const(Const {
            ty: Type::Unit,
            value: 0,
            width: 0,
        }),
        SpecExpr::ConstInt { val, pos: _ } => Expr::Const(Const {
            ty: Type::Int,
            value: *val,
            width: 0,
        }),
        SpecExpr::ConstBitVec { val, width, pos: _ } => Expr::Const(Const {
            ty: Type::BitVectorWithWidth(*width as usize),
            value: *val,
            width: (*width as usize),
        }),
        SpecExpr::ConstBool { val, pos: _ } => Expr::Const(Const {
            ty: Type::Bool,
            value: *val as i128,
            width: 0,
        }),
        SpecExpr::Var { var, pos: _ } => Expr::Var(var.0.clone()),
        SpecExpr::Op { op, args, pos } => spec_op_to_expr(op, args, pos, env),
        SpecExpr::Pair { l, r } => {
            unreachable!(
                "pairs currently only parsed as part of Switch statements, {:?} {:?}",
                l, r
            )
        }
        SpecExpr::Enum { name } => {
            if let Some(e) = env.enums.get(&name.0) {
                e.clone()
            } else {
                panic!("Can't find model for enum {}", name.0);
            }
        }
    }
}

fn model_type_to_type(model_type: &ModelType) -> veri_ir::Type {
    match model_type {
        ModelType::Int => veri_ir::Type::Int,
        ModelType::Unit => veri_ir::Type::Unit,
        ModelType::Bool => veri_ir::Type::Bool,
        ModelType::BitVec(size) => veri_ir::Type::BitVector(*size),
    }
}

fn signature_to_term_type_signature(sig: &Signature) -> TermTypeSignature {
    TermTypeSignature {
        args: sig.args.iter().map(model_type_to_type).collect(),
        ret: model_type_to_type(&sig.ret),
        canonical_type: Some(model_type_to_type(&sig.canonical)),
    }
}

pub fn parse_annotations(defs: &[Def], termenv: &TermEnv, typeenv: &TypeEnv) -> AnnotationEnv {
    let mut annotation_map = HashMap::new();
    let mut model_map = HashMap::new();

    let mut env = ParsingEnv {
        typeenv,
        enums: HashMap::new(),
    };

    // Traverse models to process spec annotations for enums
    for def in defs {
        if let &ast::Def::Model(Model { ref name, ref val }) = def {
            match val {
                ast::ModelValue::TypeValue(model_type) => {
                    let type_id = typeenv.get_type_by_name(name).unwrap();
                    let ir_type = match model_type {
                        ModelType::Int => annotation_ir::Type::Int,
                        ModelType::Unit => annotation_ir::Type::Unit,
                        ModelType::Bool => annotation_ir::Type::Bool,
                        ModelType::BitVec(None) => annotation_ir::Type::BitVector,
                        ModelType::BitVec(Some(size)) => {
                            annotation_ir::Type::BitVectorWithWidth(*size)
                        }
                    };
                    model_map.insert(type_id, ir_type);
                }
                ast::ModelValue::EnumValues(vals) => {
                    for (v, e) in vals {
                        let ident = ast::Ident(format!("{}.{}", name.0, v.0), v.1);
                        let term_id = termenv.get_term_by_name(typeenv, &ident).unwrap();
                        let val = spec_to_expr(e, &env);
                        let ty = match val {
                            Expr::Const(Const { ref ty, .. }) => ty,
                            _ => unreachable!(),
                        };
                        env.enums.insert(ident.0.clone(), val.clone());
                        let result = BoundVar {
                            name: RESULT.to_string(),
                            ty: Some(ty.clone()),
                        };
                        let sig = TermSignature {
                            args: vec![],
                            ret: result,
                        };
                        let annotation = TermAnnotation {
                            sig,
                            assumptions: vec![Box::new(Expr::Eq(
                                Box::new(Expr::Var(RESULT.to_string())),
                                Box::new(val),
                            ))],
                            assertions: vec![],
                        };
                        annotation_map.insert(term_id, annotation);
                    }
                }
            }
        }
    }

    // Traverse defs to process spec annotations
    for def in defs {
        if let ast::Def::Spec(spec) = def {
            let termname = spec.term.0.clone();
            let term_id = termenv
                .get_term_by_name(typeenv, &spec.term)
                .unwrap_or_else(|| panic!("Spec provided for unknown decl {termname}"));
            assert!(
                !annotation_map.contains_key(&term_id),
                "duplicate spec for {}",
                termname
            );
            let sig = TermSignature {
                args: spec.args.iter().map(spec_to_annotation_bound_var).collect(),
                ret: BoundVar {
                    name: RESULT.to_string(),
                    ty: None,
                },
            };

            let mut assumptions = vec![];
            let mut assertions = vec![];
            for a in &spec.provides {
                assumptions.push(Box::new(spec_to_expr(a, &env)));
            }

            for a in &spec.requires {
                assertions.push(Box::new(spec_to_expr(a, &env)));
            }

            let annotation = TermAnnotation {
                sig,
                assumptions,
                assertions,
            };
            annotation_map.insert(term_id, annotation);
        }
    }

    // Collect term instantiations.
    let mut forms_map = HashMap::new();
    for def in defs {
        if let ast::Def::Form(form) = def {
            let term_type_signatures: Vec<_> = form
                .signatures
                .iter()
                .map(signature_to_term_type_signature)
                .collect();
            forms_map.insert(form.name.0.clone(), term_type_signatures);
        }
    }

    let mut instantiations_map = HashMap::new();
    for def in defs {
        if let ast::Def::Instantiation(inst) = def {
            let term_id = termenv.get_term_by_name(typeenv, &inst.term).unwrap();
            let sigs = match &inst.form {
                Some(form) => forms_map[&form.0].clone(),
                None => inst
                    .signatures
                    .iter()
                    .map(signature_to_term_type_signature)
                    .collect(),
            };
            instantiations_map.insert(term_id, sigs);
        }
    }

    AnnotationEnv {
        annotation_map,
        instantiations_map,
        model_map,
    }
}
