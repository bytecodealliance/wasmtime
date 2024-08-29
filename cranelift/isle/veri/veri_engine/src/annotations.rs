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

fn spec_op_to_expr(s: &SpecOp, args: &Vec<SpecExpr>, pos: &Pos, env: &ParsingEnv) -> Expr {
    fn unop<F: Fn(Box<Expr>) -> Expr>(
        u: F,
        args: &Vec<SpecExpr>,
        pos: &Pos,
        env: &ParsingEnv,
    ) -> Expr {
        assert_eq!(
            args.len(),
            1,
            "Unexpected number of args for unary operator {:?}",
            pos
        );
        return u(Box::new(spec_to_expr(&args[0], env)));
    }
    fn binop<F: Fn(Box<Expr>, Box<Expr>) -> Expr>(
        b: F,
        args: &Vec<SpecExpr>,
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
        args: &Vec<SpecExpr>,
        pos: &Pos,
        env: &ParsingEnv,
    ) -> Expr {
        assert!(
            args.len() >= 1,
            "Unexpected number of args for variadic binary operator {:?}",
            pos
        );
        let mut expr_args: Vec<Expr> = args.iter().map(|a| spec_to_expr(&a, env)).collect();
        let last = expr_args.remove(expr_args.len() - 1);

        // Reverse to keep the order of the original list
        expr_args
            .iter()
            .rev()
            .fold(last, |acc, a| b(Box::new(a.clone()), Box::new(acc)))
    }

    match s {
        // Unary
        SpecOp::Not => unop(|x| Expr::Not(x), args, pos, env),
        SpecOp::BVNot => unop(|x| Expr::BVNot(x), args, pos, env),
        SpecOp::BVNeg => unop(|x| Expr::BVNeg(x), args, pos, env),
        SpecOp::Rev => unop(|x| Expr::Rev(x), args, pos, env),
        SpecOp::Clz => unop(|x| Expr::CLZ(x), args, pos, env),
        SpecOp::Cls => unop(|x| Expr::CLS(x), args, pos, env),
        SpecOp::Popcnt => unop(|x| Expr::BVPopcnt(x), args, pos, env),
        SpecOp::BV2Int => unop(|x| Expr::BVToInt(x), args, pos, env),

        // Variadic binops
        SpecOp::And => variadic_binop(|x, y| Expr::And(x, y), args, pos, env),
        SpecOp::Or => variadic_binop(|x, y| Expr::Or(x, y), args, pos, env),

        // Binary
        SpecOp::Eq => binop(|x, y| Expr::Eq(x, y), args, pos, env),
        SpecOp::Lt => binop(|x, y| Expr::Lt(x, y), args, pos, env),
        SpecOp::Lte => binop(|x, y| Expr::Lte(x, y), args, pos, env),
        SpecOp::Gt => binop(|x, y| Expr::Lt(y, x), args, pos, env),
        SpecOp::Gte => binop(|x, y| Expr::Lte(y, x), args, pos, env),
        SpecOp::Imp => binop(|x, y| Expr::Imp(x, y), args, pos, env),
        SpecOp::BVAnd => binop(|x, y| Expr::BVAnd(x, y), args, pos, env),
        SpecOp::BVOr => binop(|x, y| Expr::BVOr(x, y), args, pos, env),
        SpecOp::BVXor => binop(|x, y| Expr::BVXor(x, y), args, pos, env),
        SpecOp::BVAdd => binop(|x, y| Expr::BVAdd(x, y), args, pos, env),
        SpecOp::BVSub => binop(|x, y| Expr::BVSub(x, y), args, pos, env),
        SpecOp::BVMul => binop(|x, y| Expr::BVMul(x, y), args, pos, env),
        SpecOp::BVUdiv => binop(|x, y| Expr::BVUDiv(x, y), args, pos, env),
        SpecOp::BVUrem => binop(|x, y| Expr::BVUrem(x, y), args, pos, env),
        SpecOp::BVSdiv => binop(|x, y| Expr::BVSDiv(x, y), args, pos, env),
        SpecOp::BVSrem => binop(|x, y| Expr::BVSrem(x, y), args, pos, env),
        SpecOp::BVShl => binop(|x, y| Expr::BVShl(x, y), args, pos, env),
        SpecOp::BVLshr => binop(|x, y| Expr::BVShr(x, y), args, pos, env),
        SpecOp::BVAshr => binop(|x, y| Expr::BVAShr(x, y), args, pos, env),
        SpecOp::BVSaddo => binop(|x, y| Expr::BVSaddo(x, y), args, pos, env),
        SpecOp::BVUle => binop(|x, y| Expr::BVUlte(x, y), args, pos, env),
        SpecOp::BVUlt => binop(|x, y| Expr::BVUlt(x, y), args, pos, env),
        SpecOp::BVUgt => binop(|x, y| Expr::BVUgt(x, y), args, pos, env),
        SpecOp::BVUge => binop(|x, y| Expr::BVUgte(x, y), args, pos, env),
        SpecOp::BVSlt => binop(|x, y| Expr::BVSlt(x, y), args, pos, env),
        SpecOp::BVSle => binop(|x, y| Expr::BVSlte(x, y), args, pos, env),
        SpecOp::BVSgt => binop(|x, y| Expr::BVSgt(x, y), args, pos, env),
        SpecOp::BVSge => binop(|x, y| Expr::BVSgte(x, y), args, pos, env),
        SpecOp::Rotr => binop(|x, y| Expr::BVRotr(x, y), args, pos, env),
        SpecOp::Rotl => binop(|x, y| Expr::BVRotl(x, y), args, pos, env),
        SpecOp::ZeroExt => match spec_to_usize(&args[0]) {
            Some(i) => Expr::BVZeroExtTo(
                Box::new(Width::Const(i)),
                Box::new(spec_to_expr(&args[1], env)),
            ),
            None => binop(|x, y| Expr::BVZeroExtToVarWidth(x, y), args, pos, env),
        },
        SpecOp::SignExt => match spec_to_usize(&args[0]) {
            Some(i) => Expr::BVSignExtTo(
                Box::new(Width::Const(i)),
                Box::new(spec_to_expr(&args[1], env)),
            ),
            None => binop(|x, y| Expr::BVSignExtToVarWidth(x, y), args, pos, env),
        },
        SpecOp::ConvTo => binop(|x, y| Expr::BVConvTo(x, y), args, pos, env),
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
        SpecOp::WidthOf => unop(|x| Expr::WidthOf(x), args, pos, env),
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
        match def {
            &ast::Def::Model(Model { ref name, ref val }) => match val {
                ast::ModelValue::TypeValue(model_type) => {
                    let type_id = typeenv.get_type_by_name(&name).unwrap();
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
            },
            _ => (),
        }
    }

    // Traverse defs to process spec annotations
    for def in defs {
        match def {
            &ast::Def::Spec(ref spec) => {
                let termname = spec.term.0.clone();
                let term_id = termenv.get_term_by_name(typeenv, &spec.term)
                .unwrap_or_else(|| panic!("Spec provided for unknown decl {termname}"));
                assert!(
                    !annotation_map.contains_key(&term_id),
                    "duplicate spec for {}",
                    termname
                );
                let sig = TermSignature {
                    args: spec
                        .args
                        .iter()
                        .map(|a| spec_to_annotation_bound_var(a))
                        .collect(),
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
            _ => {}
        }
    }

    // Collect term instantiations.
    let mut forms_map = HashMap::new();
    for def in defs {
        match def {
            &ast::Def::Form(ref form) => {
                let term_type_signatures: Vec<_> = form
                    .signatures
                    .iter()
                    .map(signature_to_term_type_signature)
                    .collect();
                forms_map.insert(form.name.0.clone(), term_type_signatures);
            }
            _ => {}
        }
    }

    let mut instantiations_map = HashMap::new();
    for def in defs {
        match def {
            &ast::Def::Instantiation(ref inst) => {
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
            _ => {}
        }
    }

    AnnotationEnv {
        annotation_map,
        instantiations_map,
        model_map,
    }
}
