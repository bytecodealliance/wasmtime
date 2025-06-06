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

    pub fn tagged(tag: &str, items: &[impl ToSExpr]) -> Self {
        let mut parts = vec![SExpr::atom(tag)];
        parts.extend(items.iter().map(ToSExpr::to_sexpr));
        SExpr::List(parts)
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
        let mut sig = vec![self.term.to_sexpr()];
        sig.extend(self.args.iter().map(ToSExpr::to_sexpr));

        let mut parts = vec![SExpr::atom("extractor")];
        parts.push(SExpr::List(sig));
        parts.push(self.template.to_sexpr());
        SExpr::List(parts)
    }
}

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
        let mut sig = vec![self.term.to_sexpr()];
        sig.extend(self.args.iter().map(ToSExpr::to_sexpr));

        let mut parts = vec![SExpr::atom("spec")];
        parts.push(SExpr::List(sig));
        if !self.provides.is_empty() {
            parts.push(SExpr::tagged("provide", &self.provides));
        }
        if !self.requires.is_empty() {
            parts.push(SExpr::tagged("require", &self.requires));
        }
        SExpr::List(parts)
    }
}

impl ToSExpr for Model {
    fn to_sexpr(&self) -> SExpr {
        todo!("model")
    }
}

//             Def::Model(ref m) => sexp(vec![RcDoc::text("model"), m.name.to_doc(), m.val.to_doc()]),

impl ToSExpr for Form {
    fn to_sexpr(&self) -> SExpr {
        let mut parts = vec![SExpr::atom("form"), self.name.to_sexpr()];
        parts.extend(self.signatures.iter().map(ToSExpr::to_sexpr));
        SExpr::List(parts)
    }
}

impl ToSExpr for Instantiation {
    fn to_sexpr(&self) -> SExpr {
        let mut parts = vec![SExpr::atom("instantiate"), self.term.to_sexpr()];
        if let Some(form) = &self.form {
            parts.push(form.to_sexpr());
        } else {
            parts.extend(self.signatures.iter().map(ToSExpr::to_sexpr));
        }
        SExpr::List(parts)
    }
}

impl ToSExpr for Extern {
    fn to_sexpr(&self) -> SExpr {
        match self {
            Extern::Extractor {
                term,
                func,
                pos: _,
                infallible,
            } => {
                let mut parts = vec![SExpr::atom("extern"), SExpr::atom("extractor")];
                if *infallible {
                    parts.push(SExpr::atom("infallible"));
                }
                parts.push(term.to_sexpr());
                parts.push(func.to_sexpr());
                SExpr::List(parts)
            }
            Extern::Constructor { term, func, .. } => SExpr::List(vec![
                SExpr::atom("extern"),
                SExpr::atom("constructor"),
                term.to_sexpr(),
                func.to_sexpr(),
            ]),
            Extern::Const { name, ty, .. } => SExpr::List(vec![
                SExpr::atom("extern"),
                SExpr::atom("const"),
                SExpr::atom(format!("${}", name.0)),
                ty.to_sexpr(),
            ]),
        }
    }
}

impl ToSExpr for Converter {
    fn to_sexpr(&self) -> SExpr {
        SExpr::List(vec![
            SExpr::atom("convert"),
            self.inner_ty.to_sexpr(),
            self.outer_ty.to_sexpr(),
            self.term.to_sexpr(),
        ])
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
                parts.extend(variants.iter().map(ToSExpr::to_sexpr));
                SExpr::List(parts)
            }
        }
    }
}

impl ToSExpr for Variant {
    fn to_sexpr(&self) -> SExpr {
        let mut parts = vec![self.name.to_sexpr()];
        parts.extend(self.fields.iter().map(ToSExpr::to_sexpr));
        SExpr::List(parts)
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

impl ToSExpr for ModelType {
    fn to_sexpr(&self) -> SExpr {
        match self {
            ModelType::Unit => SExpr::atom("Unit"),
            ModelType::Int => SExpr::atom("Int"),
            ModelType::Bool => SExpr::atom("Bool"),
            ModelType::BitVec(Some(size)) => {
                SExpr::List(vec![SExpr::atom("bv"), SExpr::atom(size)])
            }
            ModelType::BitVec(None) => SExpr::List(vec![SExpr::atom("bv")]),
        }
    }
}

// impl Printable for ModelField {
//     fn to_doc(&self) -> RcDoc<()> {
//         sexp(vec![self.name.to_doc(), self.ty.to_doc()])
//     }
// }
//

impl ToSExpr for Signature {
    fn to_sexpr(&self) -> SExpr {
        SExpr::List(vec![
            SExpr::tagged("args", &self.args),
            SExpr::tagged("ret", std::slice::from_ref(&self.ret)),
            SExpr::tagged("canon", std::slice::from_ref(&self.canonical)),
        ])
    }
}

impl ToSExpr for SpecExpr {
    fn to_sexpr(&self) -> SExpr {
        match self {
            SpecExpr::ConstInt { val, .. } => SExpr::atom(val),
            SpecExpr::ConstBitVec { val, width, .. } => SExpr::atom(if *width % 4 == 0 {
                format!("#x{val:0width$x}", width = *width as usize / 4)
            } else {
                format!("#b{val:0width$b}", width = *width as usize)
            }),
            SpecExpr::ConstBool { val, .. } => SExpr::atom(if *val { "true" } else { "false" }),
            SpecExpr::ConstUnit { .. } => SExpr::List(Vec::new()),
            SpecExpr::Var { var, pos: _ } => var.to_sexpr(),
            SpecExpr::Op { op, args, .. } => {
                let mut parts = vec![op.to_sexpr()];
                parts.extend(args.iter().map(ToSExpr::to_sexpr));
                SExpr::List(parts)
            }
            SpecExpr::Pair { l, r } => SExpr::List(vec![l.to_sexpr(), r.to_sexpr()]),
            SpecExpr::Enum { name } => SExpr::List(vec![name.to_sexpr()]),
        }
    }
}

impl ToSExpr for SpecOp {
    fn to_sexpr(&self) -> SExpr {
        SExpr::atom(match self {
            SpecOp::Eq => "=",
            SpecOp::And => "and",
            SpecOp::Not => "not",
            SpecOp::Imp => "=>",
            SpecOp::Or => "or",
            SpecOp::Lte => "<=",
            SpecOp::Lt => "<",
            SpecOp::Gte => ">=",
            SpecOp::Gt => ">",
            SpecOp::BVNot => "bvnot",
            SpecOp::BVAnd => "bvand",
            SpecOp::BVOr => "bvor",
            SpecOp::BVXor => "bvxor",
            SpecOp::BVNeg => "bvneg",
            SpecOp::BVAdd => "bvadd",
            SpecOp::BVSub => "bvsub",
            SpecOp::BVMul => "bvmul",
            SpecOp::BVUdiv => "bvudiv",
            SpecOp::BVUrem => "bvurem",
            SpecOp::BVSdiv => "bvsdiv",
            SpecOp::BVSrem => "bvsrem",
            SpecOp::BVShl => "bvshl",
            SpecOp::BVLshr => "bvlshr",
            SpecOp::BVAshr => "bvashr",
            SpecOp::BVSaddo => "bvsaddo",
            SpecOp::BVUle => "bvule",
            SpecOp::BVUlt => "bvult",
            SpecOp::BVUgt => "bvugt",
            SpecOp::BVUge => "bvuge",
            SpecOp::BVSlt => "bvslt",
            SpecOp::BVSle => "bvsle",
            SpecOp::BVSgt => "bvsgt",
            SpecOp::BVSge => "bvsge",
            SpecOp::Rotr => "rotr",
            SpecOp::Rotl => "rotl",
            SpecOp::Extract => "extract",
            SpecOp::ZeroExt => "zero_ext",
            SpecOp::SignExt => "sign_ext",
            SpecOp::Concat => "concat",
            SpecOp::ConvTo => "conv_to",
            SpecOp::Int2BV => "int2bv",
            SpecOp::WidthOf => "widthof",
            SpecOp::If => "if",
            SpecOp::Switch => "switch",
            SpecOp::Popcnt => "popcnt",
            SpecOp::Rev => "rev",
            SpecOp::Cls => "cls",
            SpecOp::Clz => "clz",
            SpecOp::Subs => "subs",
            SpecOp::BV2Int => "bv2int",
            SpecOp::LoadEffect => "load_effect",
            SpecOp::StoreEffect => "store_effect",
        })
    }
}
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
