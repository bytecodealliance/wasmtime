//! Printer for ISLE language.

use std::{io::Write, vec};

use crate::ast::{Def, Field, Ident, Type, TypeValue, Variant};

/// Print ISLE definitions.
pub fn print<W: Write>(defs: &[Def], width: usize, out: &mut W) -> std::io::Result<()> {
    for def in defs {
        print_node(def, out)?;
        writeln!(out)?;
    }
    Ok(())
}

pub fn print_node<N: ToSExpr, W: Write>(node: &N, out: &mut W) -> std::io::Result<()> {
    node.to_sexpr().print(out)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SExpr {
    Atom(String),
    List(Vec<SExpr>),
}

pub trait ToSExpr {
    fn to_sexpr(&self) -> SExpr;
}

impl SExpr {
    pub fn atom<S: Into<String>>(atom: S) -> Self {
        SExpr::Atom(atom.into())
    }

    pub fn list(items: &[impl ToSExpr]) -> Self {
        SExpr::List(items.into_iter().map(|i| i.to_sexpr()).collect())
    }

    pub fn print<W: Write>(&self, out: &mut W) -> std::io::Result<()> {
        match self {
            SExpr::Atom(s) => write!(out, "{}", s),
            SExpr::List(items) => {
                write!(out, "(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(out, " ")?;
                    }
                    item.print(out)?;
                }
                write!(out, ")")
            }
        }
    }
}

impl ToSExpr for Def {
    fn to_sexpr(&self) -> SExpr {
        match self {
            Def::Pragma(_) => unimplemented!("pragmas not supported"),
            Def::Type(ty) => ty.to_sexpr(),
            Def::Rule(_) => todo!(),
            Def::Extractor(_) => todo!(),
            Def::Decl(_) => todo!(),
            Def::Spec(_) => todo!(),
            Def::Model(_) => todo!(),
            Def::Form(_) => todo!(),
            Def::Instantiation(_) => todo!(),
            Def::Extern(_) => todo!(),
            Def::Converter(_) => todo!(),
        }
    }
}

// impl Printable for Def {
//     fn to_doc(&self) -> RcDoc<()> {
//         match self {
//             Def::Rule(ref r) => {
//                 let mut parts = Vec::new();
//                 parts.push(RcDoc::text("rule"));
//                 if let Some(name) = &r.name {
//                     parts.push(name.to_doc());
//                 }
//                 if let Some(prio) = &r.prio {
//                     parts.push(RcDoc::as_string(prio));
//                 }
//                 parts.push(r.pattern.to_doc());
//                 parts.extend(r.iflets.iter().map(|il| il.to_doc()));
//                 parts.push(r.expr.to_doc());
//                 sexp(parts)
//             }
//             Def::Extractor(ref e) => sexp(vec![
//                 RcDoc::text("extractor"),
//                 sexp(
//                     Vec::from([e.term.to_doc()])
//                         .into_iter()
//                         .chain(e.args.iter().map(|v| v.to_doc())),
//                 ),
//                 e.template.to_doc(),
//             ]),
//             Def::Decl(ref d) => {
//                 let mut parts = Vec::new();
//                 parts.push(RcDoc::text("decl"));
//                 if d.pure {
//                     parts.push(RcDoc::text("pure"));
//                 }
//                 if d.multi {
//                     parts.push(RcDoc::text("multi"));
//                 }
//                 if d.partial {
//                     parts.push(RcDoc::text("partial"));
//                 }
//                 parts.push(d.term.to_doc());
//                 parts.push(sexp(d.arg_tys.iter().map(|ty| ty.to_doc())));
//                 parts.push(d.ret_ty.to_doc());
//                 sexp(parts)
//             }
//             Def::Attr(ref a) => a.to_doc(),
//             Def::Spec(ref s) => s.to_doc(),
//             Def::SpecMacro(ref m) => m.to_doc(),
//             Def::State(ref s) => s.to_doc(),
//             Def::Model(ref m) => sexp(vec![RcDoc::text("model"), m.name.to_doc(), m.val.to_doc()]),
//             Def::Form(ref f) => {
//                 let mut parts = vec![RcDoc::text("form")];
//                 parts.push(f.name.to_doc());
//                 parts.extend(f.signatures.iter().map(|s| s.to_doc()));
//                 sexp(parts)
//             }
//             Def::Instantiation(ref i) => {
//                 let mut parts = vec![RcDoc::text("instantiate"), i.term.to_doc()];
//                 if let Some(form) = &i.form {
//                     parts.push(form.to_doc());
//                 } else {
//                     parts.extend(i.signatures.iter().map(|s| s.to_doc()));
//                 }
//                 sexp(parts)
//             }
//             Def::Extern(ref e) => e.to_doc(),
//             Def::Converter(ref c) => sexp(vec![
//                 RcDoc::text("convert"),
//                 c.inner_ty.to_doc(),
//                 c.outer_ty.to_doc(),
//                 c.term.to_doc(),
//             ]),
//         }
//     }
// }

impl ToSExpr for Type {
    fn to_sexpr(&self) -> SExpr {
        let mut parts = vec![SExpr::atom("type"), self.name.to_sexpr()];
        if self.is_extern {
            parts.push(SExpr::atom("extern"));
        }
        if self.is_nodebug {
            parts.push(SExpr::Atom("nodebug".to_string()));
        }
        parts.push(self.ty.to_sexpr());
        SExpr::List(parts)
    }
}

impl ToSExpr for TypeValue {
    fn to_sexpr(&self) -> SExpr {
        match self {
            TypeValue::Primitive(name, _) => {
                SExpr::List(vec![SExpr::atom("primitive"), name.to_sexpr()])
            }
            TypeValue::Enum(variants, _) => {
                let mut parts = vec![SExpr::atom("enum")];
                for variant in variants {
                    parts.push(variant.to_sexpr());
                }
                SExpr::List(parts)
            }
        }
    }
}

impl ToSExpr for Variant {
    fn to_sexpr(&self) -> SExpr {
        SExpr::List(vec![
            SExpr::atom("variant"),
            self.name.to_sexpr(),
            SExpr::list(&self.fields),
        ])
    }
}

impl ToSExpr for Field {
    fn to_sexpr(&self) -> SExpr {
        SExpr::List(vec![self.name.to_sexpr(), self.ty.to_sexpr()])
    }
}

// impl Printable for Attr {
//     fn to_doc(&self) -> RcDoc<()> {
//         let mut parts = vec![RcDoc::text("attr")];
//         match &self.target {
//             AttrTarget::Term(name) => parts.push(name.to_doc()),
//             AttrTarget::Rule(name) => {
//                 parts.push(RcDoc::text("rule"));
//                 parts.push(name.to_doc());
//             }
//         }
//         parts.extend(self.kinds.iter().map(Printable::to_doc));
//         sexp(parts)
//     }
// }
//
// impl Printable for AttrKind {
//     fn to_doc(&self) -> RcDoc<()> {
//         match self {
//             AttrKind::Chain => sexp(vec![RcDoc::text("veri"), RcDoc::text("chain")]),
//             AttrKind::Priority => sexp(vec![RcDoc::text("veri"), RcDoc::text("priority")]),
//             AttrKind::Tag(tag) => sexp(vec![RcDoc::text("tag"), tag.to_doc()]),
//         }
//     }
// }
//
// impl Printable for ModelValue {
//     fn to_doc(&self) -> RcDoc<()> {
//         match self {
//             ModelValue::TypeValue(ref mt) => sexp(vec![RcDoc::text("type"), mt.to_doc()]),
//             ModelValue::ConstValue(ref c) => sexp(vec![RcDoc::text("const"), c.to_doc()]),
//         }
//     }
// }
//
// impl Printable for ModelType {
//     fn to_doc(&self) -> RcDoc<()> {
//         match self {
//             ModelType::Unspecified => RcDoc::text("!"),
//             ModelType::Auto => RcDoc::text("_"),
//             ModelType::Unit => RcDoc::text("Unit"),
//             ModelType::Int => RcDoc::text("Int"),
//             ModelType::Bool => RcDoc::text("Bool"),
//             ModelType::BitVec(Some(size)) => sexp(vec![RcDoc::text("bv"), RcDoc::as_string(size)]),
//             ModelType::BitVec(None) => sexp(vec![RcDoc::text("bv")]),
//             ModelType::Struct(fields) => sexp(
//                 vec![RcDoc::text("struct")]
//                     .into_iter()
//                     .chain(fields.iter().map(|f| f.to_doc())),
//             ),
//             ModelType::Named(name) => sexp(vec![RcDoc::text("named"), name.to_doc()]),
//         }
//     }
// }
//
// impl Printable for ModelField {
//     fn to_doc(&self) -> RcDoc<()> {
//         sexp(vec![self.name.to_doc(), self.ty.to_doc()])
//     }
// }
//
// impl Printable for Signature {
//     fn to_doc(&self) -> RcDoc<()> {
//         sexp(vec![
//             sexp(
//                 Vec::from([RcDoc::text("args")])
//                     .into_iter()
//                     .chain(self.args.iter().map(|a| a.to_doc())),
//             ),
//             sexp(vec![RcDoc::text("ret"), self.ret.to_doc()]),
//         ])
//     }
// }
//
// impl Printable for SpecExpr {
//     fn to_doc(&self) -> RcDoc<()> {
//         match self {
//             SpecExpr::ConstInt { val, .. } => RcDoc::as_string(val),
//             SpecExpr::ConstBitVec { val, width, .. } => RcDoc::text(if width % 4 == 0 {
//                 format!("#x{val:0width$x}", width = *width / 4)
//             } else {
//                 format!("#b{val:0width$b}", width = *width)
//             }),
//             SpecExpr::ConstBool { val, .. } => RcDoc::text(if *val { "true" } else { "false" }),
//             SpecExpr::Var { var, .. } => var.to_doc(),
//             SpecExpr::As { x, ty, pos: _ } => {
//                 sexp(vec![RcDoc::text("as"), x.to_doc(), ty.to_doc()])
//             }
//             SpecExpr::Op { op, args, .. } => sexp(
//                 Vec::from([op.to_doc()])
//                     .into_iter()
//                     .chain(args.iter().map(|a| a.to_doc())),
//             ),
//             SpecExpr::Pair { l, r, .. } => sexp(vec![l.to_doc(), r.to_doc()]),
//             SpecExpr::Enum {
//                 name,
//                 variant,
//                 args,
//                 pos: _,
//             } => sexp(
//                 Vec::from([RcDoc::text(format!("{}.{}", name.0, variant.0))])
//                     .into_iter()
//                     .chain(args.iter().map(|a| a.to_doc())),
//             ),
//             SpecExpr::Field { field, x, pos: _ } => {
//                 sexp(vec![RcDoc::text(format!(":{}", field.0)), x.to_doc()])
//             }
//             SpecExpr::Discriminator { variant, x, pos: _ } => {
//                 sexp(vec![RcDoc::text(format!("{}?", variant.0)), x.to_doc()])
//             }
//             SpecExpr::Match { x, arms, pos: _ } => sexp(
//                 Vec::from([RcDoc::text("match"), x.to_doc()])
//                     .into_iter()
//                     .chain(arms.iter().map(|arm| arm.to_doc())),
//             ),
//             SpecExpr::Let { defs, body, pos: _ } => sexp(vec![
//                 RcDoc::text("let"),
//                 sexp(defs.iter().map(|(n, e)| sexp(vec![n.to_doc(), e.to_doc()]))),
//                 body.to_doc(),
//             ]),
//             SpecExpr::With {
//                 decls,
//                 body,
//                 pos: _,
//             } => sexp(vec![
//                 RcDoc::text("with"),
//                 sexp(decls.iter().map(Printable::to_doc)),
//                 body.to_doc(),
//             ]),
//             SpecExpr::Macro {
//                 params,
//                 body,
//                 pos: _,
//             } => sexp(vec![
//                 RcDoc::text("macro"),
//                 sexp(params.iter().map(Printable::to_doc)),
//                 body.to_doc(),
//             ]),
//             SpecExpr::Expand { name, args, pos: _ } => sexp(
//                 Vec::from([RcDoc::text(format!("{}!", name.0))])
//                     .into_iter()
//                     .chain(args.iter().map(Printable::to_doc)),
//             ),
//             SpecExpr::Struct { fields, pos: _ } => sexp(
//                 Vec::from([RcDoc::text("struct")])
//                     .into_iter()
//                     .chain(fields.iter().map(Printable::to_doc)),
//             ),
//         }
//     }
// }
//
// impl Printable for SpecOp {
//     fn to_doc(&self) -> RcDoc<()> {
//         RcDoc::text(match self {
//             SpecOp::Eq => "=",
//             SpecOp::And => "and",
//             SpecOp::Not => "not",
//             SpecOp::Imp => "=>",
//             SpecOp::Or => "or",
//             SpecOp::Add => "+",
//             SpecOp::Sub => "-",
//             SpecOp::Mul => "*",
//             SpecOp::Lte => "<=",
//             SpecOp::Lt => "<",
//             SpecOp::Gte => ">=",
//             SpecOp::Gt => ">",
//             SpecOp::BVNot => "bvnot",
//             SpecOp::BVAnd => "bvand",
//             SpecOp::BVOr => "bvor",
//             SpecOp::BVXor => "bvxor",
//             SpecOp::BVNeg => "bvneg",
//             SpecOp::BVAdd => "bvadd",
//             SpecOp::BVSub => "bvsub",
//             SpecOp::BVMul => "bvmul",
//             SpecOp::BVUdiv => "bvudiv",
//             SpecOp::BVUrem => "bvurem",
//             SpecOp::BVSdiv => "bvsdiv",
//             SpecOp::BVSrem => "bvsrem",
//             SpecOp::BVShl => "bvshl",
//             SpecOp::BVLshr => "bvlshr",
//             SpecOp::BVAshr => "bvashr",
//             SpecOp::BVSaddo => "bvsaddo",
//             SpecOp::BVUle => "bvule",
//             SpecOp::BVUlt => "bvult",
//             SpecOp::BVUgt => "bvugt",
//             SpecOp::BVUge => "bvuge",
//             SpecOp::BVSlt => "bvslt",
//             SpecOp::BVSle => "bvsle",
//             SpecOp::BVSgt => "bvsgt",
//             SpecOp::BVSge => "bvsge",
//             SpecOp::Rotr => "rotr",
//             SpecOp::Rotl => "rotl",
//             SpecOp::Extract => "extract",
//             SpecOp::ZeroExt => "zero_ext",
//             SpecOp::SignExt => "sign_ext",
//             SpecOp::Concat => "concat",
//             SpecOp::Replicate => "replicate",
//             SpecOp::ConvTo => "conv_to",
//             SpecOp::Int2BV => "int2bv",
//             SpecOp::BV2Nat => "bv2nat",
//             SpecOp::ToFP => "to_fp",
//             SpecOp::FPToUBV => "fp.to_ubv",
//             SpecOp::FPToSBV => "fp.to_sbv",
//             SpecOp::ToFPUnsigned => "to_fp_unsigned",
//             SpecOp::ToFPFromFP => "to_fp_from_fp",
//             SpecOp::WidthOf => "widthof",
//             SpecOp::If => "if",
//             SpecOp::Switch => "switch",
//             SpecOp::Popcnt => "popcnt",
//             SpecOp::Rev => "rev",
//             SpecOp::Cls => "cls",
//             SpecOp::Clz => "clz",
//             SpecOp::FPPositiveInfinity => "fp.+oo",
//             SpecOp::FPNegativeInfinity => "fp.-oo",
//             SpecOp::FPPositiveZero => "fp.+zero",
//             SpecOp::FPNegativeZero => "fp.-zero",
//             SpecOp::FPNaN => "fp.NaN",
//             SpecOp::FPEq => "fp.eq",
//             SpecOp::FPNe => "fp.ne",
//             SpecOp::FPLt => "fp.lt",
//             SpecOp::FPGt => "fp.gt",
//             SpecOp::FPLe => "fp.le",
//             SpecOp::FPGe => "fp.ge",
//             SpecOp::FPAdd => "fp.add",
//             SpecOp::FPSub => "fp.sub",
//             SpecOp::FPMul => "fp.mul",
//             SpecOp::FPDiv => "fp.div",
//             SpecOp::FPMin => "fp.min",
//             SpecOp::FPMax => "fp.max",
//             SpecOp::FPNeg => "fp.neg",
//             SpecOp::FPCeil => "fp.ceil",
//             SpecOp::FPFloor => "fp.floor",
//             SpecOp::FPSqrt => "fp.sqrt",
//             SpecOp::FPTrunc => "fp.trunc",
//             SpecOp::FPNearest => "fp.nearest",
//             SpecOp::FPIsZero => "fp.isZero",
//             SpecOp::FPIsInfinite => "fp.isInfinite",
//             SpecOp::FPIsNaN => "fp.isNaN",
//             SpecOp::FPIsNegative => "fp.isNegative",
//             SpecOp::FPIsPositive => "fp.isPositive",
//         })
//     }
// }
//
// impl Printable for Arm {
//     fn to_doc(&self) -> RcDoc<()> {
//         sexp(vec![
//             sexp(
//                 Vec::from([self.variant.to_doc()])
//                     .into_iter()
//                     .chain(self.args.iter().map(|a| a.to_doc())),
//             ),
//             self.body.to_doc(),
//         ])
//     }
// }
//
// impl Printable for FieldInit {
//     fn to_doc(&self) -> RcDoc<()> {
//         sexp(vec![self.name.to_doc(), self.value.to_doc()])
//     }
// }
//
// impl Printable for SpecMacro {
//     fn to_doc(&self) -> RcDoc<()> {
//         let mut parts = vec![RcDoc::text("macro")];
//         parts.push(sexp(
//             Vec::from([self.name.to_doc()])
//                 .into_iter()
//                 .chain(self.params.iter().map(|a| a.to_doc())),
//         ));
//         parts.push(self.body.to_doc());
//         sexp(parts)
//     }
// }
//
// impl Printable for Modifies {
//     fn to_doc(&self) -> RcDoc<()> {
//         let mut parts = vec![RcDoc::text("modifies"), self.state.to_doc()];
//         if let Some(cond) = &self.cond {
//             parts.push(cond.to_doc());
//         }
//         sexp(parts)
//     }
// }
//
// impl Printable for Spec {
//     fn to_doc(&self) -> RcDoc<()> {
//         let mut parts = vec![RcDoc::text("spec")];
//         parts.push(sexp(
//             Vec::from([self.term.to_doc()])
//                 .into_iter()
//                 .chain(self.args.iter().map(|a| a.to_doc())),
//         ));
//         for modifies in &self.modifies {
//             parts.push(modifies.to_doc());
//         }
//         if !self.provides.is_empty() {
//             parts.push(sexp(
//                 Vec::from([RcDoc::text("provide")])
//                     .into_iter()
//                     .chain(self.provides.iter().map(|e| e.to_doc())),
//             ));
//         }
//         if !self.requires.is_empty() {
//             parts.push(sexp(
//                 Vec::from([RcDoc::text("require")])
//                     .into_iter()
//                     .chain(self.requires.iter().map(|e| e.to_doc())),
//             ));
//         }
//         if !self.matches.is_empty() {
//             parts.push(sexp(
//                 Vec::from([RcDoc::text("match")])
//                     .into_iter()
//                     .chain(self.matches.iter().map(|e| e.to_doc())),
//             ));
//         }
//         sexp(parts)
//     }
// }
//
// impl Printable for State {
//     fn to_doc(&self) -> RcDoc<()> {
//         sexp(vec![
//             RcDoc::text("state"),
//             self.name.to_doc(),
//             sexp(vec![RcDoc::text("type"), self.ty.to_doc()]),
//             sexp(vec![RcDoc::text("default"), self.default.to_doc()]),
//         ])
//     }
// }
//
// impl Printable for Pattern {
//     fn to_doc(&self) -> RcDoc<()> {
//         match self {
//             Pattern::Var { var, .. } => var.to_doc(),
//             Pattern::BindPattern { var, subpat, .. } => RcDoc::intersperse(
//                 vec![var.to_doc(), RcDoc::text("@"), subpat.to_doc()],
//                 Doc::space(),
//             ),
//             Pattern::ConstInt { val, .. } => RcDoc::as_string(val),
//             Pattern::ConstPrim { val, .. } => RcDoc::text("$").append(val.to_doc()),
//             Pattern::Wildcard { .. } => RcDoc::text("_"),
//             Pattern::Term { sym, args, .. } => sexp(
//                 // TODO(mbm): convenience for sexp with a fixed first element
//                 Vec::from([sym.to_doc()])
//                     .into_iter()
//                     .chain(args.iter().map(|f| f.to_doc())),
//             ),
//             Pattern::And { subpats, .. } => sexp(
//                 Vec::from([RcDoc::text("and")])
//                     .into_iter()
//                     .chain(subpats.iter().map(|p| p.to_doc())),
//             ),
//             Pattern::MacroArg { .. } => unimplemented!("macro arguments are for internal use only"),
//         }
//     }
// }
//
// impl Printable for IfLet {
//     fn to_doc(&self) -> RcDoc<()> {
//         // TODO(mbm): `if` shorthand when pattern is wildcard
//         sexp(vec![
//             RcDoc::text("if-let"),
//             self.pattern.to_doc(),
//             self.expr.to_doc(),
//         ])
//     }
// }
//
// impl Printable for Expr {
//     fn to_doc(&self) -> RcDoc<()> {
//         match self {
//             Expr::Term { sym, args, .. } => sexp(
//                 // TODO(mbm): convenience for sexp with a fixed first element
//                 Vec::from([sym.to_doc()])
//                     .into_iter()
//                     .chain(args.iter().map(|f| f.to_doc())),
//             ),
//             Expr::Var { name, .. } => name.to_doc(),
//             Expr::ConstInt { val, .. } => RcDoc::as_string(val),
//             Expr::ConstPrim { val, .. } => RcDoc::text("$").append(val.to_doc()),
//             Expr::Let { defs, body, .. } => {
//                 let mut parts = Vec::new();
//                 parts.push(RcDoc::text("let"));
//                 parts.push(sexp(defs.iter().map(|d| d.to_doc())));
//                 parts.push(body.to_doc());
//                 sexp(parts)
//             }
//         }
//     }
// }
//
// impl Printable for LetDef {
//     fn to_doc(&self) -> RcDoc<()> {
//         sexp(vec![self.var.to_doc(), self.ty.to_doc(), self.val.to_doc()])
//     }
// }
//
// impl Printable for Extern {
//     fn to_doc(&self) -> RcDoc<()> {
//         match self {
//             Extern::Extractor {
//                 term,
//                 func,
//                 pos: _,
//                 infallible,
//             } => {
//                 let mut parts = vec![RcDoc::text("extern"), RcDoc::text("extractor")];
//                 if *infallible {
//                     parts.push(RcDoc::text("infallible"));
//                 }
//                 parts.push(term.to_doc());
//                 parts.push(func.to_doc());
//                 sexp(parts)
//             }
//             Extern::Constructor { term, func, .. } => sexp(vec![
//                 RcDoc::text("extern"),
//                 RcDoc::text("constructor"),
//                 term.to_doc(),
//                 func.to_doc(),
//             ]),
//             Extern::Const { name, ty, .. } => sexp(vec![
//                 RcDoc::text("extern"),
//                 RcDoc::text("const"),
//                 RcDoc::text("$").append(name.to_doc()),
//                 ty.to_doc(),
//             ]),
//         }
//     }
// }
//
// fn sexp<'a, I, A>(docs: I) -> RcDoc<'a, A>
// where
//     I: IntoIterator,
//     I::Item: Pretty<'a, RcAllocator, A>,
//     A: Clone,
// {
//     RcDoc::text("(")
//         .append(RcDoc::intersperse(docs, Doc::line()).nest(4).group())
//         .append(RcDoc::text(")"))
// }

impl ToSExpr for Ident {
    fn to_sexpr(&self) -> SExpr {
        SExpr::atom(self.0.clone())
    }
}
