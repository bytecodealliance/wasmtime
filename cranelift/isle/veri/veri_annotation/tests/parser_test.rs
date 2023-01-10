use veri_annotation::parser;
use veri_ir::isle_annotations::isle_annotation_for_term;

#[test]
fn test_type() {
    assert!(parser::TypeParser::new().parse("bv").is_ok());
    assert!(parser::TypeParser::new().parse("bv8").is_ok());
    assert!(parser::TypeParser::new().parse("bool").is_ok());
    assert!(parser::TypeParser::new().parse("isleType").is_ok());
}

#[test]
fn test_bound_var() {
    assert!(parser::BoundVarParser::new().parse("b").is_ok());
    assert!(parser::BoundVarParser::new().parse("bv").is_err());
    assert!(parser::BoundVarParser::new().parse("arg").is_ok());
    assert!(parser::BoundVarParser::new().parse("ba").is_ok());
}

#[test]
fn test_term_signature() {
    assert!(parser::TermSignatureParser::new()
        .parse("(sig (args) (ret: bool))")
        .is_ok());
    assert!(parser::TermSignatureParser::new()
        .parse("(sig (args a: bool) (ret: bool))")
        .is_ok());
    assert!(parser::TermSignatureParser::new()
        .parse("(sig (args a: bool, b: bv) (ret: bool))")
        .is_ok());
}

#[test]
fn test_const() {
    assert!(parser::ConstParser::new().parse("10i8: bv").is_ok());
    assert!(parser::ConstParser::new().parse("true: bool").is_err());
}

#[test]
fn test_width() {
    assert!(parser::WidthParser::new().parse("(regwidth)").is_ok());
}

#[test]
fn test_expr() {
    // consts
    assert!(parser::ExprParser::new().parse("(a)").is_ok());
    assert!(parser::ExprParser::new().parse("(-1i16: bv)").is_ok());
    assert!(parser::ExprParser::new().parse("(true)").is_ok());
    assert!(parser::ExprParser::new().parse("(false)").is_ok());
    assert!(parser::ExprParser::new().parse("(tywidth)").is_ok());

    // boolean operations
    assert!(parser::ExprParser::new().parse("(!(a))").is_ok());
    assert!(parser::ExprParser::new().parse("(&& (a) (b))").is_ok());
    assert!(parser::ExprParser::new().parse("(|| (a) (false))").is_ok());
    assert!(parser::ExprParser::new().parse("(=> (true) (b))").is_ok());
    assert!(parser::ExprParser::new().parse("(= (a) (false))").is_ok());
    assert!(parser::ExprParser::new()
        .parse("(<= (a) (10i4: bv))")
        .is_ok());
    assert!(parser::ExprParser::new()
        .parse("(&& (|| (a) (b)) (c))")
        .is_ok());
    assert!(parser::ExprParser::new().parse("(&& (!(a)) (b))").is_ok());

    // bv operations
    assert!(parser::ExprParser::new().parse("(-(a))").is_ok());
    assert!(parser::ExprParser::new().parse("(~(a))").is_ok());
    assert!(parser::ExprParser::new().parse("(clz (a))").is_ok());
    assert!(parser::ExprParser::new().parse("(a64clz (ty) (a))").is_ok());
    assert!(parser::ExprParser::new().parse("(cls (a))").is_ok());
    assert!(parser::ExprParser::new().parse("(a64cls (ty) (a))").is_ok());
    assert!(parser::ExprParser::new().parse("(+ (-(a)) (b))").is_ok());
    assert!(parser::ExprParser::new().parse("(- (a) (~(b)))").is_ok());
    assert!(parser::ExprParser::new().parse("(& (a) (b))").is_ok());

    // conversions
    //assert!(parser::ExprParser::new().parse("(zero_ext 4 (a))").is_ok());
    //assert!(parser::ExprParser::new()
    //    .parse("(sign_ext 2 (-12i4: bv))")
    //    .is_ok());
    //assert!(parser::ExprParser::new().parse("(extract 0 8 (a))").is_ok());
    //assert!(parser::ExprParser::new().parse("(conv_to 6 (b))").is_ok());
    //assert!(parser::ExprParser::new().parse("(conv_to (a) (b))").is_ok());
    //assert!(parser::ExprParser::new()
    //    .parse("(signed_conv_to 6 (b))")
    //    .is_ok());
    //assert!(parser::ExprParser::new()
    //    .parse("(signed_conv_to (a) (b))")
    //    .is_ok());
    //assert!(parser::ExprParser::new()
    //    .parse("(conv_from 16 (8i128: bv))")
    //    .is_ok());

    // conditional
    assert!(parser::ExprParser::new()
        .parse("(if (a) {(+ (b) (c))} else {(d)})")
        .is_ok());
}

#[test]
fn test_term_annotation() {
    assert!(parser::TermAnnotationParser::new()
        .parse(
            "(spec (sig (args x, y) (ret))
            (assertions (= (+ (x) (y)) (ret))))"
        )
        .is_ok());
}

#[test]
fn test_real_annotations() {
    // "lower" | "put_in_reg" | "value_reg" | "first_result" | "inst_data"
    let parsed = parser::TermAnnotationParser::new()
        .parse(
            "(spec (sig (args arg) (ret))
            (assertions (= (arg) (ret))))",
        )
        .unwrap();
    let expected = isle_annotation_for_term("lower").unwrap();
    assert_eq!(parsed, expected);

    // value_type
    let parsed = parser::TermAnnotationParser::new()
        .parse(
            "(spec (sig (args arg) (ret))
            (assertions (= (arg) (tywidth))))",
        )
        .unwrap();
    let expected = isle_annotation_for_term("value_type").unwrap();
    //assert_eq!(parsed, expected);

    // has_type
    let parsed = parser::TermAnnotationParser::new()
        .parse(
            "(spec (sig (args ty, arg) (ret))
            (assertions (= (ty) (tywidth)), (= (arg) (ret))))",
        )
        .unwrap();
    let expected = isle_annotation_for_term("has_type").unwrap();
    //assert_eq!(parsed, expected);

    // fits_in_64
    let parsed = parser::TermAnnotationParser::new()
        .parse(
            "(spec (sig (args arg) (ret))
            (assertions (= (arg) (ret)), (<= (arg) (64i128: isleType))))",
        )
        .unwrap();
    let expected = isle_annotation_for_term("fits_in_64").unwrap();
    assert_eq!(parsed, expected);

    // iadd
    let parsed = parser::TermAnnotationParser::new()
        .parse(
            "(spec (sig (args a, b) (r))
            (assertions (= (+ (a) (b)) (r))))",
        )
        .unwrap();
    let expected = isle_annotation_for_term("iadd").unwrap();
    assert_eq!(parsed, expected);

    // add
    let parsed = parser::TermAnnotationParser::new()
        .parse(
            "(spec (sig (args ty, a, b) (r))
            (assertions (= (+ (a) (b)) (r))))",
        )
        .unwrap();
    //let expected = isle_annotation_for_term("add").unwrap();
    //assert_eq!(parsed, expected);

    // imm12_from_negated_value
    //let parsed = parser::TermAnnotationParser::new()
    //    .parse(
    //        "(spec (sig (args imm_arg) (ret))
    //        (assertions (= (-(conv_from 12 (imm_arg))) (ret))))",
    //    )
    //    .unwrap();
    //let expected = isle_annotation_for_term("imm12_from_negated_value").unwrap();
    //assert_eq!(parsed, expected);

    // sub_imm
    //let parsed = parser::TermAnnotationParser::new()
    //    .parse(
    //        "(spec (sig (args ty, reg, imm_arg) (ret))
    //        (assertions (= (-(reg) (conv_from 12 (imm_arg))) (ret))))",
    //    )
    //    .unwrap();
    //let expected = isle_annotation_for_term("sub_imm").unwrap();
    //assert_eq!(parsed, expected);

    // // extend
    // let parsed = parser::TermAnnotationParser::new()
    //     .parse(
    //         "(spec (sig (args a, b, c, d) (ret))
    //          (assertions (if (b) {
    //                          (= (ret) (signed_conv_to (d) (a)))
    //                   } else {
    //                       (= (ret) (conv_to (d) (a)))}),
    //          (= (widthof (a)) (c))
    //      ))",
    //     )
    //     .unwrap();
    // let expected = isle_annotation_for_term("extend").unwrap();
    // assert_eq!(parsed, expected);
}
