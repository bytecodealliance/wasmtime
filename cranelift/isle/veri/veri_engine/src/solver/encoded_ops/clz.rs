use crate::solver::SolverCtx;
use easy_smt::SExpr;

// Adapted from https://stackoverflow.com/questions/23856596/how-to-count-leading-zeros-in-a-32-bit-unsigned-integer

pub fn clz64(solver: &mut SolverCtx, x: SExpr, id: u32) -> SExpr {
    // Generated code.
    // total zeros counter
    let ret0 = solver.declare(
        format!("ret0_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(solver.smt.eq(
        ret0,
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("bv0"),
            solver.smt.numeral(64),
        ]),
    ));
    // round 1
    let ret1 = solver.declare(
        format!("ret1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let y32 = solver.declare(
        format!("y32_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let x32 = solver.declare(
        format!("x32_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(solver.smt.eq(
        y32,
        solver.smt.bvlshr(x, solver.smt.atom("#x0000000000000020")),
    ));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                y32,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(ret1, ret0),
        solver.smt.eq(
            ret1,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret0,
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
                y32,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(x32, y32),
        solver.smt.eq(x32, x),
    ]));
    // round 2
    let ret2 = solver.declare(
        format!("ret2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let y16 = solver.declare(
        format!("y16_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let x16 = solver.declare(
        format!("x16_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(
        solver.smt.eq(
            y16,
            solver
                .smt
                .bvlshr(x32, solver.smt.atom("#x0000000000000010")),
        ),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                y16,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(ret2, ret1),
        solver.smt.eq(
            ret2,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret1,
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
                y16,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(x16, y16),
        solver.smt.eq(x16, x32),
    ]));
    // round 3
    let ret3 = solver.declare(
        format!("ret3_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let y8 = solver.declare(
        format!("y8_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let x8 = solver.declare(
        format!("x8_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(
        solver.smt.eq(
            y8,
            solver
                .smt
                .bvlshr(x16, solver.smt.atom("#x0000000000000008")),
        ),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                y8,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(ret3, ret2),
        solver.smt.eq(
            ret3,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret2,
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
                y8,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(x8, y8),
        solver.smt.eq(x8, x16),
    ]));
    // round 4
    let ret4 = solver.declare(
        format!("ret4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let y4 = solver.declare(
        format!("y4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let x4 = solver.declare(
        format!("x4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(solver.smt.eq(
        y4,
        solver.smt.bvlshr(x8, solver.smt.atom("#x0000000000000004")),
    ));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                y4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(ret4, ret3),
        solver.smt.eq(
            ret4,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret3,
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
                y4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(x4, y4),
        solver.smt.eq(x4, x8),
    ]));
    // round 5
    let ret5 = solver.declare(
        format!("ret5_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let y2 = solver.declare(
        format!("y2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let x2 = solver.declare(
        format!("x2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(solver.smt.eq(
        y2,
        solver.smt.bvlshr(x4, solver.smt.atom("#x0000000000000002")),
    ));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                y2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(ret5, ret4),
        solver.smt.eq(
            ret5,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret4,
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
                y2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(x2, y2),
        solver.smt.eq(x2, x4),
    ]));
    // round 6
    let ret6 = solver.declare(
        format!("ret6_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let y1 = solver.declare(
        format!("y1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    let x1 = solver.declare(
        format!("x1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(64),
        ]),
    );
    solver.assume(solver.smt.eq(
        y1,
        solver.smt.bvlshr(x2, solver.smt.atom("#x0000000000000001")),
    ));
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                y1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(ret6, ret5),
        solver.smt.eq(
            ret6,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret5,
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
                y1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(x1, y1),
        solver.smt.eq(x1, x2),
    ]));

    // last round
    let ret7 = solver.declare(
        format!("ret7_{id}", id = id),
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
                x1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(64),
                ]),
            ),
        ]),
        solver.smt.eq(ret7, ret6),
        solver.smt.eq(
            ret7,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret6,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(64),
                ]),
            ]),
        ),
    ]));

    ret7
}

pub fn clz32(solver: &mut SolverCtx, x: SExpr, id: u32) -> SExpr {
    let x = solver.smt.extract(31, 0, x);

    // Generated code.
    // total zeros counter
    let ret0 = solver.declare(
        format!("ret0_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(solver.smt.eq(
        ret0,
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("bv0"),
            solver.smt.numeral(32),
        ]),
    ));
    // round 1
    let ret1 = solver.declare(
        format!("ret1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let y16 = solver.declare(
        format!("y16_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let x16 = solver.declare(
        format!("x16_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(y16, solver.smt.bvlshr(x, solver.smt.atom("#x00000010"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                y16,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(ret1, ret0),
        solver.smt.eq(
            ret1,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret0,
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
                y16,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(x16, y16),
        solver.smt.eq(x16, x),
    ]));
    // round 2
    let ret2 = solver.declare(
        format!("ret2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let y8 = solver.declare(
        format!("y8_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let x8 = solver.declare(
        format!("x8_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(y8, solver.smt.bvlshr(x16, solver.smt.atom("#x00000008"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                y8,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(ret2, ret1),
        solver.smt.eq(
            ret2,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret1,
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
                y8,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(x8, y8),
        solver.smt.eq(x8, x16),
    ]));
    // round 3
    let ret3 = solver.declare(
        format!("ret3_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let y4 = solver.declare(
        format!("y4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let x4 = solver.declare(
        format!("x4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(y4, solver.smt.bvlshr(x8, solver.smt.atom("#x00000004"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                y4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(ret3, ret2),
        solver.smt.eq(
            ret3,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret2,
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
                y4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(x4, y4),
        solver.smt.eq(x4, x8),
    ]));
    // round 4
    let ret4 = solver.declare(
        format!("ret4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let y2 = solver.declare(
        format!("y2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let x2 = solver.declare(
        format!("x2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(y2, solver.smt.bvlshr(x4, solver.smt.atom("#x00000002"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                y2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(ret4, ret3),
        solver.smt.eq(
            ret4,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret3,
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
                y2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(x2, y2),
        solver.smt.eq(x2, x4),
    ]));
    // round 5
    let ret5 = solver.declare(
        format!("ret5_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let y1 = solver.declare(
        format!("y1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    let x1 = solver.declare(
        format!("x1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(32),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(y1, solver.smt.bvlshr(x2, solver.smt.atom("#x00000001"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                y1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(ret5, ret4),
        solver.smt.eq(
            ret5,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret4,
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
                y1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(x1, y1),
        solver.smt.eq(x1, x2),
    ]));

    // last round
    let ret6 = solver.declare(
        format!("ret6_{id}", id = id),
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
                x1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(32),
                ]),
            ),
        ]),
        solver.smt.eq(ret6, ret5),
        solver.smt.eq(
            ret6,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret5,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(32),
                ]),
            ]),
        ),
    ]));

    if solver.find_widths {
        let padding = solver.new_fresh_bits(solver.bitwidth - 32);
        solver.smt.concat(padding, ret6)
    } else {
        ret6
    }
}

pub fn clz16(solver: &mut SolverCtx, x: SExpr, id: u32) -> SExpr {
    let x = solver.smt.extract(15, 0, x);

    // Generated code.
    // total zeros counter
    let ret1 = solver.declare(
        format!("ret1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    solver.assume(solver.smt.eq(
        ret1,
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("bv0"),
            solver.smt.numeral(16),
        ]),
    ));
    // round 1
    let ret2 = solver.declare(
        format!("ret2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let y8 = solver.declare(
        format!("y8_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let x8 = solver.declare(
        format!("x8_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(y8, solver.smt.bvlshr(x, solver.smt.atom("#x0008"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                y8,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(ret2, ret1),
        solver.smt.eq(
            ret2,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret1,
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
                y8,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(x8, y8),
        solver.smt.eq(x8, x),
    ]));
    // round 2
    let ret3 = solver.declare(
        format!("ret3_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let y4 = solver.declare(
        format!("y4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let x4 = solver.declare(
        format!("x4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(y4, solver.smt.bvlshr(x8, solver.smt.atom("#x0004"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                y4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(ret3, ret2),
        solver.smt.eq(
            ret3,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret2,
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
                y4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(x4, y4),
        solver.smt.eq(x4, x8),
    ]));
    // round 3
    let ret4 = solver.declare(
        format!("ret4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let y2 = solver.declare(
        format!("y2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let x2 = solver.declare(
        format!("x2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(y2, solver.smt.bvlshr(x4, solver.smt.atom("#x0002"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                y2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(ret4, ret3),
        solver.smt.eq(
            ret4,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret3,
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
                y2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(x2, y2),
        solver.smt.eq(x2, x4),
    ]));
    // round 4
    let ret5 = solver.declare(
        format!("ret5_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let y1 = solver.declare(
        format!("y1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    let x1 = solver.declare(
        format!("x1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(16),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(y1, solver.smt.bvlshr(x2, solver.smt.atom("#x0001"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                y1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(ret5, ret4),
        solver.smt.eq(
            ret5,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret4,
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
                y1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(x1, y1),
        solver.smt.eq(x1, x2),
    ]));

    // last round
    let ret6 = solver.declare(
        format!("ret6_{id}", id = id),
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
                x1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(16),
                ]),
            ),
        ]),
        solver.smt.eq(ret6, ret5),
        solver.smt.eq(
            ret6,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret5,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(16),
                ]),
            ]),
        ),
    ]));

    if solver.find_widths {
        let padding = solver.new_fresh_bits(solver.bitwidth - 16);
        solver.smt.concat(padding, ret6)
    } else {
        ret6
    }
}

pub fn clz8(solver: &mut SolverCtx, x: SExpr, id: u32) -> SExpr {
    let x = solver.smt.extract(7, 0, x);

    // Generated code.
    // total zeros counter
    let ret0 = solver.declare(
        format!("ret0_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    solver.assume(solver.smt.eq(
        ret0,
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("bv0"),
            solver.smt.numeral(8),
        ]),
    ));
    // round 1
    let ret3 = solver.declare(
        format!("ret3_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    let y4 = solver.declare(
        format!("y4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    let x4 = solver.declare(
        format!("x4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(y4, solver.smt.bvlshr(x, solver.smt.atom("#x04"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                y4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(8),
                ]),
            ),
        ]),
        solver.smt.eq(ret3, ret0),
        solver.smt.eq(
            ret3,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret0,
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
                y4,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(8),
                ]),
            ),
        ]),
        solver.smt.eq(x4, y4),
        solver.smt.eq(x4, x),
    ]));
    // round 2
    let ret4 = solver.declare(
        format!("ret4_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    let y2 = solver.declare(
        format!("y2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    let x2 = solver.declare(
        format!("x2_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(y2, solver.smt.bvlshr(x4, solver.smt.atom("#x02"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                y2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(8),
                ]),
            ),
        ]),
        solver.smt.eq(ret4, ret3),
        solver.smt.eq(
            ret4,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret3,
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
                y2,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(8),
                ]),
            ),
        ]),
        solver.smt.eq(x2, y2),
        solver.smt.eq(x2, x4),
    ]));
    // round 3
    let ret5 = solver.declare(
        format!("ret5_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    let y1 = solver.declare(
        format!("y1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    let x1 = solver.declare(
        format!("x1_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(8),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(y1, solver.smt.bvlshr(x2, solver.smt.atom("#x01"))),
    );
    solver.assume(solver.smt.list(vec![
        solver.smt.atom("ite"),
        solver.smt.list(vec![
            solver.smt.atom("not"),
            solver.smt.eq(
                y1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(8),
                ]),
            ),
        ]),
        solver.smt.eq(ret5, ret4),
        solver.smt.eq(
            ret5,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret4,
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
                y1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(8),
                ]),
            ),
        ]),
        solver.smt.eq(x1, y1),
        solver.smt.eq(x1, x2),
    ]));
    // last round
    let ret6 = solver.declare(
        format!("ret6_{id}", id = id),
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
                x1,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv0"),
                    solver.smt.numeral(8),
                ]),
            ),
        ]),
        solver.smt.eq(ret6, ret5),
        solver.smt.eq(
            ret6,
            solver.smt.list(vec![
                solver.smt.atom("bvadd"),
                ret5,
                solver.smt.list(vec![
                    solver.smt.atoms().und,
                    solver.smt.atom("bv1"),
                    solver.smt.numeral(8),
                ]),
            ]),
        ),
    ]));

    if solver.find_widths {
        let padding = solver.new_fresh_bits(solver.bitwidth - 8);
        solver.smt.concat(padding, ret6)
    } else {
        ret6
    }
}

pub fn clz1(solver: &mut SolverCtx, x: SExpr, id: u32) -> SExpr {
    let x = solver.smt.extract(0, 0, x);

    // Generated code.
    let clz1ret = solver.declare(
        format!("clz1ret_{id}", id = id),
        solver.smt.list(vec![
            solver.smt.atoms().und,
            solver.smt.atom("BitVec"),
            solver.smt.numeral(1),
        ]),
    );
    solver.assume(
        solver
            .smt
            .eq(clz1ret, solver.smt.list(vec![solver.smt.atom("bvnot"), x])),
    );

    if solver.find_widths {
        let padding = solver.new_fresh_bits(solver.bitwidth - 1);
        solver.smt.concat(padding, clz1ret)
    } else {
        clz1ret
    }
}
