/// This file will be replaced by a parser that consumes annotations and produces
/// the same type of structure, but for now, manually construct these annotations.
use crate::annotation_ir::{BoundVar, Const, Expr, TermAnnotation, TermSignature, Type};

pub fn isle_annotation_for_term(term: &str) -> Option<TermAnnotation> {
    match term {
        // (spec (sig (args (x: bvX) (ret: bvX))
        //       (assumptions (= x ret)))
        "lower" | "put_in_reg" | "value_reg" | "first_result" | "inst_data" => {
            // No-op for now
            let arg = BoundVar::new("arg");
            let result = BoundVar::new("ret");
            let identity = Expr::binary(Expr::Eq, arg.as_expr(), result.as_expr());
            let func = TermSignature {
                args: vec![arg],
                ret: result,
            };
            Some(TermAnnotation::new(func, vec![identity]))
        }
        "value_type" => {
            let arg = BoundVar::new("arg");
            let result = BoundVar::new("ret");
            let ty_eq = Expr::binary(Expr::Eq, arg.as_expr(), Expr::TyWidth(0));
            let func = TermSignature {
                args: vec![arg],
                ret: result,
            };
            Some(TermAnnotation::new(func, vec![ty_eq]))
        }
        // (spec (sig (args x: bvX) (ret: bvX))
        //       (assumptions (= x ret)))
        "has_type" => {
            // Add an assertion on the type
            let ty_arg = BoundVar::new("ty");
            let arg = BoundVar::new("arg");
            let result = BoundVar::new("ret");
            let ty_eq = Expr::binary(Expr::Eq, ty_arg.as_expr(), Expr::TyWidth(0));
            let identity = Expr::binary(Expr::Eq, arg.as_expr(), result.as_expr());
            let func = TermSignature {
                args: vec![ty_arg, arg],
                ret: result,
            };
            Some(TermAnnotation::new(func, vec![ty_eq, identity]))
        }
        "fits_in_64" => {
            // Identity, but add assertion on type
            let arg = BoundVar::new("arg");
            let result = BoundVar::new("ret");
            let identity = Expr::binary(Expr::Eq, arg.as_expr(), result.as_expr());
            let ty_fits = Expr::binary(
                Expr::Lte,
                arg.as_expr(),
                Expr::Const(
                    Const {
                        ty: Type::Int,
                        value: 64_i128,
                        width: 128,
                    },
                    0,
                ),
            );
            let func = TermSignature {
                args: vec![arg],
                ret: result,
            };
            Some(TermAnnotation::new(func, vec![identity, ty_fits]))
        }
        "iadd" => {
            let a = BoundVar::new("a");
            let b = BoundVar::new("b");
            let r = BoundVar::new("r");
            let sem = Expr::binary(
                Expr::Eq,
                Expr::binary(Expr::BVAdd, a.as_expr(), b.as_expr()),
                r.as_expr(),
            );
            let func = TermSignature {
                args: vec![a, b],
                ret: r,
            };
            Some(TermAnnotation::new(func, vec![sem]))
        }
        // "imm12_from_negated_value" => {
        //     // Width: bv12
        //     let imm_arg = BoundVar::new("imm_arg");

        //     // Width: bvX
        //     let result = BoundVar::new("ret");

        //     // Negate and convert
        //     let as_ty = Expr::BVConvFrom(12, Box::new(imm_arg.as_expr()), 0);
        //     let res = Expr::unary(Expr::BVNeg, as_ty);
        //     let eq = Expr::binary(Expr::Eq, res, result.as_expr());
        //     let sig = TermSignature {
        //         args: vec![imm_arg],
        //         ret: result,
        //     };
        //     Some(TermAnnotation::new(sig, vec![eq]))
        // }
        // AVH TODO
        // "sub_imm" => {
        //     // Declare bound variables
        //     let ty_arg = BoundVar::new("ty");
        //     let reg_arg = BoundVar::new("reg");
        //     let result = BoundVar::new("ret");

        //     // Width: bv12
        //     let imm_arg = BoundVar::new("imm_arg");

        //     // Conversion step
        //     // AVH
        //     let as_ty = Expr::BVConvTo(64, Box::new(imm_arg.as_expr()), 0);
        //     let res = Expr::binary(Expr::BVSub, reg_arg.as_expr(), as_ty);
        //     let assertion = Expr::binary(Expr::Eq, res, result.as_expr());
        //     let func = TermSignature {
        //         args: vec![ty_arg, reg_arg, imm_arg],
        //         ret: result,
        //     };
        //     Some(TermAnnotation::new(func, vec![assertion]))
        //}
        "uextend" => {
            // let arg = BoundVar::new("arg");
            // let ret = BoundVar::new("ret");

            // let ext = Expr::BVConvTo(Box::new(Width::RegWidth), Box::new(arg.as_expr()), 0);
            // let assertion = Expr::Eq(Box::new(ret.as_expr()), Box::new(ext), 0);
            // let sig = TermSignature{
            //     args: vec![arg],
            //     ret: ret,
            // };
            // Some(TermAnnotation::new(sig, vec![assertion]))
            todo!()
        }
        "extend" => {
            // let a = BoundVar::new("a");
            // let b = BoundVar::new("b");
            // let c = BoundVar::new("c");
            // let d = BoundVar::new("d");
            // let ret = BoundVar::new("ret");

            // let extend_stuff = Expr::Conditional(
            //     Box::new(b.as_expr()),
            //     Box::new(Expr::binary(
            //         Expr::Eq, ret.as_expr(), Expr::binary(
            //             Expr::BVSignedConvToVarWidth, d.as_expr(), a.as_expr()),
            //     )),
            //     Box::new(Expr::binary(
            //         Expr::Eq, ret.as_expr(), Expr::binary(
            //             Expr::BVConvToVarWidth, d.as_expr(), a.as_expr()),
            //     )),
            // );
            // let pre = Expr::binary(Expr::Eq, Expr::unary(
            //     Expr::WidthOf, a.as_expr()), c.as_expr());

            // let func = TermSignature {
            //     args: vec![a, b, c, d],
            //     ret: ret,
            // };
            // Some(TermAnnotation::new(func, vec![extend_stuff, pre]))
            todo!()
        }
        _ => None,
    }
}
