use crate::solver::SolverCtx;
use easy_smt::SExpr;

// Future work: possibly move these into the annotation language or an SMTLIB prelude

// Encoding strategy borrowed from
// https://github.com/fitzgen/synth-loop-free-prog/blob/6d04857693e4688eff4a36537840ba682353c2f3/src/component.rs#L219
pub fn popcnt(s: &mut SolverCtx, ty: usize, x: SExpr, id: u32) -> SExpr {
    let mut bits: Vec<_> = (0..ty)
        .map(|i| s.zero_extend(7, s.smt.extract(i as i32, i as i32, x)))
        .collect();
    let initial = bits.pop().unwrap();
    let r = bits.iter().fold(initial, |a, b| s.smt.bvadd(a, *b));

    let id = format!("{ty}_{id}");
    let result = s.declare(
        format!("popcnt_{id}"),
        s.smt.list(vec![
            s.smt.atoms().und,
            s.smt.atom("BitVec"),
            s.smt.numeral(8),
        ]),
    );
    s.assume(s.smt.eq(result, r));
    result
}
