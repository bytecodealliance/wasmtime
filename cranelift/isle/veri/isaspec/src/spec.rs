use std::collections::{HashMap, HashSet};

use anyhow::{Result, bail};

use cranelift_isle::ast::{Arm, FieldInit, Ident, ModelType, SpecExpr, SpecOp};
use cranelift_isle::lexer::Pos;

pub fn spec_const_int(val: i128) -> SpecExpr {
    SpecExpr::ConstInt {
        val,
        pos: Pos::default(),
    }
}

pub fn spec_const_bool(val: bool) -> SpecExpr {
    SpecExpr::ConstBool {
        val,
        pos: Pos::default(),
    }
}

pub fn spec_true() -> SpecExpr {
    spec_const_bool(true)
}

pub fn spec_false() -> SpecExpr {
    spec_const_bool(false)
}

pub fn spec_const_bit_vector(val: u128, width: usize) -> SpecExpr {
    assert!(width > 0);
    SpecExpr::ConstBitVec {
        val,
        width,
        pos: Pos::default(),
    }
}

pub fn spec_unary(op: SpecOp, x: SpecExpr) -> SpecExpr {
    spec_op(op, vec![x])
}

pub fn spec_binary(op: SpecOp, x: SpecExpr, y: SpecExpr) -> SpecExpr {
    spec_op(op, vec![x, y])
}

pub fn spec_ternary(op: SpecOp, x: SpecExpr, y: SpecExpr, z: SpecExpr) -> SpecExpr {
    spec_op(op, vec![x, y, z])
}

pub fn spec_not(x: SpecExpr) -> SpecExpr {
    spec_unary(SpecOp::Not, x)
}

pub fn spec_if(c: SpecExpr, t: SpecExpr, e: SpecExpr) -> SpecExpr {
    spec_ternary(SpecOp::If, c, t, e)
}

pub fn spec_eq(x: SpecExpr, y: SpecExpr) -> SpecExpr {
    spec_binary(SpecOp::Eq, x, y)
}

pub fn spec_eq_bool(x: SpecExpr, val: bool) -> SpecExpr {
    if val { x } else { spec_not(x) }
}

pub fn spec_or(args: Vec<SpecExpr>) -> SpecExpr {
    spec_op(SpecOp::Or, args)
}

pub fn spec_any(xs: Vec<SpecExpr>) -> SpecExpr {
    match xs.len() {
        0 => spec_false(),
        1 => xs[0].clone(),
        _ => spec_or(xs),
    }
}

pub fn spec_and(args: Vec<SpecExpr>) -> SpecExpr {
    spec_op(SpecOp::And, args)
}

pub fn spec_all(xs: Vec<SpecExpr>) -> SpecExpr {
    match xs.len() {
        0 => spec_true(),
        1 => xs[0].clone(),
        _ => spec_and(xs),
    }
}

pub fn spec_extract(h: usize, l: usize, x: SpecExpr) -> SpecExpr {
    spec_ternary(
        SpecOp::Extract,
        spec_const_int(h.try_into().unwrap()),
        spec_const_int(l.try_into().unwrap()),
        x,
    )
}

pub fn spec_conv_to(w: usize, x: SpecExpr) -> SpecExpr {
    spec_binary(SpecOp::ConvTo, spec_const_int(w.try_into().unwrap()), x)
}

pub fn spec_bv2nat(x: SpecExpr) -> SpecExpr {
    spec_unary(SpecOp::BV2Nat, x)
}

pub fn spec_zero_ext(w: usize, x: SpecExpr) -> SpecExpr {
    spec_ext(SpecOp::ZeroExt, w, x)
}

pub fn spec_sign_ext(w: usize, x: SpecExpr) -> SpecExpr {
    spec_ext(SpecOp::SignExt, w, x)
}

fn spec_ext(ext_op: SpecOp, w: usize, x: SpecExpr) -> SpecExpr {
    // Simplify nested extensions.
    if let SpecExpr::Op {
        ref op,
        ref args,
        pos: _,
    } = x
    {
        if *op == ext_op {
            if let [SpecExpr::ConstInt { val, .. }, n] = args.as_slice() {
                let nw: usize = (*val).try_into().unwrap();
                if w >= nw {
                    return spec_zero_ext(w, n.clone());
                }
            }
        }
    }

    // Base case just constructs zero extension operator.
    spec_binary(ext_op, spec_const_int(w.try_into().unwrap()), x)
}

pub fn spec_op(op: SpecOp, args: Vec<SpecExpr>) -> SpecExpr {
    SpecExpr::Op {
        op,
        args,
        pos: Pos::default(),
    }
}

pub fn spec_enum(name: String, variant: String, args: Vec<SpecExpr>) -> SpecExpr {
    SpecExpr::Enum {
        name: spec_ident(name),
        variant: spec_ident(variant),
        args,
        pos: Pos::default(),
    }
}

pub fn spec_enum_unit(name: String, variant: String) -> SpecExpr {
    spec_enum(name, variant, Vec::new())
}

pub fn spec_as_bit_vector_width(x: SpecExpr, width: usize) -> SpecExpr {
    SpecExpr::As {
        x: Box::new(x),
        ty: ModelType::BitVec(Some(width)),
        pos: Pos::default(),
    }
}

pub fn spec_field(field: String, x: SpecExpr) -> SpecExpr {
    SpecExpr::Field {
        field: spec_ident(field),
        x: Box::new(x),
        pos: Pos::default(),
    }
}

pub fn spec_discriminator(variant: String, x: SpecExpr) -> SpecExpr {
    SpecExpr::Discriminator {
        variant: spec_ident(variant),
        x: Box::new(x),
        pos: Pos::default(),
    }
}

pub fn spec_var(id: String) -> SpecExpr {
    SpecExpr::Var {
        var: spec_ident(id),
        pos: Pos::default(),
    }
}

pub fn spec_with(decls: Vec<Ident>, body: SpecExpr) -> SpecExpr {
    SpecExpr::With {
        decls,
        body: Box::new(body),
        pos: Pos::default(),
    }
}

pub fn spec_idents(ids: &[String]) -> Vec<Ident> {
    ids.iter().cloned().map(spec_ident).collect()
}

pub fn spec_ident(id: String) -> Ident {
    Ident(id, Pos::default())
}

#[derive(Clone)]
pub struct Conditions {
    pub requires: Vec<SpecExpr>,
    pub provides: Vec<SpecExpr>,
    pub modifies: HashSet<String>,
}

impl Conditions {
    pub fn new() -> Self {
        Self {
            requires: Vec::new(),
            provides: Vec::new(),
            modifies: HashSet::new(),
        }
    }

    pub fn merge(cs: Vec<Self>) -> Self {
        match cs.len() {
            0 => Self::new(),
            1 => cs[0].clone(),
            _ => Self {
                requires: vec![spec_or(
                    cs.iter().map(|c| spec_all(c.requires.clone())).collect(),
                )],
                provides: cs
                    .iter()
                    .map(|c| {
                        spec_binary(
                            SpecOp::Imp,
                            spec_all(c.requires.clone()),
                            spec_all(c.provides.clone()),
                        )
                    })
                    .collect(),
                modifies: cs.iter().fold(HashSet::new(), |acc, c| &acc | &c.modifies),
            },
        }
    }
}

pub fn substitute(expr: SpecExpr, substitutions: &HashMap<String, SpecExpr>) -> Result<SpecExpr> {
    Ok(match expr {
        // Variable
        SpecExpr::Var { ref var, pos: _ } => {
            if let Some(substitution) = substitutions.get(&var.0) {
                substitution.clone()
            } else {
                expr
            }
        }

        // Constants are unchanged.
        SpecExpr::ConstInt { .. } | SpecExpr::ConstBitVec { .. } | SpecExpr::ConstBool { .. } => {
            expr
        }

        // Inline macro introduces a new scope.
        SpecExpr::Macro { .. } => expr,

        // Scopes require care to ensure we are not replacing introduced variables.
        SpecExpr::Match { x, arms, pos } => SpecExpr::Match {
            x: Box::new(substitute(*x, substitutions)?),
            arms: arms
                .into_iter()
                .map(
                    |Arm {
                         variant,
                         args,
                         body,
                         pos,
                     }| {
                        for arg in &args {
                            if substitutions.contains_key(&arg.0) {
                                bail!("substituted variable collides with match arm");
                            }
                        }
                        Ok(Arm {
                            variant,
                            args,
                            body: substitute(body, substitutions)?,
                            pos,
                        })
                    },
                )
                .collect::<Result<_>>()?,
            pos,
        },
        SpecExpr::Let { defs, body, pos } => SpecExpr::Let {
            defs: defs
                .into_iter()
                .map(|(var, expr)| {
                    if substitutions.contains_key(&var.0) {
                        bail!("substituted variable collides with let binding");
                    }
                    Ok((var, substitute(expr, substitutions)?))
                })
                .collect::<Result<_>>()?,
            body: Box::new(substitute(*body, substitutions)?),
            pos,
        },
        SpecExpr::With { decls, body, pos } => {
            for decl in &decls {
                if substitutions.contains_key(&decl.0) {
                    bail!("substituted variable collides with with scope");
                }
            }
            SpecExpr::With {
                decls,
                body: Box::new(substitute(*body, substitutions)?),
                pos,
            }
        }

        // Recurse into child expressions.
        SpecExpr::As { x, ty, pos } => SpecExpr::As {
            x: Box::new(substitute(*x, substitutions)?),
            ty,
            pos,
        },
        SpecExpr::Field { field, x, pos } => SpecExpr::Field {
            field,
            x: Box::new(substitute(*x, substitutions)?),
            pos,
        },
        SpecExpr::Discriminator { variant, x, pos } => SpecExpr::Discriminator {
            variant,
            x: Box::new(substitute(*x, substitutions)?),
            pos,
        },
        SpecExpr::Op { op, args, pos } => SpecExpr::Op {
            op,
            args: args
                .into_iter()
                .map(|arg| substitute(arg, substitutions))
                .collect::<Result<_>>()?,
            pos,
        },
        SpecExpr::Pair { l, r, pos } => SpecExpr::Pair {
            l: Box::new(substitute(*l, substitutions)?),
            r: Box::new(substitute(*r, substitutions)?),
            pos,
        },
        SpecExpr::Enum {
            name,
            variant,
            args,
            pos,
        } => SpecExpr::Enum {
            name,
            variant,
            args: args
                .into_iter()
                .map(|arg| substitute(arg, substitutions))
                .collect::<Result<_>>()?,
            pos,
        },
        SpecExpr::Expand { name, args, pos } => SpecExpr::Expand {
            name,
            args: args
                .into_iter()
                .map(|arg| substitute(arg, substitutions))
                .collect::<Result<_>>()?,
            pos,
        },
        SpecExpr::Struct { fields, pos } => SpecExpr::Struct {
            fields: fields
                .into_iter()
                .map(|f| {
                    Ok(FieldInit {
                        name: f.name,
                        value: Box::new(substitute(*f.value, substitutions)?),
                        pos: f.pos,
                    })
                })
                .collect::<Result<_>>()?,
            pos,
        },
    })
}
