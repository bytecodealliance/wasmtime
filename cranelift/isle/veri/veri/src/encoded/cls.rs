// Adapted from https://stackoverflow.com/questions/23856596/how-to-count-leading-zeros-in-a-32-bit-unsigned-integer
use easy_smt::*;

fn declare(smt: &mut Context, name: String, val: SExpr) -> SExpr {
    smt.declare_const(name.clone(), val).unwrap();
    smt.atom(name)
}

pub fn cls64(smt: &mut Context, x: SExpr, id: usize) -> SExpr {
    // Generated code.
    // total zeros counter
    let zret0 = declare(
        smt,
        format!("zret0_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(
        zret0,
        smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
    ));
    // round 1
    let zret1 = declare(
        smt,
        format!("zret1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let zy32 = declare(
        smt,
        format!("zy32_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let zx32 = declare(
        smt,
        format!("zx32_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(zy32, smt.bvlshr(x, smt.atom("#x0000000000000020"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy32,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(zret1, zret0),
        smt.eq(
            zret1,
            smt.list(vec![
                smt.atom("bvadd"),
                zret0,
                smt.list(vec![smt.atoms().und, smt.atom("bv32"), smt.numeral(64)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy32,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(zx32, zy32),
        smt.eq(zx32, x),
    ]));
    // round 2
    let zret2 = declare(
        smt,
        format!("zret2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let zy16 = declare(
        smt,
        format!("zy16_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let zx16 = declare(
        smt,
        format!("zx16_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(zy16, smt.bvlshr(zx32, smt.atom("#x0000000000000010"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy16,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(zret2, zret1),
        smt.eq(
            zret2,
            smt.list(vec![
                smt.atom("bvadd"),
                zret1,
                smt.list(vec![smt.atoms().und, smt.atom("bv16"), smt.numeral(64)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy16,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(zx16, zy16),
        smt.eq(zx16, zx32),
    ]));
    // round 3
    let zret3 = declare(
        smt,
        format!("zret3_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let zy8 = declare(
        smt,
        format!("zy8_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let zx8 = declare(
        smt,
        format!("zx8_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(zy8, smt.bvlshr(zx16, smt.atom("#x0000000000000008"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy8,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(zret3, zret2),
        smt.eq(
            zret3,
            smt.list(vec![
                smt.atom("bvadd"),
                zret2,
                smt.list(vec![smt.atoms().und, smt.atom("bv8"), smt.numeral(64)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy8,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(zx8, zy8),
        smt.eq(zx8, zx16),
    ]));
    // round 4
    let zret4 = declare(
        smt,
        format!("zret4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let zy4 = declare(
        smt,
        format!("zy4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let zx4 = declare(
        smt,
        format!("zx4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(zy4, smt.bvlshr(zx8, smt.atom("#x0000000000000004"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy4,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(zret4, zret3),
        smt.eq(
            zret4,
            smt.list(vec![
                smt.atom("bvadd"),
                zret3,
                smt.list(vec![smt.atoms().und, smt.atom("bv4"), smt.numeral(64)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy4,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(zx4, zy4),
        smt.eq(zx4, zx8),
    ]));
    // round 5
    let zret5 = declare(
        smt,
        format!("zret5_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let zy2 = declare(
        smt,
        format!("zy2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let zx2 = declare(
        smt,
        format!("zx2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(zy2, smt.bvlshr(zx4, smt.atom("#x0000000000000002"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy2,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(zret5, zret4),
        smt.eq(
            zret5,
            smt.list(vec![
                smt.atom("bvadd"),
                zret4,
                smt.list(vec![smt.atoms().und, smt.atom("bv2"), smt.numeral(64)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy2,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(zx2, zy2),
        smt.eq(zx2, zx4),
    ]));
    // round 6
    let zret6 = declare(
        smt,
        format!("zret6_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let zy1 = declare(
        smt,
        format!("zy1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let zx1 = declare(
        smt,
        format!("zx1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(zy1, smt.bvlshr(zx2, smt.atom("#x0000000000000001"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(zret6, zret5),
        smt.eq(
            zret6,
            smt.list(vec![
                smt.atom("bvadd"),
                zret5,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(64)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(zx1, zy1),
        smt.eq(zx1, zx2),
    ]));
    // last round
    let zret7 = declare(
        smt,
        format!("zret7_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zx1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            ),
        ]),
        smt.eq(zret7, zret6),
        smt.eq(
            zret7,
            smt.list(vec![
                smt.atom("bvadd"),
                zret6,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(64)]),
            ]),
        ),
    ]));
    let clzret = declare(
        smt,
        format!("clzret_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.eq(
            zret7,
            smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
        ),
        smt.eq(clzret, zret7),
        smt.eq(
            clzret,
            smt.list(vec![
                smt.atom("bvsub"),
                zret7,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(64)]),
            ]),
        ),
    ]));
    // total zeros counter
    let sret0 = declare(
        smt,
        format!("sret0_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(
        sret0,
        smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
    ));
    // round 1
    let sret1 = declare(
        smt,
        format!("sret1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let sy32 = declare(
        smt,
        format!("sy32_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let sx32 = declare(
        smt,
        format!("sx32_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(sy32, smt.bvashr(x, smt.atom("#x0000000000000020"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy32,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv18446744073709551615"),
                    smt.numeral(64),
                ]),
            ),
        ]),
        smt.eq(sret1, sret0),
        smt.eq(
            sret1,
            smt.list(vec![
                smt.atom("bvadd"),
                sret0,
                smt.list(vec![smt.atoms().und, smt.atom("bv32"), smt.numeral(64)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy32,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv18446744073709551615"),
                    smt.numeral(64),
                ]),
            ),
        ]),
        smt.eq(sx32, sy32),
        smt.eq(sx32, x),
    ]));
    // round 2
    let sret2 = declare(
        smt,
        format!("sret2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let sy16 = declare(
        smt,
        format!("sy16_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let sx16 = declare(
        smt,
        format!("sx16_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(sy16, smt.bvashr(sx32, smt.atom("#x0000000000000010"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy16,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv18446744073709551615"),
                    smt.numeral(64),
                ]),
            ),
        ]),
        smt.eq(sret2, sret1),
        smt.eq(
            sret2,
            smt.list(vec![
                smt.atom("bvadd"),
                sret1,
                smt.list(vec![smt.atoms().und, smt.atom("bv16"), smt.numeral(64)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy16,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv18446744073709551615"),
                    smt.numeral(64),
                ]),
            ),
        ]),
        smt.eq(sx16, sy16),
        smt.eq(sx16, sx32),
    ]));
    // round 3
    let sret3 = declare(
        smt,
        format!("sret3_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let sy8 = declare(
        smt,
        format!("sy8_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let sx8 = declare(
        smt,
        format!("sx8_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(sy8, smt.bvashr(sx16, smt.atom("#x0000000000000008"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy8,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv18446744073709551615"),
                    smt.numeral(64),
                ]),
            ),
        ]),
        smt.eq(sret3, sret2),
        smt.eq(
            sret3,
            smt.list(vec![
                smt.atom("bvadd"),
                sret2,
                smt.list(vec![smt.atoms().und, smt.atom("bv8"), smt.numeral(64)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy8,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv18446744073709551615"),
                    smt.numeral(64),
                ]),
            ),
        ]),
        smt.eq(sx8, sy8),
        smt.eq(sx8, sx16),
    ]));
    // round 4
    let sret4 = declare(
        smt,
        format!("sret4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let sy4 = declare(
        smt,
        format!("sy4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let sx4 = declare(
        smt,
        format!("sx4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(sy4, smt.bvashr(sx8, smt.atom("#x0000000000000004"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy4,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv18446744073709551615"),
                    smt.numeral(64),
                ]),
            ),
        ]),
        smt.eq(sret4, sret3),
        smt.eq(
            sret4,
            smt.list(vec![
                smt.atom("bvadd"),
                sret3,
                smt.list(vec![smt.atoms().und, smt.atom("bv4"), smt.numeral(64)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy4,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv18446744073709551615"),
                    smt.numeral(64),
                ]),
            ),
        ]),
        smt.eq(sx4, sy4),
        smt.eq(sx4, sx8),
    ]));
    // round 5
    let sret5 = declare(
        smt,
        format!("sret5_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let sy2 = declare(
        smt,
        format!("sy2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let sx2 = declare(
        smt,
        format!("sx2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(sy2, smt.bvashr(sx4, smt.atom("#x0000000000000002"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy2,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv18446744073709551615"),
                    smt.numeral(64),
                ]),
            ),
        ]),
        smt.eq(sret5, sret4),
        smt.eq(
            sret5,
            smt.list(vec![
                smt.atom("bvadd"),
                sret4,
                smt.list(vec![smt.atoms().und, smt.atom("bv2"), smt.numeral(64)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy2,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv18446744073709551615"),
                    smt.numeral(64),
                ]),
            ),
        ]),
        smt.eq(sx2, sy2),
        smt.eq(sx2, sx4),
    ]));
    // round 6
    let sret6 = declare(
        smt,
        format!("sret6_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let sy1 = declare(
        smt,
        format!("sy1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let sx1 = declare(
        smt,
        format!("sx1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.eq(sy1, smt.bvashr(sx2, smt.atom("#x0000000000000001"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy1,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv18446744073709551615"),
                    smt.numeral(64),
                ]),
            ),
        ]),
        smt.eq(sret6, sret5),
        smt.eq(
            sret6,
            smt.list(vec![
                smt.atom("bvadd"),
                sret5,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(64)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy1,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv18446744073709551615"),
                    smt.numeral(64),
                ]),
            ),
        ]),
        smt.eq(sx1, sy1),
        smt.eq(sx1, sx2),
    ]));
    // last round
    let sret7 = declare(
        smt,
        format!("sret7_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sx1,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv18446744073709551615"),
                    smt.numeral(64),
                ]),
            ),
        ]),
        smt.eq(sret7, sret6),
        smt.eq(
            sret7,
            smt.list(vec![
                smt.atom("bvadd"),
                sret6,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(64)]),
            ]),
        ),
    ]));
    let clsret = declare(
        smt,
        format!("clsret_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.eq(
            sret7,
            smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
        ),
        smt.eq(clsret, sret7),
        smt.eq(
            clsret,
            smt.list(vec![
                smt.atom("bvsub"),
                sret7,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(64)]),
            ]),
        ),
    ]));
    let cls64ret = declare(
        smt,
        format!("cls64ret_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(64)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("bvsle"),
            smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(64)]),
            x,
        ]),
        smt.eq(cls64ret, clzret),
        smt.eq(cls64ret, clsret),
    ]));

    cls64ret
}

pub fn cls32(smt: &mut Context, x: SExpr, id: usize) -> SExpr {
    let x = smt.extract(31, 0, x);

    // Generated code.
    // total zeros counter
    let zret0 = declare(
        smt,
        format!("zret0_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(
        zret0,
        smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
    ));
    // round 1
    let zret2 = declare(
        smt,
        format!("zret2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let zy16 = declare(
        smt,
        format!("zy16_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let zx16 = declare(
        smt,
        format!("zx16_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(zy16, smt.bvlshr(x, smt.atom("#x00000010"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy16,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(zret2, zret0),
        smt.eq(
            zret2,
            smt.list(vec![
                smt.atom("bvadd"),
                zret0,
                smt.list(vec![smt.atoms().und, smt.atom("bv16"), smt.numeral(32)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy16,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(zx16, zy16),
        smt.eq(zx16, x),
    ]));
    // round 2
    let zret3 = declare(
        smt,
        format!("zret3_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let zy8 = declare(
        smt,
        format!("zy8_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let zx8 = declare(
        smt,
        format!("zx8_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(zy8, smt.bvlshr(zx16, smt.atom("#x00000008"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy8,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(zret3, zret2),
        smt.eq(
            zret3,
            smt.list(vec![
                smt.atom("bvadd"),
                zret2,
                smt.list(vec![smt.atoms().und, smt.atom("bv8"), smt.numeral(32)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy8,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(zx8, zy8),
        smt.eq(zx8, zx16),
    ]));
    // round 3
    let zret4 = declare(
        smt,
        format!("zret4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let zy4 = declare(
        smt,
        format!("zy4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let zx4 = declare(
        smt,
        format!("zx4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(zy4, smt.bvlshr(zx8, smt.atom("#x00000004"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy4,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(zret4, zret3),
        smt.eq(
            zret4,
            smt.list(vec![
                smt.atom("bvadd"),
                zret3,
                smt.list(vec![smt.atoms().und, smt.atom("bv4"), smt.numeral(32)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy4,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(zx4, zy4),
        smt.eq(zx4, zx8),
    ]));
    // round 4
    let zret5 = declare(
        smt,
        format!("zret5_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let zy2 = declare(
        smt,
        format!("zy2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let zx2 = declare(
        smt,
        format!("zx2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(zy2, smt.bvlshr(zx4, smt.atom("#x00000002"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy2,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(zret5, zret4),
        smt.eq(
            zret5,
            smt.list(vec![
                smt.atom("bvadd"),
                zret4,
                smt.list(vec![smt.atoms().und, smt.atom("bv2"), smt.numeral(32)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy2,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(zx2, zy2),
        smt.eq(zx2, zx4),
    ]));
    // round 5
    let zret6 = declare(
        smt,
        format!("zret6_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let zy1 = declare(
        smt,
        format!("zy1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let zx1 = declare(
        smt,
        format!("zx1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(zy1, smt.bvlshr(zx2, smt.atom("#x00000001"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(zret6, zret5),
        smt.eq(
            zret6,
            smt.list(vec![
                smt.atom("bvadd"),
                zret5,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(32)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(zx1, zy1),
        smt.eq(zx1, zx2),
    ]));
    // last round
    let zret7 = declare(
        smt,
        format!("zret7_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zx1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            ),
        ]),
        smt.eq(zret7, zret6),
        smt.eq(
            zret7,
            smt.list(vec![
                smt.atom("bvadd"),
                zret6,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(32)]),
            ]),
        ),
    ]));
    let clzret = declare(
        smt,
        format!("clzret_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.eq(
            zret7,
            smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
        ),
        smt.eq(clzret, zret7),
        smt.eq(
            clzret,
            smt.list(vec![
                smt.atom("bvsub"),
                zret7,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(32)]),
            ]),
        ),
    ]));
    // total zeros counter
    let sret0 = declare(
        smt,
        format!("sret0_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(
        sret0,
        smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
    ));
    // round 1
    let sret2 = declare(
        smt,
        format!("sret2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let sy16 = declare(
        smt,
        format!("sy16_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let sx16 = declare(
        smt,
        format!("sx16_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(sy16, smt.bvashr(x, smt.atom("#x00000010"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy16,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv4294967295"),
                    smt.numeral(32),
                ]),
            ),
        ]),
        smt.eq(sret2, sret0),
        smt.eq(
            sret2,
            smt.list(vec![
                smt.atom("bvadd"),
                sret0,
                smt.list(vec![smt.atoms().und, smt.atom("bv16"), smt.numeral(32)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy16,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv4294967295"),
                    smt.numeral(32),
                ]),
            ),
        ]),
        smt.eq(sx16, sy16),
        smt.eq(sx16, x),
    ]));
    // round 2
    let sret3 = declare(
        smt,
        format!("sret3_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let sy8 = declare(
        smt,
        format!("sy8_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let sx8 = declare(
        smt,
        format!("sx8_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(sy8, smt.bvashr(sx16, smt.atom("#x00000008"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy8,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv4294967295"),
                    smt.numeral(32),
                ]),
            ),
        ]),
        smt.eq(sret3, sret2),
        smt.eq(
            sret3,
            smt.list(vec![
                smt.atom("bvadd"),
                sret2,
                smt.list(vec![smt.atoms().und, smt.atom("bv8"), smt.numeral(32)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy8,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv4294967295"),
                    smt.numeral(32),
                ]),
            ),
        ]),
        smt.eq(sx8, sy8),
        smt.eq(sx8, sx16),
    ]));
    // round 3
    let sret4 = declare(
        smt,
        format!("sret4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let sy4 = declare(
        smt,
        format!("sy4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let sx4 = declare(
        smt,
        format!("sx4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(sy4, smt.bvashr(sx8, smt.atom("#x00000004"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy4,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv4294967295"),
                    smt.numeral(32),
                ]),
            ),
        ]),
        smt.eq(sret4, sret3),
        smt.eq(
            sret4,
            smt.list(vec![
                smt.atom("bvadd"),
                sret3,
                smt.list(vec![smt.atoms().und, smt.atom("bv4"), smt.numeral(32)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy4,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv4294967295"),
                    smt.numeral(32),
                ]),
            ),
        ]),
        smt.eq(sx4, sy4),
        smt.eq(sx4, sx8),
    ]));
    // round 4
    let sret5 = declare(
        smt,
        format!("sret5_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let sy2 = declare(
        smt,
        format!("sy2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let sx2 = declare(
        smt,
        format!("sx2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(sy2, smt.bvashr(sx4, smt.atom("#x00000002"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy2,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv4294967295"),
                    smt.numeral(32),
                ]),
            ),
        ]),
        smt.eq(sret5, sret4),
        smt.eq(
            sret5,
            smt.list(vec![
                smt.atom("bvadd"),
                sret4,
                smt.list(vec![smt.atoms().und, smt.atom("bv2"), smt.numeral(32)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy2,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv4294967295"),
                    smt.numeral(32),
                ]),
            ),
        ]),
        smt.eq(sx2, sy2),
        smt.eq(sx2, sx4),
    ]));
    // round 5
    let sret6 = declare(
        smt,
        format!("sret6_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let sy1 = declare(
        smt,
        format!("sy1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let sx1 = declare(
        smt,
        format!("sx1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.eq(sy1, smt.bvashr(sx2, smt.atom("#x00000001"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy1,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv4294967295"),
                    smt.numeral(32),
                ]),
            ),
        ]),
        smt.eq(sret6, sret5),
        smt.eq(
            sret6,
            smt.list(vec![
                smt.atom("bvadd"),
                sret5,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(32)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy1,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv4294967295"),
                    smt.numeral(32),
                ]),
            ),
        ]),
        smt.eq(sx1, sy1),
        smt.eq(sx1, sx2),
    ]));
    // last round
    let sret7 = declare(
        smt,
        format!("sret7_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sx1,
                smt.list(vec![
                    smt.atoms().und,
                    smt.atom("bv4294967295"),
                    smt.numeral(32),
                ]),
            ),
        ]),
        smt.eq(sret7, sret6),
        smt.eq(
            sret7,
            smt.list(vec![
                smt.atom("bvadd"),
                sret6,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(32)]),
            ]),
        ),
    ]));
    let clsret = declare(
        smt,
        format!("clsret_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.eq(
            sret7,
            smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
        ),
        smt.eq(clsret, sret7),
        smt.eq(
            clsret,
            smt.list(vec![
                smt.atom("bvsub"),
                sret7,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(32)]),
            ]),
        ),
    ]));
    let cls32ret = declare(
        smt,
        format!("cls32ret_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(32)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("bvsle"),
            smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(32)]),
            x,
        ]),
        smt.eq(cls32ret, clzret),
        smt.eq(cls32ret, clsret),
    ]));

    cls32ret
}

pub fn cls16(smt: &mut Context, x: SExpr, id: usize) -> SExpr {
    let x = smt.extract(15, 0, x);

    // Generated code.
    // total zeros counter
    let zret0 = declare(
        smt,
        format!("zret0_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.eq(
        zret0,
        smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
    ));
    // round 1
    let zret3 = declare(
        smt,
        format!("zret3_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let zy8 = declare(
        smt,
        format!("zy8_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let zx8 = declare(
        smt,
        format!("zx8_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.eq(zy8, smt.bvlshr(x, smt.atom("#x0008"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy8,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(zret3, zret0),
        smt.eq(
            zret3,
            smt.list(vec![
                smt.atom("bvadd"),
                zret0,
                smt.list(vec![smt.atoms().und, smt.atom("bv8"), smt.numeral(16)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy8,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(zx8, zy8),
        smt.eq(zx8, x),
    ]));
    // round 2
    let zret4 = declare(
        smt,
        format!("zret4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let zy4 = declare(
        smt,
        format!("zy4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let zx4 = declare(
        smt,
        format!("zx4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.eq(zy4, smt.bvlshr(zx8, smt.atom("#x0004"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy4,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(zret4, zret3),
        smt.eq(
            zret4,
            smt.list(vec![
                smt.atom("bvadd"),
                zret3,
                smt.list(vec![smt.atoms().und, smt.atom("bv4"), smt.numeral(16)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy4,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(zx4, zy4),
        smt.eq(zx4, zx8),
    ]));
    // round 3
    let zret5 = declare(
        smt,
        format!("zret5_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let zy2 = declare(
        smt,
        format!("zy2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let zx2 = declare(
        smt,
        format!("zx2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.eq(zy2, smt.bvlshr(zx4, smt.atom("#x0002"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy2,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(zret5, zret4),
        smt.eq(
            zret5,
            smt.list(vec![
                smt.atom("bvadd"),
                zret4,
                smt.list(vec![smt.atoms().und, smt.atom("bv2"), smt.numeral(16)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy2,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(zx2, zy2),
        smt.eq(zx2, zx4),
    ]));
    // round 4
    let zret6 = declare(
        smt,
        format!("zret6_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let zy1 = declare(
        smt,
        format!("zy1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let zx1 = declare(
        smt,
        format!("zx1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.eq(zy1, smt.bvlshr(zx2, smt.atom("#x0001"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(zret6, zret5),
        smt.eq(
            zret6,
            smt.list(vec![
                smt.atom("bvadd"),
                zret5,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(16)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(zx1, zy1),
        smt.eq(zx1, zx2),
    ]));
    // last round
    let zret7 = declare(
        smt,
        format!("zret7_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zx1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(zret7, zret6),
        smt.eq(
            zret7,
            smt.list(vec![
                smt.atom("bvadd"),
                zret6,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(16)]),
            ]),
        ),
    ]));
    let clzret = declare(
        smt,
        format!("clzret_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.eq(
            zret7,
            smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
        ),
        smt.eq(clzret, zret7),
        smt.eq(
            clzret,
            smt.list(vec![
                smt.atom("bvsub"),
                zret7,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(16)]),
            ]),
        ),
    ]));
    // total zeros counter
    let sret0 = declare(
        smt,
        format!("sret0_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.eq(
        sret0,
        smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
    ));
    // round 1
    let sret3 = declare(
        smt,
        format!("sret3_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let sy8 = declare(
        smt,
        format!("sy8_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let sx8 = declare(
        smt,
        format!("sx8_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.eq(sy8, smt.bvashr(x, smt.atom("#x0008"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy8,
                smt.list(vec![smt.atoms().und, smt.atom("bv65535"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(sret3, sret0),
        smt.eq(
            sret3,
            smt.list(vec![
                smt.atom("bvadd"),
                sret0,
                smt.list(vec![smt.atoms().und, smt.atom("bv8"), smt.numeral(16)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy8,
                smt.list(vec![smt.atoms().und, smt.atom("bv65535"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(sx8, sy8),
        smt.eq(sx8, x),
    ]));
    // round 2
    let sret4 = declare(
        smt,
        format!("sret4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let sy4 = declare(
        smt,
        format!("sy4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let sx4 = declare(
        smt,
        format!("sx4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.eq(sy4, smt.bvashr(sx8, smt.atom("#x0004"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy4,
                smt.list(vec![smt.atoms().und, smt.atom("bv65535"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(sret4, sret3),
        smt.eq(
            sret4,
            smt.list(vec![
                smt.atom("bvadd"),
                sret3,
                smt.list(vec![smt.atoms().und, smt.atom("bv4"), smt.numeral(16)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy4,
                smt.list(vec![smt.atoms().und, smt.atom("bv65535"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(sx4, sy4),
        smt.eq(sx4, sx8),
    ]));
    // round 3
    let sret5 = declare(
        smt,
        format!("sret5_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let sy2 = declare(
        smt,
        format!("sy2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let sx2 = declare(
        smt,
        format!("sx2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.eq(sy2, smt.bvashr(sx4, smt.atom("#x0002"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy2,
                smt.list(vec![smt.atoms().und, smt.atom("bv65535"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(sret5, sret4),
        smt.eq(
            sret5,
            smt.list(vec![
                smt.atom("bvadd"),
                sret4,
                smt.list(vec![smt.atoms().und, smt.atom("bv2"), smt.numeral(16)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy2,
                smt.list(vec![smt.atoms().und, smt.atom("bv65535"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(sx2, sy2),
        smt.eq(sx2, sx4),
    ]));
    // round 4
    let sret6 = declare(
        smt,
        format!("sret6_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let sy1 = declare(
        smt,
        format!("sy1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let sx1 = declare(
        smt,
        format!("sx1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.eq(sy1, smt.bvashr(sx2, smt.atom("#x0001"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy1,
                smt.list(vec![smt.atoms().und, smt.atom("bv65535"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(sret6, sret5),
        smt.eq(
            sret6,
            smt.list(vec![
                smt.atom("bvadd"),
                sret5,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(16)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy1,
                smt.list(vec![smt.atoms().und, smt.atom("bv65535"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(sx1, sy1),
        smt.eq(sx1, sx2),
    ]));
    // last round
    let sret7 = declare(
        smt,
        format!("sret7_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sx1,
                smt.list(vec![smt.atoms().und, smt.atom("bv65535"), smt.numeral(16)]),
            ),
        ]),
        smt.eq(sret7, sret6),
        smt.eq(
            sret7,
            smt.list(vec![
                smt.atom("bvadd"),
                sret6,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(16)]),
            ]),
        ),
    ]));
    let clsret = declare(
        smt,
        format!("clsret_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.eq(
            sret7,
            smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
        ),
        smt.eq(clsret, sret7),
        smt.eq(
            clsret,
            smt.list(vec![
                smt.atom("bvsub"),
                sret7,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(16)]),
            ]),
        ),
    ]));
    let cls16ret = declare(
        smt,
        format!("cls16ret_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(16)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("bvsle"),
            smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(16)]),
            x,
        ]),
        smt.eq(cls16ret, clzret),
        smt.eq(cls16ret, clsret),
    ]));

    cls16ret
}

pub fn cls8(smt: &mut Context, x: SExpr, id: usize) -> SExpr {
    let x = smt.extract(7, 0, x);

    // Generated code.
    // total zeros counter
    let zret0 = declare(
        smt,
        format!("zret0_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let _ = smt.assert(smt.eq(
        zret0,
        smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(8)]),
    ));
    // round 1
    let zret4 = declare(
        smt,
        format!("zret4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let zy4 = declare(
        smt,
        format!("zy4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let zx4 = declare(
        smt,
        format!("zx4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let _ = smt.assert(smt.eq(zy4, smt.bvlshr(x, smt.atom("#x04"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy4,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(8)]),
            ),
        ]),
        smt.eq(zret4, zret0),
        smt.eq(
            zret4,
            smt.list(vec![
                smt.atom("bvadd"),
                zret0,
                smt.list(vec![smt.atoms().und, smt.atom("bv4"), smt.numeral(8)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy4,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(8)]),
            ),
        ]),
        smt.eq(zx4, zy4),
        smt.eq(zx4, x),
    ]));
    // round 2
    let zret5 = declare(
        smt,
        format!("zret5_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let zy2 = declare(
        smt,
        format!("zy2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let zx2 = declare(
        smt,
        format!("zx2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let _ = smt.assert(smt.eq(zy2, smt.bvlshr(zx4, smt.atom("#x02"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy2,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(8)]),
            ),
        ]),
        smt.eq(zret5, zret4),
        smt.eq(
            zret5,
            smt.list(vec![
                smt.atom("bvadd"),
                zret4,
                smt.list(vec![smt.atoms().und, smt.atom("bv2"), smt.numeral(8)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy2,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(8)]),
            ),
        ]),
        smt.eq(zx2, zy2),
        smt.eq(zx2, zx4),
    ]));
    // round 3
    let zret6 = declare(
        smt,
        format!("zret6_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let zy1 = declare(
        smt,
        format!("zy1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let zx1 = declare(
        smt,
        format!("zx1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let _ = smt.assert(smt.eq(zy1, smt.bvlshr(zx2, smt.atom("#x01"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(8)]),
            ),
        ]),
        smt.eq(zret6, zret5),
        smt.eq(
            zret6,
            smt.list(vec![
                smt.atom("bvadd"),
                zret5,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(8)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zy1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(8)]),
            ),
        ]),
        smt.eq(zx1, zy1),
        smt.eq(zx1, zx2),
    ]));
    // last round
    let zret7 = declare(
        smt,
        format!("zret7_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                zx1,
                smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(8)]),
            ),
        ]),
        smt.eq(zret7, zret6),
        smt.eq(
            zret7,
            smt.list(vec![
                smt.atom("bvadd"),
                zret6,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(8)]),
            ]),
        ),
    ]));
    let clzret = declare(
        smt,
        format!("clzret_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.eq(
            zret7,
            smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(8)]),
        ),
        smt.eq(clzret, zret7),
        smt.eq(
            clzret,
            smt.list(vec![
                smt.atom("bvsub"),
                zret7,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(8)]),
            ]),
        ),
    ]));
    // total zeros counter
    let sret0 = declare(
        smt,
        format!("sret0_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let _ = smt.assert(smt.eq(
        sret0,
        smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(8)]),
    ));
    // round 1
    let sret4 = declare(
        smt,
        format!("sret4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let sy4 = declare(
        smt,
        format!("sy4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let sx4 = declare(
        smt,
        format!("sx4_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let _ = smt.assert(smt.eq(sy4, smt.bvashr(x, smt.atom("#x04"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy4,
                smt.list(vec![smt.atoms().und, smt.atom("bv255"), smt.numeral(8)]),
            ),
        ]),
        smt.eq(sret4, sret0),
        smt.eq(
            sret4,
            smt.list(vec![
                smt.atom("bvadd"),
                sret0,
                smt.list(vec![smt.atoms().und, smt.atom("bv4"), smt.numeral(8)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy4,
                smt.list(vec![smt.atoms().und, smt.atom("bv255"), smt.numeral(8)]),
            ),
        ]),
        smt.eq(sx4, sy4),
        smt.eq(sx4, x),
    ]));
    // round 2
    let sret5 = declare(
        smt,
        format!("sret5_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let sy2 = declare(
        smt,
        format!("sy2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let sx2 = declare(
        smt,
        format!("sx2_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let _ = smt.assert(smt.eq(sy2, smt.bvashr(sx4, smt.atom("#x02"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy2,
                smt.list(vec![smt.atoms().und, smt.atom("bv255"), smt.numeral(8)]),
            ),
        ]),
        smt.eq(sret5, sret4),
        smt.eq(
            sret5,
            smt.list(vec![
                smt.atom("bvadd"),
                sret4,
                smt.list(vec![smt.atoms().und, smt.atom("bv2"), smt.numeral(8)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy2,
                smt.list(vec![smt.atoms().und, smt.atom("bv255"), smt.numeral(8)]),
            ),
        ]),
        smt.eq(sx2, sy2),
        smt.eq(sx2, sx4),
    ]));
    // round 3
    let sret6 = declare(
        smt,
        format!("sret6_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let sy1 = declare(
        smt,
        format!("sy1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let sx1 = declare(
        smt,
        format!("sx1_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let _ = smt.assert(smt.eq(sy1, smt.bvashr(sx2, smt.atom("#x01"))));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy1,
                smt.list(vec![smt.atoms().und, smt.atom("bv255"), smt.numeral(8)]),
            ),
        ]),
        smt.eq(sret6, sret5),
        smt.eq(
            sret6,
            smt.list(vec![
                smt.atom("bvadd"),
                sret5,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(8)]),
            ]),
        ),
    ]));
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sy1,
                smt.list(vec![smt.atoms().und, smt.atom("bv255"), smt.numeral(8)]),
            ),
        ]),
        smt.eq(sx1, sy1),
        smt.eq(sx1, sx2),
    ]));
    // last round
    let sret7 = declare(
        smt,
        format!("sret7_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("not"),
            smt.eq(
                sx1,
                smt.list(vec![smt.atoms().und, smt.atom("bv255"), smt.numeral(8)]),
            ),
        ]),
        smt.eq(sret7, sret6),
        smt.eq(
            sret7,
            smt.list(vec![
                smt.atom("bvadd"),
                sret6,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(8)]),
            ]),
        ),
    ]));
    let clsret = declare(
        smt,
        format!("clsret_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.eq(
            sret7,
            smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(8)]),
        ),
        smt.eq(clsret, sret7),
        smt.eq(
            clsret,
            smt.list(vec![
                smt.atom("bvsub"),
                sret7,
                smt.list(vec![smt.atoms().und, smt.atom("bv1"), smt.numeral(8)]),
            ]),
        ),
    ]));
    let cls8ret = declare(
        smt,
        format!("cls8ret_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(8)]),
    );
    let _ = smt.assert(smt.list(vec![
        smt.atom("ite"),
        smt.list(vec![
            smt.atom("bvsle"),
            smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(8)]),
            x,
        ]),
        smt.eq(cls8ret, clzret),
        smt.eq(cls8ret, clsret),
    ]));

    cls8ret
}

pub fn cls1(smt: &mut Context, id: usize) -> SExpr {
    // Generated code.
    let cls1ret = declare(
        smt,
        format!("cls1ret_{id}", id = id),
        smt.list(vec![smt.atoms().und, smt.atom("BitVec"), smt.numeral(1)]),
    );
    let _ = smt.assert(smt.eq(
        cls1ret,
        smt.list(vec![smt.atoms().und, smt.atom("bv0"), smt.numeral(1)]),
    ));

    cls1ret
}
