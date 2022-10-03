/// This file handles renaming bound variables in assumption expressions,
/// which is necessary to use annotations that might share variable names.
use veri_ir::{
    BoundVar, Expr, FunctionApplication, UndefinedTerm, VIRTermAnnotation, VIRTermSignature,
};

pub fn rename_annotation_vars<F>(a: VIRTermAnnotation, rename: F) -> VIRTermAnnotation
where
    F: Fn(&BoundVar) -> BoundVar + Copy,
{
    let args = a.func().args.iter().map(rename).collect();
    let ret = rename(&a.func().ret);
    VIRTermAnnotation::new(
        VIRTermSignature { args, ret },
        a.assertions()
            .iter()
            .map(|e| rename_vir_expr(e.clone(), rename))
            .collect(),
    )
}

fn rename_vir_expr<F>(expr: Expr, rename: F) -> Expr
where
    F: Fn(&BoundVar) -> BoundVar + Copy,
{
    todo!()
}
