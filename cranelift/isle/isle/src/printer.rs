//! Printer for ISLE language.

use std::{io::Write, vec};

use crate::ast::*;

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
    pub fn atom<S: ToString>(atom: S) -> Self {
        SExpr::Atom(atom.to_string())
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
            Def::Rule(rule) => rule.to_sexpr(),
            Def::Extractor(extractor) => extractor.to_sexpr(),
            Def::Decl(decl) => decl.to_sexpr(),
            Def::Spec(spec) => spec.to_sexpr(),
            Def::Model(model) => model.to_sexpr(),
            Def::Form(form) => form.to_sexpr(),
            Def::Instantiation(instantiation) => instantiation.to_sexpr(),
            Def::Extern(ext) => ext.to_sexpr(),
            Def::Converter(converter) => converter.to_sexpr(),
        }
    }
}

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

impl ToSExpr for Rule {
    fn to_sexpr(&self) -> SExpr {
        let mut parts = vec![SExpr::atom("rule")];
        if let Some(name) = &self.name {
            parts.push(name.to_sexpr());
        }
        if let Some(prio) = &self.prio {
            parts.push(SExpr::atom(prio.to_string()));
        }
        parts.push(self.pattern.to_sexpr());
        parts.extend(self.iflets.iter().map(ToSExpr::to_sexpr));
        parts.push(self.expr.to_sexpr());
        SExpr::List(parts)
    }
}

impl ToSExpr for Extractor {
    fn to_sexpr(&self) -> SExpr {
        todo!("extractor")
    }
}

//             Def::Extractor(ref e) => sexp(vec![
//                 RcDoc::text("extractor"),
//                 sexp(
//                     Vec::from([e.term.to_doc()])
//                         .into_iter()
//                         .chain(e.args.iter().map(|v| v.to_doc())),
//                 ),
//                 e.template.to_doc(),
//             ]),

impl ToSExpr for Decl {
    fn to_sexpr(&self) -> SExpr {
        let mut parts = vec![SExpr::atom("decl")];
        if self.pure {
            parts.push(SExpr::atom("pure"));
        }
        if self.multi {
            parts.push(SExpr::atom("multi"));
        }
        if self.partial {
            parts.push(SExpr::atom("partial"));
        }
        parts.push(self.term.to_sexpr());
        parts.push(SExpr::list(&self.arg_tys));
        parts.push(self.ret_ty.to_sexpr());
        SExpr::List(parts)
    }
}

impl ToSExpr for Spec {
    fn to_sexpr(&self) -> SExpr {
        todo!("spec")
    }
}

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

impl ToSExpr for Model {
    fn to_sexpr(&self) -> SExpr {
        todo!("model")
    }
}

//             Def::Model(ref m) => sexp(vec![RcDoc::text("model"), m.name.to_doc(), m.val.to_doc()]),

impl ToSExpr for Form {
    fn to_sexpr(&self) -> SExpr {
        todo!("form")
    }
}

//             Def::Form(ref f) => {
//                 let mut parts = vec![RcDoc::text("form")];
//                 parts.push(f.name.to_doc());
//                 parts.extend(f.signatures.iter().map(|s| s.to_doc()));
//                 sexp(parts)
//             }

impl ToSExpr for Instantiation {
    fn to_sexpr(&self) -> SExpr {
        todo!("instantiation")
    }
}

//             Def::Instantiation(ref i) => {
//                 let mut parts = vec![RcDoc::text("instantiate"), i.term.to_doc()];
//                 if let Some(form) = &i.form {
//                     parts.push(form.to_doc());
//                 } else {
//                     parts.extend(i.signatures.iter().map(|s| s.to_doc()));
//                 }
//                 sexp(parts)
//             }

impl ToSExpr for Extern {
    fn to_sexpr(&self) -> SExpr {
        todo!("extern")
    }
}

impl ToSExpr for Converter {
    fn to_sexpr(&self) -> SExpr {
        todo!("converter")
    }
}

//             Def::Converter(ref c) => sexp(vec![
//                 RcDoc::text("convert"),
//                 c.inner_ty.to_doc(),
//                 c.outer_ty.to_doc(),
//                 c.term.to_doc(),
//             ]),

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

impl ToSExpr for Pattern {
    fn to_sexpr(&self) -> SExpr {
        match self {
            Pattern::Var { var, .. } => SExpr::atom(var.0.clone()),
            Pattern::BindPattern { var, subpat, .. } => {
                SExpr::List(vec![var.to_sexpr(), SExpr::atom("@"), subpat.to_sexpr()])
            }
            Pattern::ConstInt { val, .. } => SExpr::atom(val),
            Pattern::ConstBool { val, .. } => SExpr::atom(if *val { "true" } else { "false" }),
            Pattern::ConstPrim { val, .. } => SExpr::atom(format!("${}", val.0)),
            Pattern::Wildcard { .. } => SExpr::atom("_"),
            Pattern::Term { sym, args, .. } => {
                let mut parts = vec![sym.to_sexpr()];
                parts.extend(args.iter().map(ToSExpr::to_sexpr));
                SExpr::List(parts)
            }
            Pattern::And { subpats, .. } => {
                let mut parts = vec![SExpr::atom("and")];
                parts.extend(subpats.iter().map(ToSExpr::to_sexpr));
                SExpr::List(parts)
            }
            Pattern::MacroArg { .. } => unimplemented!("macro arguments are for internal use only"),
        }
    }
}

impl ToSExpr for IfLet {
    fn to_sexpr(&self) -> SExpr {
        SExpr::List(vec![
            SExpr::atom("if-let"),
            self.pattern.to_sexpr(),
            self.expr.to_sexpr(),
        ])
    }
}

impl ToSExpr for Expr {
    fn to_sexpr(&self) -> SExpr {
        match self {
            Expr::Term { sym, args, .. } => {
                let mut parts = vec![sym.to_sexpr()];
                parts.extend(args.iter().map(ToSExpr::to_sexpr));
                SExpr::List(parts)
            }
            Expr::Var { name, .. } => name.to_sexpr(),
            Expr::ConstInt { val, .. } => SExpr::atom(val),
            Expr::ConstBool { val, .. } => SExpr::atom(if *val { "true" } else { "false" }),
            Expr::ConstPrim { val, .. } => SExpr::atom(format!("${}", val.0)),
            Expr::Let { defs, body, .. } => {
                let mut parts = vec![SExpr::atom("let")];
                parts.push(SExpr::list(&defs));
                parts.push(body.to_sexpr());
                SExpr::List(parts)
            }
        }
    }
}

impl ToSExpr for LetDef {
    fn to_sexpr(&self) -> SExpr {
        SExpr::List(vec![
            self.var.to_sexpr(),
            self.ty.to_sexpr(),
            self.val.to_sexpr(),
        ])
    }
}

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
