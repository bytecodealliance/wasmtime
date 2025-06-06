//! Printer for ISLE language.

use std::{io::Write, vec};

use crate::ast::*;

/// Print ISLE definitions.
pub fn print<W: Write>(defs: &[Def], width: usize, out: &mut W) -> std::io::Result<()> {
    for (i, def) in defs.iter().enumerate() {
        if i > 0 {
            writeln!(out)?;
        }
        print_node(def, width, out)?;
        writeln!(out)?;
    }
    Ok(())
}

pub fn print_node<N: ToSExpr, W: Write>(
    node: &N,
    width: usize,
    out: &mut W,
) -> std::io::Result<()> {
    let mut printer = Printer::new(out, width);
    let sexpr = node.to_sexpr();
    printer.print(&sexpr)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SExpr {
    Atom(String),
    Binding(String, Box<SExpr>),
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
}

struct Printer<'a, W: Write> {
    out: &'a mut W,
    col: usize,
    indent: usize,
    width: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Wrapping {
    Wrap,
    SingleLine,
}

impl<'a, W: Write> Printer<'a, W> {
    fn new(out: &'a mut W, width: usize) -> Self {
        Self {
            out,
            col: 0,
            indent: 0,
            width,
        }
    }

    fn print(&mut self, sexpr: &SExpr) -> std::io::Result<()> {
        self.print_wrapped(sexpr, Wrapping::Wrap)
    }

    fn print_wrapped(&mut self, sexpr: &SExpr, wrapping: Wrapping) -> std::io::Result<()> {
        match sexpr {
            SExpr::Atom(atom) => self.put(atom),
            SExpr::Binding(name, sexpr) => {
                self.put(name)?;
                self.put(" @ ")?;
                self.print_wrapped(sexpr, wrapping)
            }
            SExpr::List(items) => {
                if wrapping == Wrapping::SingleLine || self.fits(sexpr) {
                    self.put("(")?;
                    for (i, item) in items.iter().enumerate() {
                        if i > 0 {
                            self.put(" ")?;
                        }
                        self.print_wrapped(item, Wrapping::SingleLine)?;
                    }
                    self.put(")")
                } else {
                    let (first, rest) = items.split_first().expect("non-empty list");
                    self.put("(")?;
                    self.print_wrapped(first, wrapping)?;
                    self.indent += 1;
                    for item in rest {
                        self.nl()?;
                        self.print_wrapped(item, wrapping)?;
                    }
                    self.indent -= 1;
                    self.nl()?;
                    self.put(")")?;
                    Ok(())
                }
            }
        }
    }

    // Would the expressions fit in the current line?
    fn fits(&self, sexpr: &SExpr) -> bool {
        let Some(mut remaining) = self.width.checked_sub(self.col) else {
            return false;
        };
        let mut stack = vec![sexpr];
        while let Some(sexpr) = stack.pop() {
            match sexpr {
                SExpr::Atom(atom) => {
                    if atom.len() > remaining {
                        return false;
                    }
                    remaining -= atom.len();
                }
                SExpr::Binding(name, inner) => {
                    let binding_size = name.len() + 3; // " @ "
                    if binding_size > remaining {
                        return false;
                    }
                    remaining -= binding_size;
                    stack.push(inner);
                }
                SExpr::List(items) => {
                    // Account for parentheses and spaces
                    let punct_size = 2 + items.len() - 1; // "(" + ")" + spaces
                    if punct_size > remaining {
                        return false;
                    }
                    remaining -= punct_size;
                    stack.extend(items.iter().rev());
                }
            }
        }
        true
    }

    fn put(&mut self, s: &str) -> std::io::Result<()> {
        write!(self.out, "{s}")?;
        self.col += s.len();
        Ok(())
    }

    fn nl(&mut self) -> std::io::Result<()> {
        writeln!(self.out)?;
        self.col = 0;
        for _ in 0..self.indent {
            write!(self.out, "    ")?;
        }
        Ok(())
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
        SExpr::List(vec![
            SExpr::atom("model"),
            self.name.to_sexpr(),
            self.val.to_sexpr(),
        ])
    }
}

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

impl ToSExpr for ModelValue {
    fn to_sexpr(&self) -> SExpr {
        match self {
            ModelValue::TypeValue(mt) => SExpr::List(vec![SExpr::atom("type"), mt.to_sexpr()]),
            ModelValue::EnumValues(enumerators) => {
                let mut parts = vec![SExpr::atom("enum")];
                for (variant, value) in enumerators {
                    parts.push(SExpr::List(vec![variant.to_sexpr(), value.to_sexpr()]));
                }
                SExpr::List(parts)
            }
        }
    }
}

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

impl ToSExpr for Pattern {
    fn to_sexpr(&self) -> SExpr {
        match self {
            Pattern::Var { var, .. } => SExpr::atom(var.0.clone()),
            Pattern::BindPattern { var, subpat, .. } => {
                SExpr::Binding(var.0.clone(), Box::new(subpat.to_sexpr()))
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

impl ToSExpr for Ident {
    fn to_sexpr(&self) -> SExpr {
        SExpr::atom(self.0.clone())
    }
}
