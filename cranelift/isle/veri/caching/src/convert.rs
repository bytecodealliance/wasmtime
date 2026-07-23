//! S-expression conversions: between two contexts' arenas, and to/from a
//! JSON tree for cache storage.
//!
//! These walks are what let the caching layer avoid parsing SMT-LIB2 text:
//! recorded commands are replayed into a freshly spawned solver context by
//! copying their trees across arenas, and cached responses are stored as JSON
//! trees (atoms as strings, lists as arrays) rather than display text.

use std::io;

use easy_smt::{Context, SExpr, SExprData};
use serde_json::Value;

/// Copy an s-expression from `src`'s arena into `dst`'s arena.
///
/// String literals (which only ever appear in solver *responses*, e.g.
/// `(error "msg")`) are copied as verbatim quoted atoms: they display
/// identically, and no caller inspects them structurally.
pub fn copy(src: &Context, dst: &Context, expr: SExpr) -> SExpr {
    match src.get(expr) {
        SExprData::Atom(a) => dst.atom(a),
        SExprData::String(s) => dst.atom(format!("\"{s}\"")),
        SExprData::List(items) => {
            let items = items.to_vec();
            let copied = items.into_iter().map(|e| copy(src, dst, e)).collect();
            dst.list(copied)
        }
    }
}

/// Encode an s-expression as a JSON tree: atoms as strings, lists as arrays.
pub fn to_json(ctx: &Context, expr: SExpr) -> Value {
    match ctx.get(expr) {
        SExprData::Atom(a) => Value::String(a.to_string()),
        SExprData::String(s) => Value::String(format!("\"{s}\"")),
        SExprData::List(items) => {
            let items = items.to_vec();
            Value::Array(items.into_iter().map(|e| to_json(ctx, e)).collect())
        }
    }
}

/// Rebuild an s-expression in `ctx`'s arena from a JSON tree.
pub fn from_json(ctx: &Context, value: &Value) -> io::Result<SExpr> {
    match value {
        Value::String(s) => Ok(ctx.atom(s)),
        Value::Array(items) => {
            let exprs = items
                .iter()
                .map(|v| from_json(ctx, v))
                .collect::<io::Result<Vec<_>>>()?;
            Ok(ctx.list(exprs))
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid cached s-expression: {value}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use easy_smt::ContextBuilder;
    use serde_json::json;

    #[test]
    fn test_copy_across_arenas() {
        let a = ContextBuilder::new().build().unwrap();
        let b = ContextBuilder::new().build().unwrap();

        let expr = a.list(vec![
            a.atom("declare-const"),
            a.atom("e0"),
            a.bit_vec_sort(a.numeral(64)),
        ]);
        let copied = copy(&a, &b, expr);
        assert_eq!(
            b.display(copied).to_string(),
            "(declare-const e0 (_ BitVec 64))"
        );
        // Copied atoms are interned in the destination: they compare equal to
        // atoms built there directly.
        let sat = copy(&a, &b, a.atoms().sat);
        assert_eq!(sat, b.atoms().sat);
    }

    #[test]
    fn test_json_roundtrip() {
        let ctx = ContextBuilder::new().build().unwrap();

        let expr = ctx.list(vec![
            ctx.list(vec![ctx.atom("e0"), ctx.atom("#b0101")]),
            ctx.list(vec![
                ctx.atom("e1"),
                ctx.list(vec![
                    ctx.atom("as"),
                    ctx.atom("@a"),
                    ctx.atom("Unspecified"),
                ]),
            ]),
        ]);
        let value = to_json(&ctx, expr);
        assert_eq!(
            value,
            json!([["e0", "#b0101"], ["e1", ["as", "@a", "Unspecified"]]])
        );
        let back = from_json(&ctx, &value).unwrap();
        assert_eq!(ctx.display(back).to_string(), ctx.display(expr).to_string());

        // Atoms rebuilt from JSON are interned normally.
        let sat = from_json(&ctx, &json!("sat")).unwrap();
        assert_eq!(sat, ctx.atoms().sat);
    }

    #[test]
    fn test_from_json_rejects_non_sexpr() {
        let ctx = ContextBuilder::new().build().unwrap();
        assert!(from_json(&ctx, &json!(42)).is_err());
        assert!(from_json(&ctx, &json!({"a": 1})).is_err());
        assert!(from_json(&ctx, &json!(null)).is_err());
    }
}
