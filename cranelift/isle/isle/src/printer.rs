//! Printer for ISLE language.

use crate::ast::*;
use crate::error::Errors;
use pretty::{Doc, Pretty, RcAllocator, RcDoc};
use std::io::Write;

/// Printable is a trait satisfied by AST nodes that can be printed.
pub trait Printable {
    /// Map the node to a pretty document.
    fn to_doc(&self) -> RcDoc<()>;
}

/// Print the given AST node with specified line width.
pub fn print<N, W>(node: &N, width: usize, out: &mut W) -> Result<(), Errors>
where
    N: Printable,
    W: ?Sized + Write,
{
    node.to_doc()
        .render(width, out)
        .map_err(|e| Errors::from_io(e, "failed to print isle"))
}

/// Dump AST node to standard output.
pub fn dump<N: Printable>(node: &N) -> Result<(), Errors> {
    let mut stdout = std::io::stdout();
    print(node, 78, &mut stdout)
}

impl Printable for Defs {
    fn to_doc(&self) -> RcDoc<()> {
        let sep = RcDoc::hardline().append(Doc::hardline());
        RcDoc::intersperse(self.defs.iter().map(|d| d.to_doc()), sep).append(Doc::hardline())
    }
}

impl Printable for Def {
    fn to_doc(&self) -> RcDoc<()> {
        match self {
            Def::Pragma(_) => unimplemented!("pragmas not supported"),
            Def::Type(ref t) => {
                let mut parts = vec![RcDoc::text("type")];
                parts.push(t.name.to_doc());
                if t.is_extern {
                    parts.push(RcDoc::text("extern"));
                }
                if t.is_nodebug {
                    parts.push(RcDoc::text("nodebug"));
                }
                parts.push(t.ty.to_doc());
                sexp(parts)
            }
            Def::Rule(ref r) => {
                let mut parts = Vec::new();
                parts.push(RcDoc::text("rule"));
                if let Some(prio) = &r.prio {
                    parts.push(RcDoc::as_string(prio));
                }
                parts.push(r.pattern.to_doc());
                parts.extend(r.iflets.iter().map(|il| il.to_doc()));
                parts.push(r.expr.to_doc());
                sexp(parts)
            }
            Def::Extractor(ref e) => sexp(vec![
                RcDoc::text("extractor"),
                sexp(
                    Vec::from([e.term.to_doc()])
                        .into_iter()
                        .chain(e.args.iter().map(|v| v.to_doc())),
                ),
                e.template.to_doc(),
            ]),
            Def::Decl(ref d) => {
                let mut parts = Vec::new();
                parts.push(RcDoc::text("decl"));
                if d.pure {
                    parts.push(RcDoc::text("pure"));
                }
                if d.multi {
                    parts.push(RcDoc::text("multi"));
                }
                if d.partial {
                    parts.push(RcDoc::text("partial"));
                }
                parts.push(d.term.to_doc());
                parts.push(sexp(d.arg_tys.iter().map(|ty| ty.to_doc())));
                parts.push(d.ret_ty.to_doc());
                sexp(parts)
            }
            Def::Extern(ref e) => e.to_doc(),
            Def::Converter(ref c) => sexp(vec![
                RcDoc::text("convert"),
                c.inner_ty.to_doc(),
                c.outer_ty.to_doc(),
                c.term.to_doc(),
            ]),
        }
    }
}

impl Printable for Ident {
    fn to_doc(&self) -> RcDoc<()> {
        RcDoc::text(self.0.clone())
    }
}

impl Printable for TypeValue {
    fn to_doc(&self) -> RcDoc<()> {
        match self {
            TypeValue::Primitive(ref name, _) => {
                sexp(vec![RcDoc::text("primitive"), name.to_doc()])
            }
            TypeValue::Enum(ref variants, _) => sexp(
                // TODO(mbm): convenience for sexp with a fixed first element
                Vec::from([RcDoc::text("enum")])
                    .into_iter()
                    .chain(variants.iter().map(|v| v.to_doc())),
            ),
        }
    }
}

impl Printable for Variant {
    fn to_doc(&self) -> RcDoc<()> {
        sexp(
            // TODO(mbm): convenience for sexp with a fixed first element
            Vec::from([self.name.to_doc()])
                .into_iter()
                .chain(self.fields.iter().map(|f| f.to_doc())),
        )
    }
}

impl Printable for Field {
    fn to_doc(&self) -> RcDoc<()> {
        sexp(vec![self.name.to_doc(), self.ty.to_doc()])
    }
}

impl Printable for Pattern {
    fn to_doc(&self) -> RcDoc<()> {
        match self {
            Pattern::Var { var, .. } => var.to_doc(),
            Pattern::BindPattern { var, subpat, .. } => RcDoc::intersperse(
                vec![var.to_doc(), RcDoc::text("@"), subpat.to_doc()],
                Doc::space(),
            ),
            Pattern::ConstInt { val, .. } => RcDoc::as_string(val),
            Pattern::ConstPrim { val, .. } => RcDoc::text("$").append(val.to_doc()),
            Pattern::Wildcard { .. } => RcDoc::text("_"),
            Pattern::Term { sym, args, .. } => sexp(
                // TODO(mbm): convenience for sexp with a fixed first element
                Vec::from([sym.to_doc()])
                    .into_iter()
                    .chain(args.iter().map(|f| f.to_doc())),
            ),
            Pattern::And { subpats, .. } => sexp(
                Vec::from([RcDoc::text("and")])
                    .into_iter()
                    .chain(subpats.iter().map(|p| p.to_doc())),
            ),
            Pattern::MacroArg { .. } => unimplemented!("macro arguments are for internal use only"),
        }
    }
}

impl Printable for IfLet {
    fn to_doc(&self) -> RcDoc<()> {
        // TODO(mbm): `if` shorthand when pattern is wildcard
        sexp(vec![
            RcDoc::text("if-let"),
            self.pattern.to_doc(),
            self.expr.to_doc(),
        ])
    }
}

impl Printable for Expr {
    fn to_doc(&self) -> RcDoc<()> {
        match self {
            Expr::Term { sym, args, .. } => sexp(
                // TODO(mbm): convenience for sexp with a fixed first element
                Vec::from([sym.to_doc()])
                    .into_iter()
                    .chain(args.iter().map(|f| f.to_doc())),
            ),
            Expr::Var { name, .. } => name.to_doc(),
            Expr::ConstInt { val, .. } => RcDoc::as_string(val),
            Expr::ConstPrim { val, .. } => RcDoc::text("$").append(val.to_doc()),
            Expr::Let { defs, body, .. } => {
                let mut parts = Vec::new();
                parts.push(RcDoc::text("let"));
                parts.push(sexp(defs.iter().map(|d| d.to_doc())));
                parts.push(body.to_doc());
                sexp(parts)
            }
        }
    }
}

impl Printable for LetDef {
    fn to_doc(&self) -> RcDoc<()> {
        sexp(vec![self.var.to_doc(), self.ty.to_doc(), self.val.to_doc()])
    }
}

impl Printable for Extern {
    fn to_doc(&self) -> RcDoc<()> {
        match self {
            Extern::Extractor {
                term,
                func,
                pos: _,
                infallible,
            } => {
                let mut parts = vec![RcDoc::text("extern"), RcDoc::text("extractor")];
                if *infallible {
                    parts.push(RcDoc::text("infallible"));
                }
                parts.push(term.to_doc());
                parts.push(func.to_doc());
                sexp(parts)
            }
            Extern::Constructor { term, func, .. } => sexp(vec![
                RcDoc::text("extern"),
                RcDoc::text("constructor"),
                term.to_doc(),
                func.to_doc(),
            ]),
            Extern::Const { name, ty, .. } => sexp(vec![
                RcDoc::text("extern"),
                RcDoc::text("const"),
                RcDoc::text("$").append(name.to_doc()),
                ty.to_doc(),
            ]),
        }
    }
}

fn sexp<'a, I, A>(docs: I) -> RcDoc<'a, A>
where
    I: IntoIterator,
    I::Item: Pretty<'a, RcAllocator, A>,
    A: Clone,
{
    RcDoc::text("(")
        .append(RcDoc::intersperse(docs, Doc::line()).nest(4).group())
        .append(RcDoc::text(")"))
}
