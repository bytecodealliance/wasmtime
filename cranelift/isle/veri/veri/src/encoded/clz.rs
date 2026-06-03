// Adapted from https://stackoverflow.com/questions/23856596/how-to-count-leading-zeros-in-a-32-bit-unsigned-integer
use easy_smt::*;

fn declare(smt: &mut Context, name: String, val: SExpr) -> SExpr {
    smt.declare_const(name.clone(), val).unwrap();
    smt.atom(name)
}

pub fn clz64(smt: &mut Context, x: SExpr, id: usize) -> SExpr {
    // Generated code.
    // total zeros counter
    let ret0 = declare(
        smt,
        format!("ret0_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(
        ret0,
        smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
    ));
    // round 1
    let ret1 = declare(
        smt,
        format!("ret1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let y32 = declare(
        smt,
        format!("y32_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let x32 = declare(
        smt,
        format!("x32_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(y32, smt.bvlshr(x, smt.atom("#x0000000000000020"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y32,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(ret1, ret0),
        smt.eq(
            ret1,
            smt.list(vec![
                smt.atom("bvadd"),
                ret0,
                smt.list(vec![smt.atoms().und, smt.atom("bv32"), smt.numeral(64)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y32,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(x32, y32),
        smt.eq(x32, x),
    ]));
    // round 2
    let ret2 = declare(
        smt,
        format!("ret2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let y16 = declare(
        smt,
        format!("y16_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let x16 = declare(
        smt,
        format!("x16_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(y16, smt.bvlshr(x32, smt.atom("#x0000000000000010"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y16,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(ret2, ret1),
        smt.eq(
            ret2,
            smt.list(vec![
                smt.atom("bvadd"),
                ret1,
                smt.list(vec![smt.atoms().und, smt.atom("bv16"), smt.numeral(64)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y16,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(x16, y16),
        smt.eq(x16, x32),
    ]));
    // round 3
    let ret3 = declare(
        smt,
        format!("ret3_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let y8 = declare(
        smt,
        format!("y8_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let x8 = declare(
        smt,
        format!("x8_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(y8, smt.bvlshr(x16, smt.atom("#x0000000000000008"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y8,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(ret3, ret2),
        smt.eq(
            ret3,
            smt.list(vec![
                smt.atom("bvadd"),
                ret2,
                smt.list(vec![smt.atoms().und, smt.atom("bv8"), smt.numeral(64)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y8,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(x8, y8),
        smt.eq(x8, x16),
    ]));
    // round 4
    let ret4 = declare(
        smt,
        format!("ret4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let y4 = declare(
        smt,
        format!("y4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let x4 = declare(
        smt,
        format!("x4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(y4, smt.bvlshr(x8, smt.atom("#x0000000000000004"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y4,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(ret4, ret3),
        smt.eq(
            ret4,
            smt.list(vec![
                smt.atom("bvadd"),
                ret3,
                smt.list(vec![smt.atoms().und, smt.atom("bv4"), smt.numeral(64)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y4,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(x4, y4),
        smt.eq(x4, x8),
    ]));
    // round 5
    let ret5 = declare(
        smt,
        format!("ret5_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let y2 = declare(
        smt,
        format!("y2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let x2 = declare(
        smt,
        format!("x2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(y2, smt.bvlshr(x4, smt.atom("#x0000000000000002"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y2,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(ret5, ret4),
        smt.eq(
            ret5,
            smt.list(vec![
                smt.atom("bvadd"),
                ret4,
                smt.list(vec![smt.atoms().und, smt.atom("bv2"), smt.numeral(64)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y2,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(x2, y2),
        smt.eq(x2, x4),
    ]));
    // round 6
    let ret6 = declare(
        smt,
        format!("ret6_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let y1 = declare(
        smt,
        format!("y1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let x1 = declare(
        smt,
        format!("x1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(y1, smt.bvlshr(x2, smt.atom("#x0000000000000001"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(ret6, ret5),
        smt.eq(
            ret6,
            smt.list(vec![
                smt.atom("bvadd"),
                ret5,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(64)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(x1, y1),
        smt.eq(x1, x2),
    ]));

    // last round
    let ret7 = declare(
        smt,
        format!("ret7_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                x1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(ret7, ret6),
        smt.eq(
            ret7,
            smt.list(vec![
                smt.atom("bvadd"),
                ret6,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(64)]),
            ]),
        ),
    ]));

    ret7
}

pub fn clz32(smt: &mut Context, x: SExpr, id: usize) -> SExpr {
    let x = smt.extract(31, 0, x);

    // Generated code.
    // total zeros counter
    let ret0 = declare(
        smt,
        format!("ret0_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(
        ret0,
        smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
    ));
    // round 1
    let ret1 = declare(
        smt,
        format!("ret1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let y16 = declare(
        smt,
        format!("y16_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let x16 = declare(
        smt,
        format!("x16_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(y16, smt.bvlshr(x, smt.atom("#x00000010"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y16,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(ret1, ret0),
        smt.eq(
            ret1,
            smt.list(vec![
                smt.atom("bvadd"),
                ret0,
                smt.list(vec![smt.atoms().und, smt.atom("bv16"), smt.numeral(32)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y16,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(x16, y16),
        smt.eq(x16, x),
    ]));
    // round 2
    let ret2 = declare(
        smt,
        format!("ret2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let y8 = declare(
        smt,
        format!("y8_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let x8 = declare(
        smt,
        format!("x8_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(y8, smt.bvlshr(x16, smt.atom("#x00000008"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y8,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(ret2, ret1),
        smt.eq(
            ret2,
            smt.list(vec![
                smt.atom("bvadd"),
                ret1,
                smt.list(vec![smt.atoms().und, smt.atom("bv8"), smt.numeral(32)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y8,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(x8, y8),
        smt.eq(x8, x16),
    ]));
    // round 3
    let ret3 = declare(
        smt,
        format!("ret3_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let y4 = declare(
        smt,
        format!("y4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let x4 = declare(
        smt,
        format!("x4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(y4, smt.bvlshr(x8, smt.atom("#x00000004"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y4,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(ret3, ret2),
        smt.eq(
            ret3,
            smt.list(vec![
                smt.atom("bvadd"),
                ret2,
                smt.list(vec![smt.atoms().und, smt.atom("bv4"), smt.numeral(32)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y4,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(x4, y4),
        smt.eq(x4, x8),
    ]));
    // round 4
    let ret4 = declare(
        smt,
        format!("ret4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let y2 = declare(
        smt,
        format!("y2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let x2 = declare(
        smt,
        format!("x2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(y2, smt.bvlshr(x4, smt.atom("#x00000002"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y2,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(ret4, ret3),
        smt.eq(
            ret4,
            smt.list(vec![
                smt.atom("bvadd"),
                ret3,
                smt.list(vec![smt.atoms().und, smt.atom("bv2"), smt.numeral(32)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y2,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(x2, y2),
        smt.eq(x2, x4),
    ]));
    // round 5
    let ret5 = declare(
        smt,
        format!("ret5_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let y1 = declare(
        smt,
        format!("y1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let x1 = declare(
        smt,
        format!("x1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(y1, smt.bvlshr(x2, smt.atom("#x00000001"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(ret5, ret4),
        smt.eq(
            ret5,
            smt.list(vec![
                smt.atom("bvadd"),
                ret4,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(32)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(x1, y1),
        smt.eq(x1, x2),
    ]));

    // last round
    let ret6 = declare(
        smt,
        format!("ret6_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                x1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(ret6, ret5),
        smt.eq(
            ret6,
            smt.list(vec![
                smt.atom("bvadd"),
                ret5,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(32)]),
            ]),
        ),
    ]));
    ret6
}

pub fn clz16(smt: &mut Context, x: SExpr, id: usize) -> SExpr {
    let x = smt.extract(15, 0, x);

    // Generated code.
    // total zeros counter
    let ret1 = declare(
        smt,
        format!("ret1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.eq(
        ret1,
        smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
    ));
    // round 1
    let ret2 = declare(
        smt,
        format!("ret2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let y8 = declare(
        smt,
        format!("y8_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let x8 = declare(
        smt,
        format!("x8_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.eq(y8, smt.bvlshr(x, smt.atom("#x0008"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y8,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(ret2, ret1),
        smt.eq(
            ret2,
            smt.list(vec![
                smt.atom("bvadd"),
                ret1,
                smt.list(vec![smt.atoms().und, smt.atom("bv8"), smt.numeral(16)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y8,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(x8, y8),
        smt.eq(x8, x),
    ]));
    // round 2
    let ret3 = declare(
        smt,
        format!("ret3_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let y4 = declare(
        smt,
        format!("y4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let x4 = declare(
        smt,
        format!("x4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.eq(y4, smt.bvlshr(x8, smt.atom("#x0004"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y4,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(ret3, ret2),
        smt.eq(
            ret3,
            smt.list(vec![
                smt.atom("bvadd"),
                ret2,
                smt.list(vec![smt.atoms().und, smt.atom("bv4"), smt.numeral(16)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y4,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(x4, y4),
        smt.eq(x4, x8),
    ]));
    // round 3
    let ret4 = declare(
        smt,
        format!("ret4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let y2 = declare(
        smt,
        format!("y2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let x2 = declare(
        smt,
        format!("x2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.eq(y2, smt.bvlshr(x4, smt.atom("#x0002"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y2,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(ret4, ret3),
        smt.eq(
            ret4,
            smt.list(vec![
                smt.atom("bvadd"),
                ret3,
                smt.list(vec![smt.atoms().und, smt.atom("bv2"), smt.numeral(16)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y2,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(x2, y2),
        smt.eq(x2, x4),
    ]));
    // round 4
    let ret5 = declare(
        smt,
        format!("ret5_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let y1 = declare(
        smt,
        format!("y1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let x1 = declare(
        smt,
        format!("x1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.eq(y1, smt.bvlshr(x2, smt.atom("#x0001"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(ret5, ret4),
        smt.eq(
            ret5,
            smt.list(vec![
                smt.atom("bvadd"),
                ret4,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(16)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(x1, y1),
        smt.eq(x1, x2),
    ]));

    // last round
    let ret6 = declare(
        smt,
        format!("ret6_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                x1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(ret6, ret5),
        smt.eq(
            ret6,
            smt.list(vec![
                smt.atom("bvadd"),
                ret5,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(16)]),
            ]),
        ),
    ]));
    ret6
}

pub fn clz8(smt: &mut Context, x: SExpr, id: usize) -> SExpr {
    let x = smt.extract(7, 0, x);

    // Generated code.
    // total zeros counter
    let ret0 = declare(
        smt,
        format!("ret0_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let _ = smt.assert(smt.eq(
        ret0,
        smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(8)]),
    ));
    // round 1
    let ret3 = declare(
        smt,
        format!("ret3_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let y4 = declare(
        smt,
        format!("y4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let x4 = declare(
        smt,
        format!("x4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let _ = smt.assert(smt.eq(y4, smt.bvlshr(x, smt.atom("#x04"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y4,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(8)]),
            ),
        ]),
        smt.eq(ret3, ret0),
        smt.eq(
            ret3,
            smt.list(vec![
                smt.atom("bvadd"),
                ret0,
                smt.list(vec![smt.atoms().und, smt.atom("bv4"), smt.numeral(8)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y4,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(8)]),
            ),
        ]),
        smt.eq(x4, y4),
        smt.eq(x4, x),
    ]));
    // round 2
    let ret4 = declare(
        smt,
        format!("ret4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let y2 = declare(
        smt,
        format!("y2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let x2 = declare(
        smt,
        format!("x2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let _ = smt.assert(smt.eq(y2, smt.bvlshr(x4, smt.atom("#x02"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y2,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(8)]),
            ),
        ]),
        smt.eq(ret4, ret3),
        smt.eq(
            ret4,
            smt.list(vec![
                smt.atom("bvadd"),
                ret3,
                smt.list(vec![smt.atoms().und, smt.atom("bv2"), smt.numeral(8)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y2,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(8)]),
            ),
        ]),
        smt.eq(x2, y2),
        smt.eq(x2, x4),
    ]));
    // round 3
    let ret5 = declare(
        smt,
        format!("ret5_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let y1 = declare(
        smt,
        format!("y1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let x1 = declare(
        smt,
        format!("x1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let _ = smt.assert(smt.eq(y1, smt.bvlshr(x2, smt.atom("#x01"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(8)]),
            ),
        ]),
        smt.eq(ret5, ret4),
        smt.eq(
            ret5,
            smt.list(vec![
                smt.atom("bvadd"),
                ret4,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(8)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                y1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(8)]),
            ),
        ]),
        smt.eq(x1, y1),
        smt.eq(x1, x2),
    ]));
    // last round
    let ret6 = declare(
        smt,
        format!("ret6_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                x1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(8)]),
            ),
        ]),
        smt.eq(ret6, ret5),
        smt.eq(
            ret6,
            smt.list(vec![
                smt.atom("bvadd"),
                ret5,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(8)]),
            ]),
        ),
    ]));

    ret6
}

pub fn clz1(smt: &mut Context, x: SExpr, id: usize) -> SExpr {
    let x = smt.extract(0, 0, x);

    // Generated code.
    let clz1ret = declare(
        smt,
        format!("clz1ret_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(1)]),
    );
    let _ = smt.assert(smt.eq(clz1ret, smt.list(vec![smt.atom("bvnot"), x])));

    clz1ret
}
