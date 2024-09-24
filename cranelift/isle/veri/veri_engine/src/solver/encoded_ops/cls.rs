use crate::solver::SolverCtx;
use easy_smt::SExpr;

// Future work: possibly move these into the annotation language or an SMTLIB prelude
// Adapted from https://stackoverflow.com/questions/23856596/how-to-count-leading-zeros-in-a-32-bit-unsigned-integer

pub fn cls64(solver: &mut SolverCtx, x: SExpr, id: u32) -> SExpr {
    // Generated code.
    // total zeros counter
    let zret0 = solver.declare(
        format!("zret0_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(solver.smt.eq(
        zret0,
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("bv0"),
            solver.smt.numeral(64),
        ]),
    ));
    // round 1
    let zret1 = solver.declare(
        format!("zret1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let zy32 = solver.declare(
        format!("zy32_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let zx32 = solver.declare(
        format!("zx32_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(solver.smt.eq(
        zy32,
        solver.smt.bvlshr(x, solver.smt.atom("#x0000000000000020")),
    ));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy32,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(zret1, zret0),
        solver.smt.eq(
            zret1,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret0,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv32"),
                    solver.smt.numeral(64),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy32,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(zx32, zy32),
        solver.smt.eq(zx32, x),
    ]));
    // round 2
    let zret2 = solver.declare(
        format!("zret2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let zy16 = solver.declare(
        format!("zy16_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let zx16 = solver.declare(
        format!("zx16_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(
        solver.smt.eq(
            zy16,
            solver
                .smt
                .bvlshr(zx32, solver.smt.atom("#x0000000000000010")),
        ),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy16,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(zret2, zret1),
        solver.smt.eq(
            zret2,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv16"),
                    solver.smt.numeral(64),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy16,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(zx16, zy16),
        solver.smt.eq(zx16, zx32),
    ]));
    // round 3
    let zret3 = solver.declare(
        format!("zret3_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let zy8 = solver.declare(
        format!("zy8_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let zx8 = solver.declare(
        format!("zx8_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(
        solver.smt.eq(
            zy8,
            solver
                .smt
                .bvlshr(zx16, solver.smt.atom("#x0000000000000008")),
        ),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy8,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(zret3, zret2),
        solver.smt.eq(
            zret3,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv8"),
                    solver.smt.numeral(64),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy8,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(zx8, zy8),
        solver.smt.eq(zx8, zx16),
    ]));
    // round 4
    let zret4 = solver.declare(
        format!("zret4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let zy4 = solver.declare(
        format!("zy4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let zx4 = solver.declare(
        format!("zx4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(
        solver.smt.eq(
            zy4,
            solver
                .smt
                .bvlshr(zx8, solver.smt.atom("#x0000000000000004")),
        ),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(zret4, zret3),
        solver.smt.eq(
            zret4,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret3,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv4"),
                    solver.smt.numeral(64),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(zx4, zy4),
        solver.smt.eq(zx4, zx8),
    ]));
    // round 5
    let zret5 = solver.declare(
        format!("zret5_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let zy2 = solver.declare(
        format!("zy2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let zx2 = solver.declare(
        format!("zx2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(
        solver.smt.eq(
            zy2,
            solver
                .smt
                .bvlshr(zx4, solver.smt.atom("#x0000000000000002")),
        ),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(zret5, zret4),
        solver.smt.eq(
            zret5,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv2"),
                    solver.smt.numeral(64),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(zx2, zy2),
        solver.smt.eq(zx2, zx4),
    ]));
    // round 6
    let zret6 = solver.declare(
        format!("zret6_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let zy1 = solver.declare(
        format!("zy1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let zx1 = solver.declare(
        format!("zx1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(
        solver.smt.eq(
            zy1,
            solver
                .smt
                .bvlshr(zx2, solver.smt.atom("#x0000000000000001")),
        ),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(zret6, zret5),
        solver.smt.eq(
            zret6,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret5,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(64),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(zx1, zy1),
        solver.smt.eq(zx1, zx2),
    ]));
    // last round
    let zret7 = solver.declare(
        format!("zret7_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zx1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(zret7, zret6),
        solver.smt.eq(
            zret7,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret6,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(64),
                ]),
            ]),
        ),
    ]));
    let clzret = solver.declare(
        format!("clzret_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.eq(
            zret7,
            solver.smt.list(vec![
                solver.smt.atoms().und,
                solver.smt.atom("bv0"),
                solver.smt.numeral(64),
            ]),
        ),
        solver.smt.eq(clzret, zret7),
        solver.smt.eq(
            clzret,
            solver.smt.list(vec![
                solver.smt.atom("bvsub"),
                zret7,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(64),
                ]),
            ]),
        ),
    ]));
    // total zeros counter
    let sret0 = solver.declare(
        format!("sret0_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(solver.smt.eq(
        sret0,
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("bv0"),
            solver.smt.numeral(64),
        ]),
    ));
    // round 1
    let sret1 = solver.declare(
        format!("sret1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let sy32 = solver.declare(
        format!("sy32_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let sx32 = solver.declare(
        format!("sx32_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(solver.smt.eq(
        sy32,
        solver.smt.bvashr(x, solver.smt.atom("#x0000000000000020")),
    ));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy32,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv18446744073709551615"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(sret1, sret0),
        solver.smt.eq(
            sret1,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret0,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv32"),
                    solver.smt.numeral(64),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy32,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv18446744073709551615"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(sx32, sy32),
        solver.smt.eq(sx32, x),
    ]));
    // round 2
    let sret2 = solver.declare(
        format!("sret2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let sy16 = solver.declare(
        format!("sy16_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let sx16 = solver.declare(
        format!("sx16_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(
        solver.smt.eq(
            sy16,
            solver
                .smt
                .bvashr(sx32, solver.smt.atom("#x0000000000000010")),
        ),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy16,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv18446744073709551615"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(sret2, sret1),
        solver.smt.eq(
            sret2,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv16"),
                    solver.smt.numeral(64),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy16,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv18446744073709551615"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(sx16, sy16),
        solver.smt.eq(sx16, sx32),
    ]));
    // round 3
    let sret3 = solver.declare(
        format!("sret3_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let sy8 = solver.declare(
        format!("sy8_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let sx8 = solver.declare(
        format!("sx8_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(
        solver.smt.eq(
            sy8,
            solver
                .smt
                .bvashr(sx16, solver.smt.atom("#x0000000000000008")),
        ),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy8,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv18446744073709551615"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(sret3, sret2),
        solver.smt.eq(
            sret3,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv8"),
                    solver.smt.numeral(64),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy8,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv18446744073709551615"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(sx8, sy8),
        solver.smt.eq(sx8, sx16),
    ]));
    // round 4
    let sret4 = solver.declare(
        format!("sret4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let sy4 = solver.declare(
        format!("sy4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let sx4 = solver.declare(
        format!("sx4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(
        solver.smt.eq(
            sy4,
            solver
                .smt
                .bvashr(sx8, solver.smt.atom("#x0000000000000004")),
        ),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv18446744073709551615"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(sret4, sret3),
        solver.smt.eq(
            sret4,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret3,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv4"),
                    solver.smt.numeral(64),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv18446744073709551615"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(sx4, sy4),
        solver.smt.eq(sx4, sx8),
    ]));
    // round 5
    let sret5 = solver.declare(
        format!("sret5_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let sy2 = solver.declare(
        format!("sy2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let sx2 = solver.declare(
        format!("sx2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(
        solver.smt.eq(
            sy2,
            solver
                .smt
                .bvashr(sx4, solver.smt.atom("#x0000000000000002")),
        ),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv18446744073709551615"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(sret5, sret4),
        solver.smt.eq(
            sret5,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv2"),
                    solver.smt.numeral(64),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv18446744073709551615"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(sx2, sy2),
        solver.smt.eq(sx2, sx4),
    ]));
    // round 6
    let sret6 = solver.declare(
        format!("sret6_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let sy1 = solver.declare(
        format!("sy1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let sx1 = solver.declare(
        format!("sx1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(
        solver.smt.eq(
            sy1,
            solver
                .smt
                .bvashr(sx2, solver.smt.atom("#x0000000000000001")),
        ),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv18446744073709551615"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(sret6, sret5),
        solver.smt.eq(
            sret6,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret5,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(64),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv18446744073709551615"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(sx1, sy1),
        solver.smt.eq(sx1, sx2),
    ]));
    // last round
    let sret7 = solver.declare(
        format!("sret7_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sx1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv18446744073709551615"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(sret7, sret6),
        solver.smt.eq(
            sret7,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret6,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(64),
                ]),
            ]),
        ),
    ]));
    let clsret = solver.declare(
        format!("clsret_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.eq(
            sret7,
            solver.smt.list(vec![
                solver.smt.atoms().und,
                solver.smt.atom("bv0"),
                solver.smt.numeral(64),
            ]),
        ),
        solver.smt.eq(clsret, sret7),
        solver.smt.eq(
            clsret,
            solver.smt.list(vec![
                solver.smt.atom("bvsub"),
                sret7,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(64),
                ]),
            ]),
        ),
    ]));
    let cls64ret = solver.declare(
        format!("cls64ret_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("bvsle"),
            solver.smt.list(vec![
                solver.smt.atoms().und,
                solver.smt.atom("bv0"),
                solver.smt.numeral(64),
            ]),
            x,
        ]),
        solver.smt.eq(cls64ret, clzret),
        solver.smt.eq(cls64ret, clsret),
    ]));

    cls64ret
}

pub fn cls32(solver: &mut SolverCtx, x: SExpr, id: u32) -> SExpr {
    let x = solver.smt.extract(31, 0, x);

    // Generated code.
    // total zeros counter
    let zret0 = solver.declare(
        format!("zret0_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(solver.smt.eq(
        zret0,
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("bv0"),
            solver.smt.numeral(32),
        ]),
    ));
    // round 1
    let zret2 = solver.declare(
        format!("zret2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let zy16 = solver.declare(
        format!("zy16_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let zx16 = solver.declare(
        format!("zx16_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(zy16, solver.smt.bvlshr(x, solver.smt.atom("#x00000010"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy16,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(zret2, zret0),
        solver.smt.eq(
            zret2,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret0,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv16"),
                    solver.smt.numeral(32),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy16,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(zx16, zy16),
        solver.smt.eq(zx16, x),
    ]));
    // round 2
    let zret3 = solver.declare(
        format!("zret3_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let zy8 = solver.declare(
        format!("zy8_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let zx8 = solver.declare(
        format!("zx8_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(zy8, solver.smt.bvlshr(zx16, solver.smt.atom("#x00000008"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy8,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(zret3, zret2),
        solver.smt.eq(
            zret3,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv8"),
                    solver.smt.numeral(32),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy8,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(zx8, zy8),
        solver.smt.eq(zx8, zx16),
    ]));
    // round 3
    let zret4 = solver.declare(
        format!("zret4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let zy4 = solver.declare(
        format!("zy4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let zx4 = solver.declare(
        format!("zx4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(zy4, solver.smt.bvlshr(zx8, solver.smt.atom("#x00000004"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(zret4, zret3),
        solver.smt.eq(
            zret4,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret3,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv4"),
                    solver.smt.numeral(32),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(zx4, zy4),
        solver.smt.eq(zx4, zx8),
    ]));
    // round 4
    let zret5 = solver.declare(
        format!("zret5_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let zy2 = solver.declare(
        format!("zy2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let zx2 = solver.declare(
        format!("zx2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(zy2, solver.smt.bvlshr(zx4, solver.smt.atom("#x00000002"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(zret5, zret4),
        solver.smt.eq(
            zret5,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv2"),
                    solver.smt.numeral(32),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(zx2, zy2),
        solver.smt.eq(zx2, zx4),
    ]));
    // round 5
    let zret6 = solver.declare(
        format!("zret6_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let zy1 = solver.declare(
        format!("zy1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let zx1 = solver.declare(
        format!("zx1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(zy1, solver.smt.bvlshr(zx2, solver.smt.atom("#x00000001"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(zret6, zret5),
        solver.smt.eq(
            zret6,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret5,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(32),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(zx1, zy1),
        solver.smt.eq(zx1, zx2),
    ]));
    // last round
    let zret7 = solver.declare(
        format!("zret7_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zx1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(zret7, zret6),
        solver.smt.eq(
            zret7,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret6,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(32),
                ]),
            ]),
        ),
    ]));
    let clzret = solver.declare(
        format!("clzret_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.eq(
            zret7,
            solver.smt.list(vec![
                solver.smt.atoms().und,
                solver.smt.atom("bv0"),
                solver.smt.numeral(32),
            ]),
        ),
        solver.smt.eq(clzret, zret7),
        solver.smt.eq(
            clzret,
            solver.smt.list(vec![
                solver.smt.atom("bvsub"),
                zret7,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(32),
                ]),
            ]),
        ),
    ]));
    // total zeros counter
    let sret0 = solver.declare(
        format!("sret0_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(solver.smt.eq(
        sret0,
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("bv0"),
            solver.smt.numeral(32),
        ]),
    ));
    // round 1
    let sret2 = solver.declare(
        format!("sret2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let sy16 = solver.declare(
        format!("sy16_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let sx16 = solver.declare(
        format!("sx16_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(sy16, solver.smt.bvashr(x, solver.smt.atom("#x00000010"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy16,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv4294967295"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(sret2, sret0),
        solver.smt.eq(
            sret2,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret0,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv16"),
                    solver.smt.numeral(32),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy16,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv4294967295"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(sx16, sy16),
        solver.smt.eq(sx16, x),
    ]));
    // round 2
    let sret3 = solver.declare(
        format!("sret3_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let sy8 = solver.declare(
        format!("sy8_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let sx8 = solver.declare(
        format!("sx8_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(sy8, solver.smt.bvashr(sx16, solver.smt.atom("#x00000008"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy8,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv4294967295"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(sret3, sret2),
        solver.smt.eq(
            sret3,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv8"),
                    solver.smt.numeral(32),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy8,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv4294967295"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(sx8, sy8),
        solver.smt.eq(sx8, sx16),
    ]));
    // round 3
    let sret4 = solver.declare(
        format!("sret4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let sy4 = solver.declare(
        format!("sy4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let sx4 = solver.declare(
        format!("sx4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(sy4, solver.smt.bvashr(sx8, solver.smt.atom("#x00000004"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv4294967295"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(sret4, sret3),
        solver.smt.eq(
            sret4,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret3,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv4"),
                    solver.smt.numeral(32),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv4294967295"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(sx4, sy4),
        solver.smt.eq(sx4, sx8),
    ]));
    // round 4
    let sret5 = solver.declare(
        format!("sret5_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let sy2 = solver.declare(
        format!("sy2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let sx2 = solver.declare(
        format!("sx2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(sy2, solver.smt.bvashr(sx4, solver.smt.atom("#x00000002"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv4294967295"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(sret5, sret4),
        solver.smt.eq(
            sret5,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv2"),
                    solver.smt.numeral(32),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv4294967295"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(sx2, sy2),
        solver.smt.eq(sx2, sx4),
    ]));
    // round 5
    let sret6 = solver.declare(
        format!("sret6_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let sy1 = solver.declare(
        format!("sy1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let sx1 = solver.declare(
        format!("sx1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(sy1, solver.smt.bvashr(sx2, solver.smt.atom("#x00000001"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv4294967295"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(sret6, sret5),
        solver.smt.eq(
            sret6,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret5,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(32),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv4294967295"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(sx1, sy1),
        solver.smt.eq(sx1, sx2),
    ]));
    // last round
    let sret7 = solver.declare(
        format!("sret7_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sx1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv4294967295"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(sret7, sret6),
        solver.smt.eq(
            sret7,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret6,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(32),
                ]),
            ]),
        ),
    ]));
    let clsret = solver.declare(
        format!("clsret_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.eq(
            sret7,
            solver.smt.list(vec![
                solver.smt.atoms().und,
                solver.smt.atom("bv0"),
                solver.smt.numeral(32),
            ]),
        ),
        solver.smt.eq(clsret, sret7),
        solver.smt.eq(
            clsret,
            solver.smt.list(vec![
                solver.smt.atom("bvsub"),
                sret7,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(32),
                ]),
            ]),
        ),
    ]));
    let cls32ret = solver.declare(
        format!("cls32ret_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("bvsle"),
            solver.smt.list(vec![
                solver.smt.atoms().und,
                solver.smt.atom("bv0"),
                solver.smt.numeral(32),
            ]),
            x,
        ]),
        solver.smt.eq(cls32ret, clzret),
        solver.smt.eq(cls32ret, clsret),
    ]));

    if solver.find_widths {
        let padding = solver.new_fresh_bits(solver.bitwidth - 32);
        solver.smt.concat(padding, cls32ret)
    } else {
        cls32ret
    }
}

pub fn cls16(solver: &mut SolverCtx, x: SExpr, id: u32) -> SExpr {
    let x = solver.smt.extract(15, 0, x);

    // Generated code.
    // total zeros counter
    let zret0 = solver.declare(
        format!("zret0_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    solver.assume(solver.smt.eq(
        zret0,
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("bv0"),
            solver.smt.numeral(16),
        ]),
    ));
    // round 1
    let zret3 = solver.declare(
        format!("zret3_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let zy8 = solver.declare(
        format!("zy8_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let zx8 = solver.declare(
        format!("zx8_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(zy8, solver.smt.bvlshr(x, solver.smt.atom("#x0008"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy8,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(zret3, zret0),
        solver.smt.eq(
            zret3,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret0,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv8"),
                    solver.smt.numeral(16),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy8,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(zx8, zy8),
        solver.smt.eq(zx8, x),
    ]));
    // round 2
    let zret4 = solver.declare(
        format!("zret4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let zy4 = solver.declare(
        format!("zy4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let zx4 = solver.declare(
        format!("zx4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(zy4, solver.smt.bvlshr(zx8, solver.smt.atom("#x0004"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(zret4, zret3),
        solver.smt.eq(
            zret4,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret3,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv4"),
                    solver.smt.numeral(16),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(zx4, zy4),
        solver.smt.eq(zx4, zx8),
    ]));
    // round 3
    let zret5 = solver.declare(
        format!("zret5_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let zy2 = solver.declare(
        format!("zy2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let zx2 = solver.declare(
        format!("zx2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(zy2, solver.smt.bvlshr(zx4, solver.smt.atom("#x0002"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(zret5, zret4),
        solver.smt.eq(
            zret5,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv2"),
                    solver.smt.numeral(16),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(zx2, zy2),
        solver.smt.eq(zx2, zx4),
    ]));
    // round 4
    let zret6 = solver.declare(
        format!("zret6_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let zy1 = solver.declare(
        format!("zy1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let zx1 = solver.declare(
        format!("zx1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(zy1, solver.smt.bvlshr(zx2, solver.smt.atom("#x0001"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(zret6, zret5),
        solver.smt.eq(
            zret6,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret5,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(16),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(zx1, zy1),
        solver.smt.eq(zx1, zx2),
    ]));
    // last round
    let zret7 = solver.declare(
        format!("zret7_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zx1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(zret7, zret6),
        solver.smt.eq(
            zret7,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret6,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(16),
                ]),
            ]),
        ),
    ]));
    let clzret = solver.declare(
        format!("clzret_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.eq(
            zret7,
            solver.smt.list(vec![
                solver.smt.atoms().und,
                solver.smt.atom("bv0"),
                solver.smt.numeral(16),
            ]),
        ),
        solver.smt.eq(clzret, zret7),
        solver.smt.eq(
            clzret,
            solver.smt.list(vec![
                solver.smt.atom("bvsub"),
                zret7,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(16),
                ]),
            ]),
        ),
    ]));
    // total zeros counter
    let sret0 = solver.declare(
        format!("sret0_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    solver.assume(solver.smt.eq(
        sret0,
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("bv0"),
            solver.smt.numeral(16),
        ]),
    ));
    // round 1
    let sret3 = solver.declare(
        format!("sret3_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let sy8 = solver.declare(
        format!("sy8_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let sx8 = solver.declare(
        format!("sx8_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(sy8, solver.smt.bvashr(x, solver.smt.atom("#x0008"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy8,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv65535"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(sret3, sret0),
        solver.smt.eq(
            sret3,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret0,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv8"),
                    solver.smt.numeral(16),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy8,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv65535"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(sx8, sy8),
        solver.smt.eq(sx8, x),
    ]));
    // round 2
    let sret4 = solver.declare(
        format!("sret4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let sy4 = solver.declare(
        format!("sy4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let sx4 = solver.declare(
        format!("sx4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(sy4, solver.smt.bvashr(sx8, solver.smt.atom("#x0004"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv65535"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(sret4, sret3),
        solver.smt.eq(
            sret4,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret3,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv4"),
                    solver.smt.numeral(16),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv65535"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(sx4, sy4),
        solver.smt.eq(sx4, sx8),
    ]));
    // round 3
    let sret5 = solver.declare(
        format!("sret5_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let sy2 = solver.declare(
        format!("sy2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let sx2 = solver.declare(
        format!("sx2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(sy2, solver.smt.bvashr(sx4, solver.smt.atom("#x0002"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv65535"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(sret5, sret4),
        solver.smt.eq(
            sret5,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv2"),
                    solver.smt.numeral(16),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv65535"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(sx2, sy2),
        solver.smt.eq(sx2, sx4),
    ]));
    // round 4
    let sret6 = solver.declare(
        format!("sret6_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let sy1 = solver.declare(
        format!("sy1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let sx1 = solver.declare(
        format!("sx1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(sy1, solver.smt.bvashr(sx2, solver.smt.atom("#x0001"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv65535"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(sret6, sret5),
        solver.smt.eq(
            sret6,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret5,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(16),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv65535"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(sx1, sy1),
        solver.smt.eq(sx1, sx2),
    ]));
    // last round
    let sret7 = solver.declare(
        format!("sret7_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sx1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv65535"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(sret7, sret6),
        solver.smt.eq(
            sret7,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret6,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(16),
                ]),
            ]),
        ),
    ]));
    let clsret = solver.declare(
        format!("clsret_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.eq(
            sret7,
            solver.smt.list(vec![
                solver.smt.atoms().und,
                solver.smt.atom("bv0"),
                solver.smt.numeral(16),
            ]),
        ),
        solver.smt.eq(clsret, sret7),
        solver.smt.eq(
            clsret,
            solver.smt.list(vec![
                solver.smt.atom("bvsub"),
                sret7,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(16),
                ]),
            ]),
        ),
    ]));
    let cls16ret = solver.declare(
        format!("cls16ret_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("bvsle"),
            solver.smt.list(vec![
                solver.smt.atoms().und,
                solver.smt.atom("bv0"),
                solver.smt.numeral(16),
            ]),
            x,
        ]),
        solver.smt.eq(cls16ret, clzret),
        solver.smt.eq(cls16ret, clsret),
    ]));

    if solver.find_widths {
        let padding = solver.new_fresh_bits(solver.bitwidth - 16);
        solver.smt.concat(padding, cls16ret)
    } else {
        cls16ret
    }
}

pub fn cls8(solver: &mut SolverCtx, x: SExpr, id: u32) -> SExpr {
    let x = solver.smt.extract(7, 0, x);

    // Generated code.
    // total zeros counter
    let zret0 = solver.declare(
        format!("zret0_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    solver.assume(solver.smt.eq(
        zret0,
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("bv0"),
            solver.smt.numeral(8),
        ]),
    ));
    // round 1
    let zret4 = solver.declare(
        format!("zret4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    let zy4 = solver.declare(
        format!("zy4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    let zx4 = solver.declare(
        format!("zx4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(zy4, solver.smt.bvlshr(x, solver.smt.atom("#x04"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(8),
                ]),
            ),
        ]),
        solver.smt.eq(zret4, zret0),
        solver.smt.eq(
            zret4,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret0,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv4"),
                    solver.smt.numeral(8),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(8),
                ]),
            ),
        ]),
        solver.smt.eq(zx4, zy4),
        solver.smt.eq(zx4, x),
    ]));
    // round 2
    let zret5 = solver.declare(
        format!("zret5_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    let zy2 = solver.declare(
        format!("zy2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    let zx2 = solver.declare(
        format!("zx2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(zy2, solver.smt.bvlshr(zx4, solver.smt.atom("#x02"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(8),
                ]),
            ),
        ]),
        solver.smt.eq(zret5, zret4),
        solver.smt.eq(
            zret5,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv2"),
                    solver.smt.numeral(8),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(8),
                ]),
            ),
        ]),
        solver.smt.eq(zx2, zy2),
        solver.smt.eq(zx2, zx4),
    ]));
    // round 3
    let zret6 = solver.declare(
        format!("zret6_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    let zy1 = solver.declare(
        format!("zy1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    let zx1 = solver.declare(
        format!("zx1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(zy1, solver.smt.bvlshr(zx2, solver.smt.atom("#x01"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(8),
                ]),
            ),
        ]),
        solver.smt.eq(zret6, zret5),
        solver.smt.eq(
            zret6,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret5,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(8),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zy1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(8),
                ]),
            ),
        ]),
        solver.smt.eq(zx1, zy1),
        solver.smt.eq(zx1, zx2),
    ]));
    // last round
    let zret7 = solver.declare(
        format!("zret7_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                zx1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(8),
                ]),
            ),
        ]),
        solver.smt.eq(zret7, zret6),
        solver.smt.eq(
            zret7,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                zret6,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(8),
                ]),
            ]),
        ),
    ]));
    let clzret = solver.declare(
        format!("clzret_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.eq(
            zret7,
            solver.smt.list(vec![
                solver.smt.atoms().und,
                solver.smt.atom("bv0"),
                solver.smt.numeral(8),
            ]),
        ),
        solver.smt.eq(clzret, zret7),
        solver.smt.eq(
            clzret,
            solver.smt.list(vec![
                solver.smt.atom("bvsub"),
                zret7,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(8),
                ]),
            ]),
        ),
    ]));
    // total zeros counter
    let sret0 = solver.declare(
        format!("sret0_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    solver.assume(solver.smt.eq(
        sret0,
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("bv0"),
            solver.smt.numeral(8),
        ]),
    ));
    // round 1
    let sret4 = solver.declare(
        format!("sret4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    let sy4 = solver.declare(
        format!("sy4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    let sx4 = solver.declare(
        format!("sx4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(sy4, solver.smt.bvashr(x, solver.smt.atom("#x04"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv255"),
                    solver.smt.numeral(8),
                ]),
            ),
        ]),
        solver.smt.eq(sret4, sret0),
        solver.smt.eq(
            sret4,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret0,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv4"),
                    solver.smt.numeral(8),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv255"),
                    solver.smt.numeral(8),
                ]),
            ),
        ]),
        solver.smt.eq(sx4, sy4),
        solver.smt.eq(sx4, x),
    ]));
    // round 2
    let sret5 = solver.declare(
        format!("sret5_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    let sy2 = solver.declare(
        format!("sy2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    let sx2 = solver.declare(
        format!("sx2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(sy2, solver.smt.bvashr(sx4, solver.smt.atom("#x02"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv255"),
                    solver.smt.numeral(8),
                ]),
            ),
        ]),
        solver.smt.eq(sret5, sret4),
        solver.smt.eq(
            sret5,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv2"),
                    solver.smt.numeral(8),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv255"),
                    solver.smt.numeral(8),
                ]),
            ),
        ]),
        solver.smt.eq(sx2, sy2),
        solver.smt.eq(sx2, sx4),
    ]));
    // round 3
    let sret6 = solver.declare(
        format!("sret6_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    let sy1 = solver.declare(
        format!("sy1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    let sx1 = solver.declare(
        format!("sx1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(sy1, solver.smt.bvashr(sx2, solver.smt.atom("#x01"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv255"),
                    solver.smt.numeral(8),
                ]),
            ),
        ]),
        solver.smt.eq(sret6, sret5),
        solver.smt.eq(
            sret6,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret5,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(8),
                ]),
            ]),
        ),
    ]));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sy1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv255"),
                    solver.smt.numeral(8),
                ]),
            ),
        ]),
        solver.smt.eq(sx1, sy1),
        solver.smt.eq(sx1, sx2),
    ]));
    // last round
    let sret7 = solver.declare(
        format!("sret7_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                sx1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv255"),
                    solver.smt.numeral(8),
                ]),
            ),
        ]),
        solver.smt.eq(sret7, sret6),
        solver.smt.eq(
            sret7,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                sret6,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(8),
                ]),
            ]),
        ),
    ]));
    let clsret = solver.declare(
        format!("clsret_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.eq(
            sret7,
            solver.smt.list(vec![
                solver.smt.atoms().und,
                solver.smt.atom("bv0"),
                solver.smt.numeral(8),
            ]),
        ),
        solver.smt.eq(clsret, sret7),
        solver.smt.eq(
            clsret,
            solver.smt.list(vec![
                solver.smt.atom("bvsub"),
                sret7,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(8),
                ]),
            ]),
        ),
    ]));
    let cls8ret = solver.declare(
        format!("cls8ret_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("bvsle"),
            solver.smt.list(vec![
                solver.smt.atoms().und,
                solver.smt.atom("bv0"),
                solver.smt.numeral(8),
            ]),
            x,
        ]),
        solver.smt.eq(cls8ret, clzret),
        solver.smt.eq(cls8ret, clsret),
    ]));

    if solver.find_widths {
        let padding = solver.new_fresh_bits(solver.bitwidth - 8);
        solver.smt.concat(padding, cls8ret)
    } else {
        cls8ret
    }
}

pub fn cls1(solver: &mut SolverCtx, id: u32) -> SExpr {
    // Generated code.
    let cls1ret = solver.declare(
        format!("cls1ret_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(1),
        ]),
    );
    solver.assume(solver.smt.eq(
        cls1ret,
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("bv0"),
            solver.smt.numeral(1),
        ]),
    ));

    if solver.find_widths {
        let padding = solver.new_fresh_bits(solver.bitwidth - 1);
        solver.smt.concat(padding, cls1ret)
    } else {
        cls1ret
    }
}
