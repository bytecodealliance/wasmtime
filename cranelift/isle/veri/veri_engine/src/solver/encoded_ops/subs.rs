use crate::solver::SolverCtx;
use easy_smt::SExpr;

// Build the results of a subtraction with flags. Put the 4 flags in the high bits.
// Encoding adapted from SAIL ISLA: https://github.com/rems-project/isla
//
// N: Set to 1 when the result of the operation is negative
// Z: Set to 1 when the result of the operation is zero
// C: Set to 1 when the operation results in a carry, or when a subtraction results in no borrow
// V: Set to 1 when the operation causes overflow
//
//   67  66  65  64  63 ...            0
//  [ N | Z | C | V |   ... result ...  ]
pub fn subs(s: &mut SolverCtx, ty: usize, x: SExpr, y: SExpr, id: u32) -> SExpr {
    let id = format!("{ty}_{id}");
    let (size, wide_size, x, y, zero, one, w_minus_one) = match ty {
        32 => (
            s.smt.numeral(32),
            s.smt.numeral(32 * 2),
            s.smt.extract(31, 0, x),
            s.smt.extract(31, 0, y),
            s.bv(0, 32),
            s.bv(1, 32 * 2),
            s.bv(31, 32),
        ),
        64 => (
            s.smt.numeral(64),
            s.smt.numeral(64 * 2),
            s.smt.extract(63, 0, x),
            s.smt.extract(63, 0, y),
            s.bv(0, 64),
            s.bv(1, 64 * 2),
            s.bv(63, 64),
        ),
        _ => unreachable!(),
    };

    let b0 = s.bv(0, 1);
    let b1 = s.bv(1, 1);

    // (define-const ynot (bvnot y))
    let ynot = s.declare(
        format!("ynot_{id}", id = id),
        s.smt
            .list(vec![s.smt.atoms().und, s.smt.atom("BitVec"), size]),
    );
    s.assume(s.smt.eq(ynot, s.smt.bvnot(y)));

    // (define-const
    //   subs_wide
    //   (bvadd (bvadd ((_ zero_extend 64) x) ((_ zero_extend 64) ynot)) #x00000000000000000000000000000001))
    let subs_wide = s.declare(
        format!("subs_wide_{id}", id = id),
        s.smt
            .list(vec![s.smt.atoms().und, s.smt.atom("BitVec"), wide_size]),
    );
    s.assume(s.smt.eq(
        subs_wide,
        s.smt.bvadd(
            s.smt.bvadd(s.zero_extend(ty, x), s.zero_extend(ty, ynot)),
            one,
        ),
    ));

    // (define-const subs ((_ extract 63 0) subs_wide))
    let subs = s.declare(
        format!("subs_{id}", id = id),
        s.smt
            .list(vec![s.smt.atoms().und, s.smt.atom("BitVec"), size]),
    );
    s.assume(s.smt.eq(
        subs,
        s.smt.extract((ty - 1).try_into().unwrap(), 0, subs_wide),
    ));

    // (define-const flags
    //  (concat (concat (concat
    //    ((_ extract 0 0) (bvlshr subs #x000000000000003f))
    //    (ite (= subs #x0000000000000000) #b1 #b0))
    //    (ite (= ((_ zero_extend 64) subs) subs_wide) #b0 #b1))
    //    (ite (= ((_ sign_extend 64) subs) (bvadd (bvadd ((_ sign_extend 64) x) ((_ sign_extend 64) ynot)) #x00000000000000000000000000000001)) #b0 #b1)))
    let flags = s.declare(
        format!("flags_{id}", id = id),
        s.smt.list(vec![
            s.smt.atoms().und,
            s.smt.atom("BitVec"),
            s.smt.numeral(4),
        ]),
    );

    // N: Set to 1 when the result of the operation is negative
    // Z: Set to 1 when the result of the operation is zero
    // C: Set to 1 when the operation results in a carry, or when a subtraction results in no borrow
    // V: Set to 1 when the operation causes overflow
    s.assume(
        s.smt.eq(
            flags,
            s.smt.concat(
                s.smt.concat(
                    s.smt.concat(
                        // N flag: result is negative
                        s.smt.extract(0, 0, s.smt.bvlshr(subs, w_minus_one)),
                        // Z flag: result is zero
                        s.smt.ite(s.smt.eq(subs, zero), b1, b0),
                    ),
                    // C flag: result has carry/subtraction has no borrow
                    s.smt
                        .ite(s.smt.eq(s.zero_extend(ty, subs), subs_wide), b0, b1),
                ),
                // V: operation causes overflow
                s.smt.ite(
                    s.smt.eq(
                        s.sign_extend(ty, subs),
                        s.smt.bvadd(
                            s.smt.bvadd(s.sign_extend(ty, x), s.sign_extend(ty, ynot)),
                            one,
                        ),
                    ),
                    b0,
                    b1,
                ),
            ),
        ),
    );

    let ret = s.declare(
        format!("subs_ret_{id}", id = id),
        s.smt.list(vec![
            s.smt.atoms().und,
            s.smt.atom("BitVec"),
            s.smt.numeral(68),
        ]),
    );

    s.assume(s.smt.eq(
        ret,
        match ty {
            // Pad 32 back to full reg width of 64 before adding flags to the left
            32 => s.smt.concat(flags, s.zero_extend(ty, subs)),
            64 => s.smt.concat(flags, subs),
            _ => unreachable!(),
        },
    ));
    ret
}
