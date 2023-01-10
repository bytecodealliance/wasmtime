use std::any::Any;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::Hash;

use cranelift_isle as isle;
use isle::ast::{Decl, Defs};
use isle::sema::{Sym, TermEnv, TypeEnv, VarId};
use veri_annotation::parser_wrapper::{parse_annotations, AnnotationEnv};
use veri_ir::Expr;
use veri_ir::{annotation_ir, TypeContext};

use crate::REG_WIDTH;

#[derive(Clone, Debug)]
struct RuleParseTree<'a> {
    // a map of var name to type variable, where var could be
    // Pattern::Var or var used in Pattern::BindPattern
    varid_to_type_var_map: HashMap<VarId, u32>,
    // a map of type var to value, if known
    type_var_to_val_map: HashMap<u32, i128>,
    // bookkeeping that tells the next unused type var
    next_type_var: u32,
    // combined constraints from all nodes
    concrete_constraints: HashSet<TypeExpr>,
    var_constraints: HashSet<TypeExpr>,
    bv_constraints: HashSet<TypeExpr>,
    // a map of terms in the rule to their isle ast decl
    decls: &'a HashMap<String, isle::ast::Decl>,

    ty_vars: HashMap<veri_ir::Expr, u32>,
    quantified_vars: HashSet<(String, u32)>,
    free_vars: HashSet<(String, u32)>,
    assumptions: Vec<Expr>,
}

#[derive(Clone, Debug)]
pub enum TypeVarConstruct {
    Var,
    BindPattern,
    Wildcard,
    Term(String),
    Const(i128),
    Let(Vec<String>),
    And,
}

#[derive(Clone, Debug)]
pub struct TypeVarNode {
    ident: String,
    construct: TypeVarConstruct,
    type_var: u32,
    children: Vec<TypeVarNode>,
    assertions: Vec<veri_ir::Expr>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
// Constraints either assign concrete types to type variables
// or set them equal to other type variables
enum TypeExpr {
    Concrete(u32, annotation_ir::Type),
    Variable(u32, u32),
}

#[derive(Debug)]
pub struct AnnotationTypeInfo {
    // map of annotation variable to assigned type var
    pub term: String,
    pub var_to_type_var: HashMap<String, u32>,
}

#[derive(Debug)]
pub struct Solution {
    pub annotation_infos: Vec<AnnotationTypeInfo>,

    // map of type var to solved type
    pub type_var_to_type: HashMap<u32, annotation_ir::Type>,

    pub lhs: veri_ir::Expr,
    pub rhs: veri_ir::Expr,
    pub quantified_vars: Vec<veri_ir::BoundVar>,
    pub free_vars: Vec<veri_ir::BoundVar>,
    pub assumptions: Vec<Expr>,
    pub tyctx: TypeContext,
}

pub fn type_all_rules(
    defs: Defs,
    termenv: &TermEnv,
    typeenv: &TypeEnv,
    annotation_env: &AnnotationEnv,
) -> HashMap<isle::sema::RuleId, Solution> {
    let decls = build_decl_map(defs);

    let mut solutions = HashMap::new();
    for r in &termenv.rules {
        if let Some(s) = type_annotations_using_rule(r, annotation_env, &decls, typeenv, termenv) {
            // Uncomment for debugging
            // for a in &s.annotation_infos {
            //     println!("{}", a.term);
            //     for (var, type_var) in &a.var_to_type_var {
            //         println!("{}: {:#?}", var, s.type_var_to_type[type_var]);
            //     }
            //     println!();
            // }
            solutions.insert(r.id, s);
        }
    }
    solutions
}

fn build_decl_map(defs: Defs) -> HashMap<String, Decl> {
    let mut decls = HashMap::new();
    for def in defs.defs {
        match def {
            isle::ast::Def::Decl(d) => {
                decls.insert(d.term.0.clone(), d);
            }
            _ => continue,
        }
    }
    decls
}

fn convert_type(aty: &annotation_ir::Type) -> veri_ir::Type {
    match aty {
        annotation_ir::Type::BitVectorUnknown(..) => veri_ir::Type::BitVector(None),
        annotation_ir::Type::BitVector => veri_ir::Type::BitVector(None),
        annotation_ir::Type::BitVectorWithWidth(w) => veri_ir::Type::BitVector(Some(*w)),
        annotation_ir::Type::Int => veri_ir::Type::Int,
        annotation_ir::Type::Bool => veri_ir::Type::Bool,
        annotation_ir::Type::Poly(_) => unreachable!(),
    }
}

fn type_annotations_using_rule<'a>(
    rule: &'a isle::sema::Rule,
    annotation_env: &'a AnnotationEnv,
    decls: &'a HashMap<String, isle::ast::Decl>,
    typeenv: &'a TypeEnv,
    termenv: &'a TermEnv,
) -> Option<Solution> {
    let mut parse_tree = RuleParseTree {
        varid_to_type_var_map: HashMap::new(),
        type_var_to_val_map: HashMap::new(),
        next_type_var: 1,
        concrete_constraints: HashSet::new(),
        var_constraints: HashSet::new(),
        bv_constraints: HashSet::new(),
        decls,
        ty_vars: HashMap::new(),
        quantified_vars: HashSet::new(),
        free_vars: HashSet::new(),
        assumptions: vec![],
    };

    let mut annotation_infos = vec![];
    if !rule.iflets.is_empty() {
        print!("\n\tif-lets:");
        for iflet in &rule.iflets {
            let iflet_lhs =
                &mut create_parse_tree_pattern(rule, &iflet.lhs, &mut parse_tree, typeenv, termenv);
            let iflet_rhs =
                &mut create_parse_tree_expr(rule, &iflet.rhs, &mut parse_tree, typeenv, termenv);

            let iflet_lhs_expr = add_rule_constraints(
                &mut parse_tree,
                iflet_lhs,
                annotation_env,
                &mut annotation_infos,
            );
            if iflet_lhs_expr.is_none() {
                return None;
            }

            let iflet_rhs_expr = add_rule_constraints(
                &mut parse_tree,
                iflet_rhs,
                annotation_env,
                &mut annotation_infos,
            );
            if iflet_rhs_expr.is_none() {
                return None;
            }
            parse_tree
                .var_constraints
                .insert(TypeExpr::Variable(iflet_lhs.type_var, iflet_rhs.type_var));
            parse_tree.assumptions.push(veri_ir::Expr::Binary(
                veri_ir::BinaryOp::Eq,
                Box::new(iflet_lhs_expr.unwrap()),
                Box::new(iflet_rhs_expr.unwrap()),
            ));
        }
        print!("\n");
    }
    let lhs = &mut create_parse_tree_pattern(
        rule,
        // Hack for now: typeid not used
        &isle::sema::Pattern::Term(
            cranelift_isle::sema::TypeId(0),
            rule.root_term,
            rule.args.clone(),
        ),
        &mut parse_tree,
        typeenv,
        termenv,
    );
    let rhs = &mut create_parse_tree_expr(rule, &rule.rhs, &mut parse_tree, typeenv, termenv);

    println!("Typing rule:");
    print!("\tLHS:");
    let lhs_expr =
        add_rule_constraints(&mut parse_tree, lhs, annotation_env, &mut annotation_infos);
    if lhs_expr.is_none() {
        return None;
    }
    print!("\n\tRHS:");
    let rhs_expr =
        add_rule_constraints(&mut parse_tree, rhs, annotation_env, &mut annotation_infos);
    if rhs_expr.is_none() {
        return None;
    }
    println!();

    match (lhs_expr, rhs_expr) {
        (Some(lhs_expr), Some(rhs_expr)) => {
            parse_tree
                .var_constraints
                .insert(TypeExpr::Variable(lhs.type_var, rhs.type_var));

            let (solution, bv_unknown_width_sets) = solve_constraints(
                parse_tree.concrete_constraints,
                parse_tree.var_constraints,
                parse_tree.bv_constraints,
            );

            let mut tymap = HashMap::new();

            for (expr, t) in &parse_tree.ty_vars {
                if let Some(ty) = solution.get(&t) {
                    tymap.insert(*t, convert_type(ty));
                } else {
                    panic!("missing type variable {} in solution for: {:?}", t, expr);
                }
            }
            let mut quantified_vars = vec![];
            for (s, t) in parse_tree.quantified_vars {
                let expr = veri_ir::Expr::Terminal(veri_ir::Terminal::Var(s.clone()));
                if let Some(ty) = solution.get(&t) {
                    let ty = convert_type(ty);
                    parse_tree.ty_vars.insert(expr, t);
                    tymap.insert(t, ty.clone());
                    quantified_vars.push(veri_ir::BoundVar { name: s, tyvar: t });
                } else {
                    panic!("missing type variable {} in solution for: {:?}", t, expr);
                }
            }
            let mut free_vars = vec![];
            for (s, t) in parse_tree.free_vars {
                let expr = veri_ir::Expr::Terminal(veri_ir::Terminal::Var(s.clone()));
                if let Some(ty) = solution.get(&t) {
                    let ty = convert_type(ty);
                    parse_tree.ty_vars.insert(expr, t);
                    tymap.insert(t, ty.clone());
                    free_vars.push(veri_ir::BoundVar { name: s, tyvar: t });
                } else {
                    panic!("missing type variable {} in solution for: {:?}", t, expr);
                }
            }

            Some(Solution {
                annotation_infos,
                type_var_to_type: solution,
                lhs: lhs_expr,
                rhs: rhs_expr,
                assumptions: parse_tree.assumptions,
                quantified_vars,
                free_vars,
                tyctx: TypeContext {
                    tyvars: parse_tree.ty_vars.clone(),
                    tymap,
                    tyvals: parse_tree.type_var_to_val_map,
                    bv_unknown_width_sets,
                },
            })
        }
        _ => None,
    }
}

fn add_annotation_constraints(
    expr: annotation_ir::Expr,
    tree: &mut RuleParseTree,
    annotation_info: &mut AnnotationTypeInfo,
) -> (veri_ir::Expr, u32) {
    let (e, t) = match expr {
        annotation_ir::Expr::Var(x, ..) => {
            let mut t = tree.next_type_var;
            if annotation_info.var_to_type_var.contains_key(&x) {
                t = annotation_info.var_to_type_var[&x];
            } else {
                annotation_info.var_to_type_var.insert(x.clone(), t);
                tree.next_type_var += 1;
            }
            let name = format!("{}__{}__{}", annotation_info.term, x, t);
            (veri_ir::Expr::Terminal(veri_ir::Terminal::Var(name)), t)
        }
        annotation_ir::Expr::Const(c, ..) => {
            let t = tree.next_type_var;
            let e = veri_ir::Expr::Terminal(veri_ir::Terminal::Const(c.value, t));
            match c.ty {
                annotation_ir::Type::BitVector => {
                    tree.bv_constraints
                        .insert(TypeExpr::Concrete(t, c.ty.clone()));
                }
                _ => {
                    tree.concrete_constraints
                        .insert(TypeExpr::Concrete(t, c.ty.clone()));
                }
            }
            tree.next_type_var += 1;

            // If constant is known, add the value to the tree. Useful for
            // capturing isleTypes
            tree.type_var_to_val_map.insert(t, c.value);
            (e, t)
        }

        annotation_ir::Expr::WidthOf(x, _) => {
            let (ex, tx) = add_annotation_constraints(*x.clone(), tree, annotation_info);
            let t = tree.next_type_var;
            tree.next_type_var += 1;
            tree.bv_constraints
                .insert(TypeExpr::Concrete(tx, annotation_ir::Type::BitVector));
            tree.concrete_constraints
                .insert(TypeExpr::Concrete(t, annotation_ir::Type::Int));
            (veri_ir::Expr::WidthOf(Box::new(ex)), t)
        }

        annotation_ir::Expr::Eq(x, y, _) => {
            let (e1, t1) = add_annotation_constraints(*x, tree, annotation_info);
            let (e2, t2) = add_annotation_constraints(*y, tree, annotation_info);
            let t = tree.next_type_var;

            tree.concrete_constraints
                .insert(TypeExpr::Concrete(t, annotation_ir::Type::Bool));
            tree.var_constraints.insert(TypeExpr::Variable(t1, t2));

            tree.next_type_var += 1;
            (
                veri_ir::Expr::Binary(veri_ir::BinaryOp::Eq, Box::new(e1), Box::new(e2)),
                t,
            )
        }
        annotation_ir::Expr::Lte(x, y, _) => {
            let (e1, t1) = add_annotation_constraints(*x, tree, annotation_info);
            let (e2, t2) = add_annotation_constraints(*y, tree, annotation_info);
            let t = tree.next_type_var;

            tree.concrete_constraints
                .insert(TypeExpr::Concrete(t, annotation_ir::Type::Bool));
            tree.var_constraints.insert(TypeExpr::Variable(t1, t2));

            tree.next_type_var += 1;
            (
                veri_ir::Expr::Binary(veri_ir::BinaryOp::Lte, Box::new(e1), Box::new(e2)),
                t,
            )
        }

        annotation_ir::Expr::Or(x, y, _) => {
            let (e1, t1) = add_annotation_constraints(*x, tree, annotation_info);
            let (e2, t2) = add_annotation_constraints(*y, tree, annotation_info);
            let t = tree.next_type_var;

            tree.concrete_constraints
                .insert(TypeExpr::Concrete(t, annotation_ir::Type::Bool));
            tree.concrete_constraints
                .insert(TypeExpr::Concrete(t1, annotation_ir::Type::Bool));
            tree.concrete_constraints
                .insert(TypeExpr::Concrete(t2, annotation_ir::Type::Bool));

            tree.next_type_var += 1;
            (
                veri_ir::Expr::Binary(veri_ir::BinaryOp::Or, Box::new(e1), Box::new(e2)),
                t,
            )
        }

        annotation_ir::Expr::BVNeg(x, _) => {
            let (e1, t1) = add_annotation_constraints(*x, tree, annotation_info);

            let t = tree.next_type_var;
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t, annotation_ir::Type::BitVector));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t1, annotation_ir::Type::BitVector));
            tree.var_constraints.insert(TypeExpr::Variable(t, t1));

            tree.next_type_var += 1;
            (
                veri_ir::Expr::Unary(veri_ir::UnaryOp::BVNeg, Box::new(e1)),
                t,
            )
        }

        annotation_ir::Expr::BVNot(x, _) => {
            let (e1, t1) = add_annotation_constraints(*x, tree, annotation_info);

            let t = tree.next_type_var;
            tree.var_constraints.insert(TypeExpr::Variable(t, t1));

            tree.next_type_var += 1;
            (
                veri_ir::Expr::Unary(veri_ir::UnaryOp::BVNot, Box::new(e1)),
                t,
            )
        }

        annotation_ir::Expr::BVMul(x, y, _) => {
            let (e1, t1) = add_annotation_constraints(*x, tree, annotation_info);
            let (e2, t2) = add_annotation_constraints(*y, tree, annotation_info);
            let t = tree.next_type_var;

            tree.bv_constraints
                .insert(TypeExpr::Concrete(t, annotation_ir::Type::BitVector));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t1, annotation_ir::Type::BitVector));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t2, annotation_ir::Type::BitVector));
            tree.var_constraints.insert(TypeExpr::Variable(t1, t2));
            tree.var_constraints.insert(TypeExpr::Variable(t, t1));
            tree.var_constraints.insert(TypeExpr::Variable(t, t2));

            tree.next_type_var += 1;
            (
                veri_ir::Expr::Binary(veri_ir::BinaryOp::BVMul, Box::new(e1), Box::new(e2)),
                t,
            )
        }
        annotation_ir::Expr::BVAdd(x, y, _) => {
            let (e1, t1) = add_annotation_constraints(*x, tree, annotation_info);
            let (e2, t2) = add_annotation_constraints(*y, tree, annotation_info);
            let t = tree.next_type_var;

            tree.bv_constraints
                .insert(TypeExpr::Concrete(t, annotation_ir::Type::BitVector));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t1, annotation_ir::Type::BitVector));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t2, annotation_ir::Type::BitVector));
            tree.var_constraints.insert(TypeExpr::Variable(t1, t2));
            tree.var_constraints.insert(TypeExpr::Variable(t, t1));
            tree.var_constraints.insert(TypeExpr::Variable(t, t2));

            tree.next_type_var += 1;
            (
                veri_ir::Expr::Binary(veri_ir::BinaryOp::BVAdd, Box::new(e1), Box::new(e2)),
                t,
            )
        }
        annotation_ir::Expr::BVSub(x, y, _) => {
            let (e1, t1) = add_annotation_constraints(*x, tree, annotation_info);
            let (e2, t2) = add_annotation_constraints(*y, tree, annotation_info);
            let t = tree.next_type_var;

            tree.bv_constraints
                .insert(TypeExpr::Concrete(t, annotation_ir::Type::BitVector));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t1, annotation_ir::Type::BitVector));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t2, annotation_ir::Type::BitVector));
            tree.var_constraints.insert(TypeExpr::Variable(t1, t2));
            tree.var_constraints.insert(TypeExpr::Variable(t, t1));
            tree.var_constraints.insert(TypeExpr::Variable(t, t2));

            tree.next_type_var += 1;
            (
                veri_ir::Expr::Binary(veri_ir::BinaryOp::BVSub, Box::new(e1), Box::new(e2)),
                t,
            )
        }
        annotation_ir::Expr::BVAnd(x, y, _) => {
            let (e1, t1) = add_annotation_constraints(*x, tree, annotation_info);
            let (e2, t2) = add_annotation_constraints(*y, tree, annotation_info);
            let t = tree.next_type_var;

            tree.bv_constraints
                .insert(TypeExpr::Concrete(t, annotation_ir::Type::BitVector));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t1, annotation_ir::Type::BitVector));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t2, annotation_ir::Type::BitVector));
            tree.var_constraints.insert(TypeExpr::Variable(t1, t2));
            tree.var_constraints.insert(TypeExpr::Variable(t, t1));
            tree.var_constraints.insert(TypeExpr::Variable(t, t2));

            tree.next_type_var += 1;
            (
                veri_ir::Expr::Binary(veri_ir::BinaryOp::BVAnd, Box::new(e1), Box::new(e2)),
                t,
            )
        }
        annotation_ir::Expr::BVOr(x, y, _) => {
            let (e1, t1) = add_annotation_constraints(*x, tree, annotation_info);
            let (e2, t2) = add_annotation_constraints(*y, tree, annotation_info);
            let t = tree.next_type_var;

            tree.bv_constraints
                .insert(TypeExpr::Concrete(t, annotation_ir::Type::BitVector));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t1, annotation_ir::Type::BitVector));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t2, annotation_ir::Type::BitVector));
            tree.var_constraints.insert(TypeExpr::Variable(t1, t2));
            tree.var_constraints.insert(TypeExpr::Variable(t, t1));
            tree.var_constraints.insert(TypeExpr::Variable(t, t2));

            tree.next_type_var += 1;
            (
                veri_ir::Expr::Binary(veri_ir::BinaryOp::BVOr, Box::new(e1), Box::new(e2)),
                t,
            )
        }
        annotation_ir::Expr::BVXor(x, y, _) => {
            let (e1, t1) = add_annotation_constraints(*x, tree, annotation_info);
            let (e2, t2) = add_annotation_constraints(*y, tree, annotation_info);
            let t = tree.next_type_var;

            tree.bv_constraints
                .insert(TypeExpr::Concrete(t, annotation_ir::Type::BitVector));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t1, annotation_ir::Type::BitVector));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t2, annotation_ir::Type::BitVector));
            tree.var_constraints.insert(TypeExpr::Variable(t1, t2));
            tree.var_constraints.insert(TypeExpr::Variable(t, t1));
            tree.var_constraints.insert(TypeExpr::Variable(t, t2));

            tree.next_type_var += 1;
            (
                veri_ir::Expr::Binary(veri_ir::BinaryOp::BVXor, Box::new(e1), Box::new(e2)),
                t,
            )
        }
        annotation_ir::Expr::BVRotl(x, a, _) => {
            let (xe, xt) = add_annotation_constraints(*x, tree, annotation_info);
            let (ae, at) = add_annotation_constraints(*a, tree, annotation_info);
            let t = tree.next_type_var;
            tree.next_type_var += 1;

            tree.bv_constraints
                .insert(TypeExpr::Concrete(xt, annotation_ir::Type::BitVector));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(at, annotation_ir::Type::BitVector));
            tree.var_constraints.insert(TypeExpr::Variable(t, xt));

            (
                veri_ir::Expr::Binary(veri_ir::BinaryOp::BVRotl, Box::new(xe), Box::new(ae)),
                t,
            )
        }
        annotation_ir::Expr::BVRotr(x, a, _) => {
            let (xe, xt) = add_annotation_constraints(*x, tree, annotation_info);
            let (ae, at) = add_annotation_constraints(*a, tree, annotation_info);
            let t = tree.next_type_var;
            tree.next_type_var += 1;

            tree.bv_constraints
                .insert(TypeExpr::Concrete(xt, annotation_ir::Type::BitVector));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(at, annotation_ir::Type::BitVector));
            tree.var_constraints.insert(TypeExpr::Variable(t, xt));

            (
                veri_ir::Expr::Binary(veri_ir::BinaryOp::BVRotr, Box::new(xe), Box::new(ae)),
                t,
            )
        }
        annotation_ir::Expr::BVShr(x, a, _) => {
            let (xe, xt) = add_annotation_constraints(*x, tree, annotation_info);
            let (ae, at) = add_annotation_constraints(*a, tree, annotation_info);
            let t = tree.next_type_var;
            tree.next_type_var += 1;

            tree.bv_constraints
                .insert(TypeExpr::Concrete(xt, annotation_ir::Type::BitVector));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(at, annotation_ir::Type::BitVector));
            tree.var_constraints.insert(TypeExpr::Variable(t, xt));

            (
                veri_ir::Expr::Binary(veri_ir::BinaryOp::BVShr, Box::new(xe), Box::new(ae)),
                t,
            )
        }
        annotation_ir::Expr::BVShl(x, a, _) => {
            let (xe, xt) = add_annotation_constraints(*x, tree, annotation_info);
            let (ae, at) = add_annotation_constraints(*a, tree, annotation_info);
            let t = tree.next_type_var;
            tree.next_type_var += 1;

            tree.bv_constraints
                .insert(TypeExpr::Concrete(xt, annotation_ir::Type::BitVector));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(at, annotation_ir::Type::BitVector));
            tree.var_constraints.insert(TypeExpr::Variable(t, xt));

            (
                veri_ir::Expr::Binary(veri_ir::BinaryOp::BVShl, Box::new(xe), Box::new(ae)),
                t,
            )
        }
        annotation_ir::Expr::BVConvTo(w, x, _) => {
            let (e1, t1) = add_annotation_constraints(*x, tree, annotation_info);
            let t = tree.next_type_var;
            tree.next_type_var += 1;

            let width = match *w {
                annotation_ir::Width::Const(x) => x,
                annotation_ir::Width::RegWidth => REG_WIDTH,
            };

            tree.concrete_constraints.insert(TypeExpr::Concrete(
                t,
                annotation_ir::Type::BitVectorWithWidth(width),
            ));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t1, annotation_ir::Type::BitVector));

            (veri_ir::Expr::BVConvTo(Box::new(e1)), t)
        }
        annotation_ir::Expr::BVConvToVarWidth(w, x, _) => {
            let (we, wt) = add_annotation_constraints(*w, tree, annotation_info);
            let (e1, t1) = add_annotation_constraints(*x, tree, annotation_info);
            let t = tree.next_type_var;

            // In the dynamic case, we don't know the width at this point
            tree.concrete_constraints
                .insert(TypeExpr::Concrete(wt, annotation_ir::Type::Int));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t1, annotation_ir::Type::BitVector));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t, annotation_ir::Type::BitVector));

            tree.next_type_var += 1;

            (
                veri_ir::Expr::BVConvToVarWidth(Box::new(we), Box::new(e1)),
                t,
            )
        }
        annotation_ir::Expr::BVSignExtToVarWidth(w, x, _) => {
            let (we, wt) = add_annotation_constraints(*w, tree, annotation_info);
            let (e1, t1) = add_annotation_constraints(*x, tree, annotation_info);
            let t = tree.next_type_var;

            // In the dynamic case, we don't know the width at this point
            tree.concrete_constraints
                .insert(TypeExpr::Concrete(wt, annotation_ir::Type::Int));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t1, annotation_ir::Type::BitVector));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t, annotation_ir::Type::BitVector));

            tree.next_type_var += 1;

            (
                veri_ir::Expr::BVSignExtToVarWidth(Box::new(we), Box::new(e1)),
                t,
            )
        }
        annotation_ir::Expr::BVZeroExtTo(w, x, _) => {
            let (e1, t1) = add_annotation_constraints(*x, tree, annotation_info);
            let t = tree.next_type_var;

            let width = match *w {
                veri_ir::annotation_ir::Width::Const(c) => c,
                veri_ir::annotation_ir::Width::RegWidth => REG_WIDTH,
            };

            tree.bv_constraints
                .insert(TypeExpr::Concrete(t1, annotation_ir::Type::BitVector));
            tree.concrete_constraints.insert(TypeExpr::Concrete(
                t,
                annotation_ir::Type::BitVectorWithWidth(width),
            ));

            tree.next_type_var += 1;

            (veri_ir::Expr::BVZeroExtTo(width, Box::new(e1)), t)
        }
        annotation_ir::Expr::BVZeroExtToVarWidth(w, x, _) => {
            let (we, wt) = add_annotation_constraints(*w, tree, annotation_info);
            let (e1, t1) = add_annotation_constraints(*x, tree, annotation_info);
            let t = tree.next_type_var;

            // In the dynamic case, we don't know the width at this point
            tree.concrete_constraints
                .insert(TypeExpr::Concrete(wt, annotation_ir::Type::Int));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t1, annotation_ir::Type::BitVector));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t, annotation_ir::Type::BitVector));

            tree.next_type_var += 1;

            (
                veri_ir::Expr::BVZeroExtToVarWidth(Box::new(we), Box::new(e1)),
                t,
            )
        }
        annotation_ir::Expr::BVSignExtTo(w, x, _) => {
            let (e1, t1) = add_annotation_constraints(*x, tree, annotation_info);
            let t = tree.next_type_var;

            let width = match *w {
                veri_ir::annotation_ir::Width::Const(c) => c,
                veri_ir::annotation_ir::Width::RegWidth => REG_WIDTH,
            };

            tree.bv_constraints
                .insert(TypeExpr::Concrete(t1, annotation_ir::Type::BitVector));
            tree.concrete_constraints.insert(TypeExpr::Concrete(
                t,
                annotation_ir::Type::BitVectorWithWidth(width),
            ));

            tree.next_type_var += 1;

            (veri_ir::Expr::BVSignExtTo(width, Box::new(e1)), t)
        }
        annotation_ir::Expr::BVExtract(l, r, x, _) => {
            let (e1, t1) = add_annotation_constraints(*x, tree, annotation_info);
            let t = tree.next_type_var;

            tree.bv_constraints
                .insert(TypeExpr::Concrete(t1, annotation_ir::Type::BitVector));
            tree.concrete_constraints.insert(TypeExpr::Concrete(
                t,
                annotation_ir::Type::BitVectorWithWidth(l - r + 1),
            ));

            tree.next_type_var += 1;

            (veri_ir::Expr::BVExtract(l, r, Box::new(e1)), t)
        }
        annotation_ir::Expr::BVIntToBv(w, x, _) => {
            let (ex, tx) = add_annotation_constraints(*x.clone(), tree, annotation_info);

            let t = tree.next_type_var;
            tree.next_type_var += 1;

            tree.concrete_constraints
                .insert(TypeExpr::Concrete(tx, annotation_ir::Type::Int));

            tree.concrete_constraints.insert(TypeExpr::Concrete(
                t,
                annotation_ir::Type::BitVectorWithWidth(w),
            ));

            (veri_ir::Expr::BVIntToBV(w, Box::new(ex)), t)
        }
        annotation_ir::Expr::Conditional(c, t, e, _) => {
            let (e1, t1) = add_annotation_constraints(*c, tree, annotation_info);
            let (e2, t2) = add_annotation_constraints(*t, tree, annotation_info);
            let (e3, t3) = add_annotation_constraints(*e, tree, annotation_info);
            let t = tree.next_type_var;

            tree.concrete_constraints
                .insert(TypeExpr::Concrete(t1, annotation_ir::Type::Bool));
            tree.var_constraints.insert(TypeExpr::Variable(t2, t3));
            tree.var_constraints.insert(TypeExpr::Variable(t, t2));

            tree.next_type_var += 1;
            (
                veri_ir::Expr::Conditional(Box::new(e1), Box::new(e2), Box::new(e3)),
                t,
            )
        }
        annotation_ir::Expr::CLZ(x, _) => {
            let (e1, t1) = add_annotation_constraints(*x, tree, annotation_info);

            let t = tree.next_type_var;
            // TODO: currently we're making the return type of clz some
            // bv with unspecified length. Since we include an equality check
            // at the end of the clz operation, it's implied that we can only
            // compare against other bv types (hopefully with known length)
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t, annotation_ir::Type::BitVector));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t1, annotation_ir::Type::BitVector));
            tree.var_constraints.insert(TypeExpr::Variable(t, t1));

            tree.next_type_var += 1;
            (veri_ir::Expr::CLZ(Box::new(e1)), t)
        }
        annotation_ir::Expr::A64CLZ(ty, x, _) => {
            let (e0, t0) = add_annotation_constraints(*ty, tree, annotation_info);
            let (e1, t1) = add_annotation_constraints(*x, tree, annotation_info);

            let t = tree.next_type_var;
            tree.concrete_constraints.insert(TypeExpr::Concrete(
                t,
                annotation_ir::Type::BitVectorWithWidth(REG_WIDTH),
            ));
            tree.concrete_constraints
                .insert(TypeExpr::Concrete(t0, annotation_ir::Type::Int));
            tree.bv_constraints
                .insert(TypeExpr::Concrete(t1, annotation_ir::Type::BitVector));

            tree.next_type_var += 1;
            (veri_ir::Expr::A64CLZ(Box::new(e0), Box::new(e1)), t)
        }
        _ => todo!("expr {:#?} not yet implemented", expr),
    };
    tree.ty_vars.insert(e.clone(), t);
    (e, t)
}

fn add_isle_constraints(
    def: isle::ast::Def,
    tree: &mut RuleParseTree,
    annotation_info: &mut AnnotationTypeInfo,
    annotation: annotation_ir::TermSignature,
) {
    let clif_to_ir_types = HashMap::from([
        ("Type".to_owned(), annotation_ir::Type::Int),
        (
            "Imm12".to_owned(),
            annotation_ir::Type::BitVectorWithWidth(12),
        ),
        (
            "Imm64".to_owned(),
            annotation_ir::Type::BitVectorWithWidth(64),
        ),
        (
            "ImmShift".to_owned(),
            annotation_ir::Type::BitVectorWithWidth(6),
        ),
        (
            "ImmLogic".to_owned(),
            annotation_ir::Type::BitVectorWithWidth(12),
        ),
        (
            "u64".to_owned(),
            annotation_ir::Type::BitVectorWithWidth(64),
        ),
        ("u8".to_owned(), annotation_ir::Type::BitVectorWithWidth(8)),
        // AVH TODO: needed for lower from rolt to small_rotr, but should rework.
        ("usize".to_owned(), annotation_ir::Type::BitVector),
        ("bool".to_owned(), annotation_ir::Type::Bool),
        (
            "MoveWideConst".to_owned(),
            annotation_ir::Type::BitVectorWithWidth(16),
        ),
        ("OperandSize".to_owned(), annotation_ir::Type::BitVector),
        ("Reg".to_owned(), annotation_ir::Type::BitVector),
        ("Inst".to_owned(), annotation_ir::Type::BitVector),
        ("Value".to_owned(), annotation_ir::Type::BitVector),
        ("ValueRegs".to_owned(), annotation_ir::Type::BitVector),
        ("InstOutput".to_owned(), annotation_ir::Type::BitVector),
    ]);

    let mut annotation_vars = vec![];
    for a in annotation.args {
        annotation_vars.push(a.name);
    }
    annotation_vars.push(annotation.ret.name);

    match def {
        isle::ast::Def::Decl(d) => {
            let mut isle_types = vec![];
            for t in d.arg_tys {
                isle_types.push(t.0);
            }
            isle_types.push(d.ret_ty.0);
            assert_eq!(annotation_vars.len(), isle_types.len());

            for (isle_type, annotation_var) in isle_types.iter().zip(annotation_vars) {
                // in case the var was not in the annotation
                if !annotation_info
                    .var_to_type_var
                    .contains_key(&annotation_var)
                {
                    let type_var = tree.next_type_var;
                    tree.next_type_var += 1;

                    annotation_info
                        .var_to_type_var
                        .insert(annotation_var.clone(), type_var);
                }

                if let Some(ir_type) = clif_to_ir_types.get(isle_type) {
                    let type_var = annotation_info.var_to_type_var[&annotation_var];
                    match ir_type {
                        annotation_ir::Type::BitVector => tree
                            .bv_constraints
                            .insert(TypeExpr::Concrete(type_var, ir_type.clone())),
                        _ => tree
                            .concrete_constraints
                            .insert(TypeExpr::Concrete(type_var, ir_type.clone())),
                    };
                }
            }
        }
        _ => panic!("def must be decl, got: {:#?}", def),
    }
}

fn add_rule_constraints(
    tree: &mut RuleParseTree,
    curr: &mut TypeVarNode,
    annotation_env: &AnnotationEnv,
    annotation_infos: &mut Vec<AnnotationTypeInfo>,
) -> Option<veri_ir::Expr> {
    // Only relate args to annotations for terms. For leaves, return immediately.
    // For recursive definitions without annotations (like And and Let), recur.
    let mut children = vec![];
    for child in &mut curr.children {
        if let Some(e) = add_rule_constraints(tree, child, annotation_env, annotation_infos) {
            children.push(e);
        } else {
            return None;
        }
    }
    let e = match &curr.construct {
        TypeVarConstruct::Var => {
            tree.quantified_vars
                .insert((curr.ident.clone(), curr.type_var));
            tree.free_vars.insert((curr.ident.clone(), curr.type_var));
            Some(veri_ir::Expr::Terminal(veri_ir::Terminal::Var(
                curr.ident.clone(),
            )))
        }
        TypeVarConstruct::BindPattern => {
            assert_eq!(children.len(), 2);
            // tree.quantified_vars
            //     .insert((curr.ident.clone(), curr.type_var));
            // tree.free_vars.insert((curr.ident.clone(), curr.type_var));
            tree.assumptions.push(veri_ir::Expr::Binary(
                veri_ir::BinaryOp::Eq,
                Box::new(children[0].clone()),
                Box::new(children[1].clone()),
            ));
            Some(children[0].clone())
        }
        TypeVarConstruct::Wildcard => Some(veri_ir::Expr::Terminal(veri_ir::Terminal::Wildcard)),
        TypeVarConstruct::Const(i) => {
            // If constant is known, add the value to the tree. Useful for
            // capturing isleTypes
            tree.type_var_to_val_map.insert(curr.type_var, *i);

            Some(veri_ir::Expr::Terminal(veri_ir::Terminal::Const(
                *i,
                curr.type_var,
            )))
        }
        TypeVarConstruct::And => {
            tree.quantified_vars
                .insert((curr.ident.clone(), curr.type_var));
            let first = &children[0];
            for (i, e) in children.iter().enumerate() {
                if i != 0 {
                    tree.assumptions.push(veri_ir::Expr::Binary(
                        veri_ir::BinaryOp::Eq,
                        Box::new(first.clone()),
                        Box::new(e.clone()),
                    ))
                }
            }
            Some(first.to_owned())
        }
        TypeVarConstruct::Let(bound) => {
            tree.quantified_vars
                .insert((curr.ident.clone(), curr.type_var));
            for (e, s) in children.iter().zip(bound) {
                tree.assumptions.push(veri_ir::Expr::Binary(
                    veri_ir::BinaryOp::Eq,
                    Box::new(veri_ir::Expr::Terminal(veri_ir::Terminal::Var(
                        s.to_owned(),
                    ))),
                    Box::new(e.to_owned()),
                ))
            }
            children.last().cloned()
        }
        TypeVarConstruct::Term(t) => {
            // Print term for debugging
            print!(" {}", t);
            tree.quantified_vars
                .insert((curr.ident.clone(), curr.type_var));
            let a = annotation_env.get_annotation_for_term(t);
            if a.is_none() {
                println!("\nSKIPPING RULE with unannotated term: {}", t);
                return None;
            }
            let annotation = a.unwrap();

            // use a fresh mapping for each term
            // keep the same mapping between assertions in the same annotation
            let mut annotation_info = AnnotationTypeInfo {
                term: curr.ident.clone(),
                var_to_type_var: HashMap::new(),
            };
            for expr in annotation.assertions {
                let (typed_expr, _) = add_annotation_constraints(*expr, tree, &mut annotation_info);
                curr.assertions.push(typed_expr.clone());
                tree.assumptions.push(typed_expr);
                if tree.decls.contains_key(t) {
                    add_isle_constraints(
                        cranelift_isle::ast::Def::Decl(tree.decls[t].clone()),
                        tree,
                        &mut annotation_info,
                        annotation.sig.clone(),
                    );
                }
            }

            // set args in rule equal to args in annotation
            for (child, arg) in curr.children.iter().zip(&annotation.sig.args) {
                let rule_type_var = child.type_var;
                let annotation_type_var = annotation_info.var_to_type_var[&arg.name];
                tree.var_constraints
                    .insert(TypeExpr::Variable(rule_type_var, annotation_type_var));
            }

            for (child, arg) in children.iter().zip(&annotation.sig.args) {
                let annotation_type_var = annotation_info.var_to_type_var[&arg.name];
                let arg_name = format!(
                    "{}__{}__{}",
                    annotation_info.term, arg.name, annotation_type_var
                );
                tree.quantified_vars
                    .insert((arg_name.clone(), annotation_type_var));
                tree.assumptions.push(veri_ir::Expr::Binary(
                    veri_ir::BinaryOp::Eq,
                    Box::new(child.clone()),
                    Box::new(veri_ir::Expr::Terminal(veri_ir::Terminal::Var(arg_name))),
                ))
            }
            // set term ret var equal to annotation ret var
            let ret_var = annotation_info.var_to_type_var[&annotation.sig.ret.name];
            tree.var_constraints
                .insert(TypeExpr::Variable(curr.type_var, ret_var));
            let ret_name = format!(
                "{}__{}__{}",
                annotation_info.term, annotation.sig.ret.name, ret_var
            );
            tree.quantified_vars.insert((ret_name.clone(), ret_var));
            tree.assumptions.push(veri_ir::Expr::Binary(
                veri_ir::BinaryOp::Eq,
                Box::new(veri_ir::Expr::Terminal(veri_ir::Terminal::Var(
                    curr.ident.clone(),
                ))),
                Box::new(veri_ir::Expr::Terminal(veri_ir::Terminal::Var(ret_name))),
            ));

            annotation_infos.push(annotation_info);

            Some(veri_ir::Expr::Terminal(veri_ir::Terminal::Var(
                curr.ident.clone(),
            )))
        }
    };
    if let Some(e) = e {
        tree.ty_vars.insert(e.clone(), curr.type_var);
        Some(e)
    } else {
        None
    }
}

// Solve constraints as follows:
//   - process concrete constraints first
//   - then process variable constraints
//   - constraints involving bv without widths are last priority
//
// for example:
//   t2 = bv16
//   t3 = bv8
//
//   t5 = t4
//   t6 = t1
//   t3 = t4
//   t1 = t2
//   t7 = t8
//
//   t4 = bv
//   t1 = bv
//   t7 = bv
//
// would result in:
//   bv16 -> t2, t6, t1
//   bv8 -> t3, t5, t4
//   poly(0) -> t5, t4 (intermediate group that gets removed)
//   poly(1) -> t6, t1 (intermediate group that gets removed)
//   poly(2) -> t7, t8 (intermediate group that gets removed)
//   bv -> t7, t8

// TODO: clean up
fn solve_constraints(
    concrete: HashSet<TypeExpr>,
    var: HashSet<TypeExpr>,
    bv: HashSet<TypeExpr>,
) -> (HashMap<u32, annotation_ir::Type>, HashMap<u32, u32>) {
    // maintain a union find that maps types to sets of type vars that have that type
    let mut union_find = HashMap::new();
    let mut poly = 0;

    // initialize union find with groups corresponding to concrete constraints
    for c in concrete {
        match c {
            TypeExpr::Concrete(v, t) => {
                if !union_find.contains_key(&t) {
                    union_find.insert(t.clone(), HashSet::new());
                }
                if let Some(group) = union_find.get_mut(&t) {
                    group.insert(v);
                }
            }
            _ => panic!(
                "Non-concrete constraint found in concrete constraints: {:#?}",
                c
            ),
        };
    }

    // process variable constraints as follows:
    //   if t1 = t2 and only t1 has been typed, add t2 to the same set as t1
    //   if t1 = t2 and only t2 has been typed, add t1 to the same set t2
    //   if t1 = t2 and neither has been typed, create a new poly type and add both to the set
    //   if t1 = t2 and both have been typed, union appropriately
    for v in var {
        match v {
            TypeExpr::Variable(v1, v2) => {
                let t1 = get_var_type(v1, &union_find);
                let t2 = get_var_type(v2, &union_find);

                match (t1, t2) {
                    (Some(x), Some(y)) => {
                        match (x.is_poly(), y.is_poly()) {
                            (false, false) => {
                                if x != y {
                                    panic!(
                                        "type conflict at constraint {:#?}: t{} has type {:#?}, t{} has type {:#?}",
                                        v, v1, x, v2, y
                                    )
                                }
                            }
                            // union t1 and t2, keeping t2 as the leader
                            (true, false) => {
                                let g1 = union_find.remove(&x).expect("expected key in union find");
                                let g2 =
                                    union_find.get_mut(&y).expect("expected key in union find");
                                g2.extend(g1.iter());
                            }
                            // union t1 and t2, keeping t1 as the leader
                            (_, true) => {
                                // guard against the case where x and y have the same poly type
                                // so we remove the key and can't find it in the next line
                                if x != y {
                                    let g2 =
                                        union_find.remove(&y).expect("expected key in union find");
                                    let g1 =
                                        union_find.get_mut(&x).expect("expected key in union find");
                                    g1.extend(g2.iter());
                                }
                            }
                        };
                    }
                    (Some(x), None) => {
                        if let Some(group) = union_find.get_mut(&x) {
                            group.insert(v2);
                        }
                    }
                    (None, Some(x)) => {
                        if let Some(group) = union_find.get_mut(&x) {
                            group.insert(v1);
                        }
                    }
                    (None, None) => {
                        let t = annotation_ir::Type::Poly(poly);
                        union_find.insert(t.clone(), HashSet::new());
                        if let Some(group) = union_find.get_mut(&t) {
                            group.insert(v1);
                            group.insert(v2);
                        }
                        poly += 1;
                    }
                }
            }
            _ => panic!("Non-variable constraint found in var constraints: {:#?}", v),
        }
    }

    for b in bv {
        match b {
            TypeExpr::Concrete(v, ref t) => {
                match t {
                    annotation_ir::Type::BitVector => {
                        // if there's a bv constraint and the var has already
                        // been typed (with a width), ignore the constraint
                        if let Some(var_type) = get_var_type_concrete(v, &union_find) {
                            match var_type {
                                annotation_ir::Type::BitVectorWithWidth(_) => {
                                    continue;
                                }
                                annotation_ir::Type::BitVectorUnknown(_) => {
                                    continue;
                                }
                                _ => panic!("Var was already typed as {:#?} but currently processing constraint: {:#?}", var_type, b),
                            }

                        // otherwise add it to a generic bv bucket
                        } else {
                            // if !union_find.contains_key(t) {
                            //     union_find.insert(t.clone(), HashSet::new());
                            // }
                            // if let Some(group) = union_find.get_mut(t) {
                            //     group.insert(v);
                            // }
                            let unknown_by_tyvar = annotation_ir::Type::BitVectorUnknown(v);
                            let mut set = HashSet::new();
                            set.insert(v);
                            union_find.insert(unknown_by_tyvar.clone(), set);

                            // if this type var also has a polymorphic type, union
                            if let Some(var_type) = get_var_type_poly(v, &union_find) {
                                let poly_bucket = union_find
                                    .remove(&var_type)
                                    .expect("expected key in union find");
                                let bv_bucket = union_find
                                    .get_mut(&unknown_by_tyvar)
                                    .expect("expected key in union find");
                                bv_bucket.extend(poly_bucket.iter());
                            }
                        }
                    }
                    _ => panic!("Non-bv constraint found in bv constraints: {:#?}", b),
                }
            }
            TypeExpr::Variable(_, _) => {
                panic!("Non-bv constraint found in bv constraints: {:#?}", b)
            }
        }
    }

    let mut result = HashMap::new();
    let mut bv_unknown_width_sets = HashMap::new();
    let mut bv_unknown_width_idx = 0u32;
    for (t, vars) in union_find {
        for var in &vars {
            result.insert(*var, t.clone());
        }
        if matches!(t, annotation_ir::Type::BitVectorUnknown(..)) {
            for var in &vars {
                bv_unknown_width_sets.insert(*var, bv_unknown_width_idx);
            }
            bv_unknown_width_idx += 1;
        }
    }
    (result, bv_unknown_width_sets)
}

// if the union find already contains the type var, return its type
// otherwise return None
fn get_var_type(
    t: u32,
    u: &HashMap<annotation_ir::Type, HashSet<u32>>,
) -> Option<annotation_ir::Type> {
    for (ty, vars) in u {
        if vars.contains(&t) {
            return Some(ty.clone());
        }
    }
    None
}

// If the union find contains the type var and it has a non-polymorphic, specific type
// return it. Otherwise return None.
fn get_var_type_concrete(
    t: u32,
    u: &HashMap<annotation_ir::Type, HashSet<u32>>,
) -> Option<annotation_ir::Type> {
    for (ty, vars) in u {
        match ty {
            annotation_ir::Type::Poly(_) | annotation_ir::Type::BitVector => continue,
            _ => {
                if vars.contains(&t) {
                    return Some(ty.clone());
                }
            }
        }
    }
    None
}

// If the union find contains the type var and it has a polymorphic type,
// return the polymorphic type. Otherwise return None.
fn get_var_type_poly(
    t: u32,
    u: &HashMap<annotation_ir::Type, HashSet<u32>>,
) -> Option<annotation_ir::Type> {
    for (ty, vars) in u {
        match ty {
            annotation_ir::Type::Poly(_) => {
                if vars.contains(&t) {
                    return Some(ty.clone());
                }
            }
            _ => continue,
        }
    }
    None
}

fn create_parse_tree_pattern(
    rule: &isle::sema::Rule,
    pattern: &isle::sema::Pattern,
    tree: &mut RuleParseTree,
    typeenv: &TypeEnv,
    termenv: &TermEnv,
) -> TypeVarNode {
    match pattern {
        isle::sema::Pattern::Term(_, term_id, args) => {
            let sym = termenv.terms[term_id.index()].name;
            let name = typeenv.syms[sym.index()].clone();

            // process children first
            let mut children = vec![];
            for arg in args {
                let child = create_parse_tree_pattern(rule, arg, tree, typeenv, termenv);
                children.push(child);
            }
            let type_var = tree.next_type_var;
            tree.next_type_var += 1;

            TypeVarNode {
                ident: format!("{}__{}", name, type_var),
                construct: TypeVarConstruct::Term(name),
                type_var,
                children,
                assertions: vec![],
            }
        }
        isle::sema::Pattern::Var(_, var_id) => {
            let sym = rule.vars[var_id.index()].name;
            let ident = typeenv.syms[sym.index()].clone();

            let type_var = tree
                .varid_to_type_var_map
                .entry(var_id.clone())
                .or_insert(tree.next_type_var);
            if *type_var == tree.next_type_var {
                tree.next_type_var += 1;
            }
            let ident = format!("{}__{}", ident, *type_var);
            // this is a base case so there are no children
            TypeVarNode {
                ident,
                construct: TypeVarConstruct::Var,
                type_var: *type_var,
                children: vec![],
                assertions: vec![],
            }
        }
        isle::sema::Pattern::BindPattern(_, var_id, subpat) => {
            let sym = rule.vars[var_id.index()].name;
            let var_ident = format!("{}_clif_{}", typeenv.syms[sym.index()], var_id.index());

            let var_type_var = tree.next_type_var;
            tree.next_type_var += 1;

            tree.varid_to_type_var_map.insert(var_id.clone(), var_type_var);

            let ident = format!("{}__{}", var_ident, var_type_var);
            // this is a base case so there are no children
            let var_node = TypeVarNode {
                ident: ident.clone(),
                construct: TypeVarConstruct::Var,
                type_var: var_type_var,
                children: vec![],
                assertions: vec![],
            };

            let subpat_node = create_parse_tree_pattern(rule, subpat, tree, typeenv, termenv);

            let type_var = tree.next_type_var;
            tree.next_type_var += 1;

            tree.var_constraints
                .insert(TypeExpr::Variable(var_type_var, subpat_node.type_var));
            tree.var_constraints
            .insert(TypeExpr::Variable(type_var, var_type_var));

            let ident = format!("bind_{}", ident);

            TypeVarNode {
                ident,
                construct: TypeVarConstruct::BindPattern,
                type_var: type_var,
                children: vec![var_node, subpat_node],
                assertions: vec![],
            }
        }
        isle::sema::Pattern::Wildcard(_) => {
            let type_var = tree.next_type_var;
            tree.next_type_var += 1;
            TypeVarNode {
                ident: String::from(""),
                construct: TypeVarConstruct::Wildcard,
                type_var: type_var,
                children: vec![],
                assertions: vec![],
            }
        }
        isle::sema::Pattern::ConstPrim(_, sym) => {
            let type_var = tree.next_type_var;
            tree.next_type_var += 1;
            let name = typeenv.syms[sym.index()].clone();
            let val = match name.as_str() {
                "I64" => 64,
                "I32" => 32,
                "I16" => 16,
                "I8" => 8,
                _ => todo!("{:?}", &name),
            };
            let name = format!("{}__{}", name, type_var);

            TypeVarNode {
                ident: name,
                construct: TypeVarConstruct::Const(val),
                type_var,
                children: vec![],
                assertions: vec![],
            }
        }
        isle::sema::Pattern::ConstInt(_, num) => {
            let type_var = tree.next_type_var;
            tree.next_type_var += 1;
            let name = format!("{}__{}", num, type_var);
            TypeVarNode {
                ident: name,
                construct: TypeVarConstruct::Const(*num as i128),
                type_var,
                children: vec![],
                assertions: vec![],
            }
        }
        isle::sema::Pattern::And(_, subpats) => {
            let mut children = vec![];
            let mut ty_vars = vec![];
            for p in subpats {
                let child = create_parse_tree_pattern(rule, p, tree, typeenv, termenv);
                ty_vars.push(child.type_var);
                children.push(child);
            }
            let type_var = tree.next_type_var;
            tree.next_type_var += 1;

            // Assert all sub type constraints are equivalent to the first subexpression
            let first = ty_vars[0];
            for e in &ty_vars[1..] {
                tree.var_constraints
                    .insert(TypeExpr::Variable(first, e.to_owned()));
            }

            TypeVarNode {
                ident: String::from("and"),
                construct: TypeVarConstruct::And,
                type_var,
                children,
                assertions: vec![],
            }
        }
    }
}

fn create_parse_tree_expr(
    rule: &isle::sema::Rule,
    expr: &isle::sema::Expr,
    tree: &mut RuleParseTree,
    typeenv: &TypeEnv,
    termenv: &TermEnv,
) -> TypeVarNode {
    match expr {
        isle::sema::Expr::Term(_, term_id, args) => {
            let sym = termenv.terms[term_id.index()].name;
            let name = typeenv.syms[sym.index()].clone();

            // process children first
            let mut children = vec![];
            for arg in args {
                let child = create_parse_tree_expr(rule, arg, tree, typeenv, termenv);
                children.push(child);
            }
            let type_var = tree.next_type_var;
            tree.next_type_var += 1;

            TypeVarNode {
                ident: format!("{}__{}", name, type_var),
                construct: TypeVarConstruct::Term(name),
                type_var,
                children,
                assertions: vec![],
            }
        }
        isle::sema::Expr::Var(_, var_id) => {
            let mut ident = var_id.0.to_string();
            if var_id.index() < rule.vars.len() {
                let sym = rule.vars[var_id.index()].name;
                ident = typeenv.syms[sym.index()].clone();
            } else {
                println!("var {} not found, using var id instead", var_id.0);
                ident = format!("v{}", ident);
            }

            let type_var = tree
                .varid_to_type_var_map
                .entry(var_id.clone())
                .or_insert(tree.next_type_var);
            if *type_var == tree.next_type_var {
                tree.next_type_var += 1;
            }
            let ident = format!("{}__{}", ident, *type_var);
            // this is a base case so there are no children
            TypeVarNode {
                ident,
                construct: TypeVarConstruct::Var,
                type_var: *type_var,
                children: vec![],
                assertions: vec![],
            }
        }
        isle::sema::Expr::ConstPrim(_, sym) => {
            let type_var = tree.next_type_var;
            tree.next_type_var += 1;
            let name = typeenv.syms[sym.index()].clone();
            let val = match name.as_str() {
                "I64" => 64,
                "I32" => 32,
                "false" => 0,
                "true" => 1,
                _ => todo!("{:?}", &name),
            };
            let name = format!("{}__{}", name, type_var);
            TypeVarNode {
                ident: name.clone(),
                construct: TypeVarConstruct::Const(val),
                type_var,
                children: vec![],
                assertions: vec![],
            }
        }
        isle::sema::Expr::ConstInt(_, num) => {
            let type_var = tree.next_type_var;
            tree.next_type_var += 1;
            let name = format!("{}__{}", num, type_var);
            TypeVarNode {
                ident: name,
                construct: TypeVarConstruct::Const(*num as i128),
                type_var,
                children: vec![],
                assertions: vec![],
            }
        }
        isle::sema::Expr::Let { bindings, body, .. } => {
            let mut children = vec![];
            let mut bound = vec![];
            for (varid, sym, expr) in bindings {
                let var = typeenv.syms[sym.index()].clone();
                let subpat_node = create_parse_tree_expr(rule, expr, tree, typeenv, termenv);

                let ty_var = tree.next_type_var;
                tree.next_type_var += 1;

                tree.varid_to_type_var_map.insert(varid.clone(), ty_var);
                children.push(subpat_node);
                let var = format!("{}__{}", var, ty_var);
                tree.quantified_vars.insert((var.clone(), ty_var));
                bound.push(var);
            }
            let body = create_parse_tree_expr(rule, body, tree, typeenv, termenv);
            let body_var = body.type_var;
            children.push(body);

            let type_var = tree.next_type_var;
            tree.next_type_var += 1;

            let name = format!("let__{}", type_var);

            // The let should have the same type as the body
            tree.var_constraints
                .insert(TypeExpr::Variable(type_var, body_var));

            TypeVarNode {
                ident: name.to_string(),
                construct: TypeVarConstruct::Let(bound),
                type_var,
                children,
                assertions: vec![],
            }
        }
    }
}

// TODO mod tests?
#[test]
fn test_solve_constraints() {
    // simple with specific and generic bvs
    let concrete = HashSet::from([
        TypeExpr::Concrete(2, annotation_ir::Type::BitVectorWithWidth(16)),
        TypeExpr::Concrete(3, annotation_ir::Type::BitVectorWithWidth(8)),
    ]);
    let var = HashSet::from([
        TypeExpr::Variable(5, 4),
        TypeExpr::Variable(6, 1),
        TypeExpr::Variable(3, 4),
        TypeExpr::Variable(1, 2),
    ]);
    let bv = HashSet::from([
        TypeExpr::Concrete(1, annotation_ir::Type::BitVector),
        TypeExpr::Concrete(4, annotation_ir::Type::BitVector),
    ]);
    let expected = HashMap::from([
        (1, annotation_ir::Type::BitVectorWithWidth(16)),
        (2, annotation_ir::Type::BitVectorWithWidth(16)),
        (3, annotation_ir::Type::BitVectorWithWidth(8)),
        (4, annotation_ir::Type::BitVectorWithWidth(8)),
        (5, annotation_ir::Type::BitVectorWithWidth(8)),
        (6, annotation_ir::Type::BitVectorWithWidth(16)),
    ]);
    let (sol, bvsets) = solve_constraints(concrete, var, bv);
    assert_eq!(expected, sol);
    assert!(bvsets.is_empty());

    // slightly more complicated with specific and generic bvs
    let concrete = HashSet::from([
        TypeExpr::Concrete(2, annotation_ir::Type::BitVectorWithWidth(16)),
        TypeExpr::Concrete(3, annotation_ir::Type::BitVectorWithWidth(8)),
    ]);
    let var = HashSet::from([
        TypeExpr::Variable(5, 4),
        TypeExpr::Variable(6, 1),
        TypeExpr::Variable(3, 4),
        TypeExpr::Variable(1, 2),
        TypeExpr::Variable(7, 8),
    ]);
    let bv = HashSet::from([
        TypeExpr::Concrete(1, annotation_ir::Type::BitVector),
        TypeExpr::Concrete(4, annotation_ir::Type::BitVector),
        TypeExpr::Concrete(7, annotation_ir::Type::BitVector),
    ]);
    let expected = HashMap::from([
        (1, annotation_ir::Type::BitVectorWithWidth(16)),
        (2, annotation_ir::Type::BitVectorWithWidth(16)),
        (3, annotation_ir::Type::BitVectorWithWidth(8)),
        (4, annotation_ir::Type::BitVectorWithWidth(8)),
        (5, annotation_ir::Type::BitVectorWithWidth(8)),
        (6, annotation_ir::Type::BitVectorWithWidth(16)),
        (7, annotation_ir::Type::BitVectorUnknown(7)),
        (8, annotation_ir::Type::BitVectorUnknown(7)),
    ]);
    let expected_bvsets = HashMap::from([(7, 0), (8, 0)]);
    let (sol, bvsets) = solve_constraints(concrete, var, bv);
    assert_eq!(expected, sol);
    assert_eq!(expected_bvsets, bvsets);
}

#[test]
#[should_panic]
fn test_solve_constraints_ill_typed() {
    // ill-typed
    let concrete = HashSet::from([
        TypeExpr::Concrete(2, annotation_ir::Type::BitVectorWithWidth(16)),
        TypeExpr::Concrete(3, annotation_ir::Type::BitVectorWithWidth(8)),
    ]);
    let var = HashSet::from([
        TypeExpr::Variable(5, 4),
        TypeExpr::Variable(6, 1),
        TypeExpr::Variable(4, 6),
        TypeExpr::Variable(3, 4),
        TypeExpr::Variable(1, 2),
    ]);
    let bv = HashSet::from([
        TypeExpr::Concrete(1, annotation_ir::Type::BitVector),
        TypeExpr::Concrete(4, annotation_ir::Type::BitVector),
    ]);
    solve_constraints(concrete, var, bv);
}
