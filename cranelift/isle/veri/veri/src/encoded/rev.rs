use easy_smt::*;

fn declare(smt: &mut Context, name: String, val: SExpr) -> SExpr {
    smt.declare_const(name.clone(), val).unwrap();
    smt.atom(name)
}

pub fn rev64(smt: &mut Context, x: SExpr, id: usize) -> SExpr {
    // Generated code.
    let x1 = declare(
        smt,
        format!("x1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(
        x1,
        smt.bvor(
            smt.bvlshr(x, smt.atom("#x0000000000000020")),
            smt.bvshl(x, smt.atom("#x0000000000000020")),
        ),
    ));
    let x2 = declare(
        smt,
        format!("x2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(
        x2,
        smt.bvor(
            smt.bvlshr(
                smt.bvand(x1, smt.atom("#xffff0000ffff0000")),
                smt.atom("#x0000000000000010"),
            ),
            smt.bvshl(
                smt.bvand(x1, smt.atom("#x0000ffff0000ffff")),
                smt.atom("#x0000000000000010"),
            ),
        ),
    ));
    let x3 = declare(
        smt,
        format!("x3_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(
        x3,
        smt.bvor(
            smt.bvlshr(
                smt.bvand(x2, smt.atom("#xff00ff00ff00ff00")),
                smt.atom("#x0000000000000008"),
            ),
            smt.bvshl(
                smt.bvand(x2, smt.atom("#x00ff00ff00ff00ff")),
                smt.atom("#x0000000000000008"),
            ),
        ),
    ));
    let x4 = declare(
        smt,
        format!("x4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(
        x4,
        smt.bvor(
            smt.bvlshr(
                smt.bvand(x3, smt.atom("#xf0f0f0f0f0f0f0f0")),
                smt.atom("#x0000000000000004"),
            ),
            smt.bvshl(
                smt.bvand(x3, smt.atom("#x0f0f0f0f0f0f0f0f")),
                smt.atom("#x0000000000000004"),
            ),
        ),
    ));
    let x5 = declare(
        smt,
        format!("x5_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(
        x5,
        smt.bvor(
            smt.bvlshr(
                smt.bvand(x4, smt.atom("#xcccccccccccccccc")),
                smt.atom("#x0000000000000002"),
            ),
            smt.bvshl(
                smt.bvand(x4, smt.atom("#x3333333333333333")),
                smt.atom("#x0000000000000002"),
            ),
        ),
    ));
    let rev64ret = declare(
        smt,
        format!("rev64ret_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(
        rev64ret,
        smt.bvor(
            smt.bvlshr(
                smt.bvand(x5, smt.atom("#xaaaaaaaaaaaaaaaa")),
                smt.atom("#x0000000000000001"),
            ),
            smt.bvshl(
                smt.bvand(x5, smt.atom("#x5555555555555555")),
                smt.atom("#x0000000000000001"),
            ),
        ),
    ));

    rev64ret
}

pub fn rev32(smt: &mut Context, x: SExpr, id: usize) -> SExpr {
    let x = smt.extract(31, 0, x);

    // Generated code.
    let x1 = declare(
        smt,
        format!("x1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(
        x1,
        smt.bvor(
            smt.bvlshr(x, smt.atom("#x00000010")),
            smt.bvshl(x, smt.atom("#x00000010")),
        ),
    ));
    let x2 = declare(
        smt,
        format!("x2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(
        x2,
        smt.bvor(
            smt.bvlshr(
                smt.bvand(x1, smt.atom("#xff00ff00")),
                smt.atom("#x00000008"),
            ),
            smt.bvshl(
                smt.bvand(x1, smt.atom("#x00ff00ff")),
                smt.atom("#x00000008"),
            ),
        ),
    ));
    let x3 = declare(
        smt,
        format!("x3_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(
        x3,
        smt.bvor(
            smt.bvlshr(
                smt.bvand(x2, smt.atom("#xf0f0f0f0")),
                smt.atom("#x00000004"),
            ),
            smt.bvshl(
                smt.bvand(x2, smt.atom("#x0f0f0f0f")),
                smt.atom("#x00000004"),
            ),
        ),
    ));
    let x4 = declare(
        smt,
        format!("x4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(
        x4,
        smt.bvor(
            smt.bvlshr(
                smt.bvand(x3, smt.atom("#xcccccccc")),
                smt.atom("#x00000002"),
            ),
            smt.bvshl(
                smt.bvand(x3, smt.atom("#x33333333")),
                smt.atom("#x00000002"),
            ),
        ),
    ));
    let rev32ret = declare(
        smt,
        format!("rev32ret_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(
        rev32ret,
        smt.bvor(
            smt.bvlshr(
                smt.bvand(x4, smt.atom("#xaaaaaaaa")),
                smt.atom("#x00000001"),
            ),
            smt.bvshl(
                smt.bvand(x4, smt.atom("#x55555555")),
                smt.atom("#x00000001"),
            ),
        ),
    ));

    rev32ret
}

pub fn rev16(smt: &mut Context, x: SExpr, id: usize) -> SExpr {
    let x = smt.extract(15, 0, x);

    // Generated code.
    let x1 = declare(
        smt,
        format!("x1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.eq(
        x1,
        smt.bvor(
            smt.bvlshr(x, smt.atom("#x0008")),
            smt.bvshl(x, smt.atom("#x0008")),
        ),
    ));
    let x2 = declare(
        smt,
        format!("x2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.eq(
        x2,
        smt.bvor(
            smt.bvlshr(smt.bvand(x1, smt.atom("#xf0f0")), smt.atom("#x0004")),
            smt.bvshl(smt.bvand(x1, smt.atom("#x0f0f")), smt.atom("#x0004")),
        ),
    ));
    let x3 = declare(
        smt,
        format!("x3_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.eq(
        x3,
        smt.bvor(
            smt.bvlshr(smt.bvand(x2, smt.atom("#xcccc")), smt.atom("#x0002")),
            smt.bvshl(smt.bvand(x2, smt.atom("#x3333")), smt.atom("#x0002")),
        ),
    ));
    let rev16ret = declare(
        smt,
        format!("rev16ret_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.eq(
        rev16ret,
        smt.bvor(
            smt.bvlshr(smt.bvand(x3, smt.atom("#xaaaa")), smt.atom("#x0001")),
            smt.bvshl(smt.bvand(x3, smt.atom("#x5555")), smt.atom("#x0001")),
        ),
    ));

    // let padding = smt.new_fresh_bits(smt.bitwidth - 16);
    // smt.concat(padding, rev16ret)
    rev16ret
}

pub fn rev8(smt: &mut Context, x: SExpr, id: usize) -> SExpr {
    let x = smt.extract(7, 0, x);

    // Generated code.
    let x1 = declare(
        smt,
        format!("x1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let _ = smt.assert(smt.eq(
        x1,
        smt.bvor(
            smt.bvlshr(x, smt.atom("#x04")),
            smt.bvshl(x, smt.atom("#x04")),
        ),
    ));
    let x2 = declare(
        smt,
        format!("x2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let _ = smt.assert(smt.eq(
        x2,
        smt.bvor(
            smt.bvlshr(smt.bvand(x1, smt.atom("#xcc")), smt.atom("#x02")),
            smt.bvshl(smt.bvand(x1, smt.atom("#x33")), smt.atom("#x02")),
        ),
    ));
    let rev8ret = declare(
        smt,
        format!("rev8ret_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let _ = smt.assert(smt.eq(
        rev8ret,
        smt.bvor(
            smt.bvlshr(smt.bvand(x2, smt.atom("#xaa")), smt.atom("#x01")),
            smt.bvshl(smt.bvand(x2, smt.atom("#x55")), smt.atom("#x01")),
        ),
    ));

    // let padding = smt.new_fresh_bits(smt.bitwidth - 8);
    // smt.concat(padding, rev8ret)
    rev8ret
}

pub fn rev1(smt: &mut Context, x: SExpr, id: usize) -> SExpr {
    let x = smt.extract(0, 0, x);

    // Generated code.
    let rev1ret = declare(
        smt,
        format!("rev1ret_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(1)]),
    );
    let _ = smt.assert(smt.eq(rev1ret, x));

    // let padding = smt.new_fresh_bits(smt.bitwidth - 1);
    // smt.concat(padding, rev1ret)
    rev1ret
}
