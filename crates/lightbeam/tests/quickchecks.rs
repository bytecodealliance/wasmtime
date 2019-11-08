use lazy_static::lazy_static;
use lightbeam::{translate, ExecutableModule};
use quickcheck::quickcheck;

fn translate_wat(wat: &str) -> ExecutableModule {
    let wasm = wat::parse_str(wat).unwrap();
    let compiled = translate(&wasm).unwrap();
    compiled
}

mod op32 {
    use super::{lazy_static, quickcheck, translate_wat, ExecutableModule};

    macro_rules! binop_test {
        ($op:ident, $func:expr) => {
            mod $op {
                use super::{lazy_static, quickcheck, translate_wat, ExecutableModule};
                use std::sync::Once;

                const OP: &str = stringify!($op);

                lazy_static! {
                    static ref AS_PARAMS: ExecutableModule = translate_wat(&format!(
                        "(module (func (param i32) (param i32) (result i32)
                            (i32.{op} (get_local 0) (get_local 1))))",
                        op = OP
                    ));
                }

                quickcheck! {
                    fn as_params(a: i32, b: i32) -> bool {
                         AS_PARAMS.execute_func::<(i32, i32), i32>(0, (a, b)) == Ok($func(a, b))
                    }

                    fn lit_lit(a: i32, b: i32) -> bool {
                        let translated = translate_wat(&format!("
                            (module (func (result i32)
                                (i32.{op} (i32.const {left}) (i32.const {right}))))
                        ", op = OP, left = a, right = b));
                        static ONCE: Once = Once::new();
                        ONCE.call_once(|| translated.disassemble());

                        translated.execute_func::<(), i32>(0, ()) == Ok($func(a, b))
                    }

                    fn lit_reg(a: i32, b: i32) -> bool {
                        let translated = translate_wat(&format!("
                            (module (func (param i32) (result i32)
                                (i32.{op} (i32.const {left}) (get_local 0))))
                        ", op = OP, left = a));
                        static ONCE: Once = Once::new();
                        ONCE.call_once(|| translated.disassemble());

                        translated.execute_func::<(i32,), i32>(0, (b,)) == Ok($func(a, b))
                    }

                    fn reg_lit(a: i32, b: i32) -> bool {
                        let translated = translate_wat(&format!("
                            (module (func (param i32) (result i32)
                                (i32.{op} (get_local 0) (i32.const {right}))))
                        ", op = OP, right = b));
                        static ONCE: Once = Once::new();
                        ONCE.call_once(|| translated.disassemble());

                        translated.execute_func::<(i32,), i32>(0, (a,)) == Ok($func(a, b))
                    }
                }
            }
        };
    }

    macro_rules! unop_test {
        ($name:ident, $func:expr) => {
            mod $name {
                use super::{lazy_static, quickcheck, translate_wat, ExecutableModule};
                use std::sync::Once;

                lazy_static! {
                    static ref AS_PARAM: ExecutableModule = translate_wat(concat!(
                        "(module (func (param i32) (result i32)
                            (i32.",
                        stringify!($name),
                        " (get_local 0))))"
                    ),);
                }

                quickcheck! {
                    fn as_param(a: u32) -> bool {
                         AS_PARAM.execute_func::<(u32,), u32>(0, (a,)) == Ok($func(a))
                    }

                    fn lit(a: u32) -> bool {
                        let translated = translate_wat(&format!(concat!("
                            (module (func (result i32)
                                (i32.",stringify!($name)," (i32.const {val}))))
                        "), val = a));
                        static ONCE: Once = Once::new();
                        ONCE.call_once(|| translated.disassemble());

                        translated.execute_func::<(), u32>(0, ()) == Ok($func(a))
                    }
                }
            }
        };
    }

    unop_test!(clz, u32::leading_zeros);
    unop_test!(ctz, u32::trailing_zeros);
    unop_test!(popcnt, u32::count_ones);
    unop_test!(eqz, |a: u32| if a == 0 { 1 } else { 0 });

    binop_test!(add, i32::wrapping_add);
    binop_test!(sub, i32::wrapping_sub);
    binop_test!(and, std::ops::BitAnd::bitand);
    binop_test!(or, std::ops::BitOr::bitor);
    binop_test!(xor, std::ops::BitXor::bitxor);
    binop_test!(mul, i32::wrapping_mul);
    binop_test!(eq, |a, b| if a == b { 1 } else { 0 });
    binop_test!(ne, |a, b| if a != b { 1 } else { 0 });
    binop_test!(lt_u, |a, b| if (a as u32) < (b as u32) { 1 } else { 0 });
    binop_test!(le_u, |a, b| if (a as u32) <= (b as u32) { 1 } else { 0 });
    binop_test!(gt_u, |a, b| if (a as u32) > (b as u32) { 1 } else { 0 });
    binop_test!(ge_u, |a, b| if (a as u32) >= (b as u32) { 1 } else { 0 });
    binop_test!(lt_s, |a, b| if a < b { 1 } else { 0 });
    binop_test!(le_s, |a, b| if a <= b { 1 } else { 0 });
    binop_test!(gt_s, |a, b| if a > b { 1 } else { 0 });
    binop_test!(ge_s, |a, b| if a >= b { 1 } else { 0 });
    binop_test!(shl, |a, b| (a as i32).wrapping_shl(b as _));
    binop_test!(shr_s, |a, b| (a as i32).wrapping_shr(b as _));
    binop_test!(shr_u, |a, b| (a as u32).wrapping_shr(b as _) as i32);
    binop_test!(rotl, |a, b| (a as u32).rotate_left(b as _) as i32);
    binop_test!(rotr, |a, b| (a as u32).rotate_right(b as _) as i32);
}

mod op64 {
    use super::{lazy_static, quickcheck, translate_wat, ExecutableModule};

    macro_rules! binop_test {
        ($op:ident, $func:expr) => {
            binop_test!($op, $func, i64);
        };
        ($op:ident, $func:expr, $retty:ident) => {
            mod $op {
                use super::{translate_wat, ExecutableModule, quickcheck, lazy_static};

                const RETTY: &str = stringify!($retty);
                const OP: &str = stringify!($op);

                lazy_static! {
                    static ref AS_PARAMS: ExecutableModule = translate_wat(&format!("
                        (module (func (param i64) (param i64) (result {retty})
                            (i64.{op} (get_local 0) (get_local 1))))
                    ", retty = RETTY, op = OP));
                }

                quickcheck! {
                    fn as_params(a: i64, b: i64) -> bool {
                        AS_PARAMS.execute_func::<(i64, i64), $retty>(0, (a, b)) == Ok($func(a, b) as $retty)
                    }

                    fn lit_lit(a: i64, b: i64) -> bool {
                        translate_wat(&format!("
                            (module (func (result {retty})
                                (i64.{op} (i64.const {left}) (i64.const {right}))))
                        ", retty = RETTY, op = OP, left = a, right = b)).execute_func::<(), $retty>(0, ()) == Ok($func(a, b) as $retty)
                    }

                    fn lit_reg(a: i64, b: i64) -> bool {
                        use std::sync::Once;

                        let translated = translate_wat(&format!("
                            (module (func (param i64) (result {retty})
                                (i64.{op} (i64.const {left}) (get_local 0))))
                        ", retty = RETTY, op = OP, left = a));
                        static ONCE: Once = Once::new();
                        ONCE.call_once(|| translated.disassemble());

                        translated.execute_func::<(i64,), $retty>(0, (b,)) == Ok($func(a, b) as $retty)
                    }

                    fn reg_lit(a: i64, b: i64) -> bool {
                        use std::sync::Once;

                        let translated = translate_wat(&format!("
                            (module (func (param i64) (result {retty})
                                (i64.{op} (get_local 0) (i64.const {right}))))
                        ", retty = RETTY, op = OP, right = b));
                        static ONCE: Once = Once::new();
                        ONCE.call_once(|| translated.disassemble());

                        translated.execute_func::<(i64,), $retty>(0, (a,)) == Ok($func(a, b) as $retty)
                    }
                }
            }
        };
    }

    macro_rules! unop_test {
        ($name:ident, $func:expr) => {
            unop_test!($name, $func, i64);
        };
        ($name:ident, $func:expr, $out_ty:ty) => {
            mod $name {
                use super::{lazy_static, quickcheck, translate_wat, ExecutableModule};
                use std::sync::Once;

                lazy_static! {
                    static ref AS_PARAM: ExecutableModule = translate_wat(concat!(
                        "(module (func (param i64) (result ",
                        stringify!($out_ty),
                        ")
                            (i64.",
                        stringify!($name),
                        " (get_local 0))))"
                    ),);
                }

                quickcheck! {
                    fn as_param(a: u64) -> bool {
                         AS_PARAM.execute_func::<(u64,), $out_ty>(0, (a,)) == Ok($func(a))
                    }

                    fn lit(a: u64) -> bool {
                                                let translated = translate_wat(&format!(concat!("
                            (module (func (result ",stringify!($out_ty),")
                                (i64.",stringify!($name)," (i64.const {val}))))
                        "), val = a));
                        static ONCE: Once = Once::new();
                        ONCE.call_once(|| translated.disassemble());

                        translated.execute_func::<(), $out_ty>(0, ()) == Ok($func(a))
                    }
                }
            }
        };
    }

    unop_test!(clz, |a: u64| a.leading_zeros() as _);
    unop_test!(ctz, |a: u64| a.trailing_zeros() as _);
    unop_test!(popcnt, |a: u64| a.count_ones() as _);
    unop_test!(eqz, |a: u64| if a == 0 { 1 } else { 0 }, i32);

    binop_test!(add, i64::wrapping_add);
    binop_test!(sub, i64::wrapping_sub);
    binop_test!(and, std::ops::BitAnd::bitand);
    binop_test!(or, std::ops::BitOr::bitor);
    binop_test!(xor, std::ops::BitXor::bitxor);
    binop_test!(mul, i64::wrapping_mul);
    binop_test!(eq, |a, b| if a == b { 1 } else { 0 }, i32);
    binop_test!(ne, |a, b| if a != b { 1 } else { 0 }, i32);
    binop_test!(
        lt_u,
        |a, b| if (a as u64) < (b as u64) { 1 } else { 0 },
        i32
    );
    binop_test!(
        le_u,
        |a, b| if (a as u64) <= (b as u64) { 1 } else { 0 },
        i32
    );
    binop_test!(
        gt_u,
        |a, b| if (a as u64) > (b as u64) { 1 } else { 0 },
        i32
    );
    binop_test!(
        ge_u,
        |a, b| if (a as u64) >= (b as u64) { 1 } else { 0 },
        i32
    );
    binop_test!(lt_s, |a, b| if a < b { 1 } else { 0 }, i32);
    binop_test!(le_s, |a, b| if a <= b { 1 } else { 0 }, i32);
    binop_test!(gt_s, |a, b| if a > b { 1 } else { 0 }, i32);
    binop_test!(ge_s, |a, b| if a >= b { 1 } else { 0 }, i32);
    binop_test!(shl, |a, b| (a as i64).wrapping_shl(b as _));
    binop_test!(shr_s, |a, b| (a as i64).wrapping_shr(b as _));
    binop_test!(shr_u, |a, b| (a as u64).wrapping_shr(b as _) as i64);
    binop_test!(rotl, |a, b| (a as u64).rotate_left(b as _) as i64);
    binop_test!(rotr, |a, b| (a as u64).rotate_right(b as _) as i64);
}

mod opf32 {
    use super::{lazy_static, quickcheck, translate_wat, ExecutableModule};

    macro_rules! binop_test {
        ($op:ident, $func:expr) => {
            binop_test!($op, $func, f32);
        };
        ($op:ident, $func:expr, $retty:ident) => {
            mod $op {
                use super::{translate_wat, ExecutableModule, quickcheck, lazy_static};

                const RETTY: &str = stringify!($retty);
                const OP: &str = stringify!($op);

                lazy_static! {
                    static ref AS_PARAMS: ExecutableModule = translate_wat(&format!("
                        (module (func (param f32) (param f32) (result {retty})
                            (f32.{op} (get_local 0) (get_local 1))))
                    ", retty = RETTY, op = OP));
                }

                quickcheck! {
                    fn as_params(a: f32, b: f32) -> bool {
                        AS_PARAMS.execute_func::<(f32, f32), $retty>(0, (a, b)) == Ok($func(a, b) as $retty)
                    }

                    fn lit_lit(a: f32, b: f32) -> bool {
                        translate_wat(&format!("
                            (module (func (result {retty})
                                (f32.{op} (f32.const {left}) (f32.const {right}))))
                        ", retty = RETTY, op = OP, left = a, right = b)).execute_func::<(), $retty>(0, ()) == Ok($func(a, b) as $retty)
                    }

                    fn lit_reg(a: f32, b: f32) -> bool {
                        use std::sync::Once;

                        let translated = translate_wat(&format!("
                            (module (func (param f32) (result {retty})
                                (f32.{op} (f32.const {left}) (get_local 0))))
                        ", retty = RETTY, op = OP, left = a));
                        static ONCE: Once = Once::new();
                        ONCE.call_once(|| translated.disassemble());

                        translated.execute_func::<(f32,), $retty>(0, (b,)) == Ok($func(a, b) as $retty)
                    }

                    fn reg_lit(a: f32, b: f32) -> bool {
                        use std::sync::Once;

                        let translated = translate_wat(&format!("
                            (module (func (param f32) (result {retty})
                                (f32.{op} (get_local 0) (f32.const {right}))))
                        ", retty = RETTY, op = OP, right = b));
                        static ONCE: Once = Once::new();
                        ONCE.call_once(|| translated.disassemble());

                        translated.execute_func::<(f32,), $retty>(0, (a,)) == Ok($func(a, b) as $retty)
                    }
                }
            }
        };
    }

    macro_rules! unop_test {
        ($name:ident, $func:expr) => {
            unop_test!($name, $func, f32);
        };
        ($name:ident, $func:expr, $out_ty:ty) => {
            mod $name {
                use super::{lazy_static, quickcheck, translate_wat, ExecutableModule};
                use std::sync::Once;

                lazy_static! {
                    static ref AS_PARAM: ExecutableModule = translate_wat(concat!(
                        "(module (func (param f32) (result ",
                        stringify!($out_ty),
                        ")
                            (f32.",
                        stringify!($name),
                        " (get_local 0))))"
                    ),);
                }

                quickcheck! {
                    fn as_param(a: f32) -> bool {
                        static ONCE: Once = Once::new();
                        ONCE.call_once(|| AS_PARAM.disassemble());
                        AS_PARAM.execute_func::<(f32,), $out_ty>(0, (a,)) == Ok($func(a))
                    }

                    fn lit(a: f32) -> bool {
                                                let translated = translate_wat(&format!(concat!("
                            (module (func (result ",stringify!($out_ty),")
                                (f32.",stringify!($name)," (f32.const {val}))))
                        "), val = a));
                        static ONCE: Once = Once::new();
                        ONCE.call_once(|| translated.disassemble());

                        translated.execute_func::<(), $out_ty>(0, ()) == Ok($func(a))
                    }
                }
            }
        };
    }

    binop_test!(add, |a, b| a + b);
    binop_test!(mul, |a, b| a * b);
    binop_test!(sub, |a, b| a - b);
    binop_test!(gt, |a, b| a > b, i32);
    binop_test!(lt, |a, b| a < b, i32);
    binop_test!(ge, |a, b| a >= b, i32);
    binop_test!(le, |a, b| a <= b, i32);

    unop_test!(neg, |a: f32| -a);
    unop_test!(abs, |a: f32| a.abs());
}

mod opf64 {
    use super::{lazy_static, quickcheck, translate_wat, ExecutableModule};

    macro_rules! binop_test {
        ($op:ident, $func:expr) => {
            binop_test!($op, $func, f64);
        };
        ($op:ident, $func:expr, $retty:ident) => {
            mod $op {
                use super::{translate_wat, ExecutableModule, quickcheck, lazy_static};

                const RETTY: &str = stringify!($retty);
                const OP: &str = stringify!($op);

                lazy_static! {
                    static ref AS_PARAMS: ExecutableModule = translate_wat(&format!("
                        (module (func (param f64) (param f64) (result {retty})
                            (f64.{op} (get_local 0) (get_local 1))))
                    ", retty = RETTY, op = OP));
                }

                quickcheck! {
                    fn as_params(a: f64, b: f64) -> bool {
                        AS_PARAMS.execute_func::<(f64, f64), $retty>(0, (a, b)) == Ok($func(a, b) as $retty)
                    }

                    fn lit_lit(a: f64, b: f64) -> bool {
                        translate_wat(&format!("
                            (module (func (result {retty})
                                (f64.{op} (f64.const {left}) (f64.const {right}))))
                        ", retty = RETTY, op = OP, left = a, right = b)).execute_func::<(), $retty>(0, ()) == Ok($func(a, b) as $retty)
                    }

                    fn lit_reg(a: f64, b: f64) -> bool {
                        use std::sync::Once;

                        let translated = translate_wat(&format!("
                            (module (func (param f64) (result {retty})
                                (f64.{op} (f64.const {left}) (get_local 0))))
                        ", retty = RETTY, op = OP, left = a));
                        static ONCE: Once = Once::new();
                        ONCE.call_once(|| translated.disassemble());

                        translated.execute_func::<(f64,), $retty>(0, (b,)) == Ok($func(a, b) as $retty)
                    }

                    fn reg_lit(a: f64, b: f64) -> bool {
                        use std::sync::Once;

                        let translated = translate_wat(&format!("
                            (module (func (param f64) (result {retty})
                                (f64.{op} (get_local 0) (f64.const {right}))))
                        ", retty = RETTY, op = OP, right = b));
                        static ONCE: Once = Once::new();
                        ONCE.call_once(|| translated.disassemble());

                        translated.execute_func::<(f64,), $retty>(0, (a,)) == Ok($func(a, b) as $retty)
                    }
                }
            }
        };
    }

    macro_rules! unop_test {
        ($name:ident, $func:expr) => {
            unop_test!($name, $func, f64);
        };
        ($name:ident, $func:expr, $out_ty:ty) => {
            mod $name {
                use super::{lazy_static, quickcheck, translate_wat, ExecutableModule};
                use std::sync::Once;

                lazy_static! {
                    static ref AS_PARAM: ExecutableModule = translate_wat(concat!(
                        "(module (func (param f64) (result ",
                        stringify!($out_ty),
                        ")
                            (f64.",
                        stringify!($name),
                        " (get_local 0))))"
                    ),);
                }

                quickcheck! {
                    fn as_param(a: f64) -> bool {
                        static ONCE: Once = Once::new();
                        ONCE.call_once(|| AS_PARAM.disassemble());
                        AS_PARAM.execute_func::<(f64,), $out_ty>(0, (a,)) == Ok($func(a))
                    }

                    fn lit(a: f64) -> bool {
                                                let translated = translate_wat(&format!(concat!("
                            (module (func (result ",stringify!($out_ty),")
                                (f64.",stringify!($name)," (f64.const {val}))))
                        "), val = a));
                        static ONCE: Once = Once::new();
                        ONCE.call_once(|| translated.disassemble());

                        translated.execute_func::<(), $out_ty>(0, ()) == Ok($func(a))
                    }
                }
            }
        };
    }

    binop_test!(add, |a, b| a + b);
    binop_test!(mul, |a, b| a * b);
    binop_test!(sub, |a, b| a - b);
    binop_test!(gt, |a, b| a > b, i32);
    binop_test!(lt, |a, b| a < b, i32);
    binop_test!(ge, |a, b| a >= b, i32);
    binop_test!(le, |a, b| a <= b, i32);

    unop_test!(neg, |a: f64| -a);
    unop_test!(abs, |a: f64| a.abs());
}

quickcheck! {
    fn if_then_else(a: u32, b: u32) -> bool {
        const CODE: &str = r#"
(module
  (func (param i32) (param i32) (result i32)
    (if (result i32)
      (i32.eq
        (get_local 0)
        (get_local 1)
      )
      (then (get_local 0))
      (else (get_local 1))
    )
  )
)
        "#;

        lazy_static! {
            static ref TRANSLATED: ExecutableModule = {let out = translate_wat(CODE); out.disassemble(); out};
        }

        let out = TRANSLATED.execute_func::<(u32, u32), u32>(0, (a, b));

        out == Ok(if a == b { a } else { b })
    }
}

quickcheck! {
    #[test]
    fn literals(a: i32, b: i64, c: i32, d: i64) -> bool {
        let code = format!(r#"
            (module
              (func (result i32)
                (i32.const {})
              )
              (func (result i64)
                (i64.const {})
              )
              (func (result f32)
                (f32.const {})
              )
              (func (result f64)
                (f64.const {})
              )
            )
        "#, a, b, c, d);

        let translated = translate_wat(&code);

        assert_eq!(translated.execute_func::<(), i32>(0, ()), Ok(a));
        assert_eq!(translated.execute_func::<(), i64>(1, ()), Ok(b));
        assert_eq!(translated.execute_func::<(), f32>(2, ()), Ok(c as _));
        assert_eq!(translated.execute_func::<(), f64>(3, ()), Ok(d as _));

        true
    }
}

quickcheck! {
    #[test]
    fn params(a: i32, b: i64, c: i32, d: i64) -> bool {
        let code = r#"
            (module
              (func (param i32) (param i64) (param f32) (param f64) (result i32)
                (get_local 0)
              )
              (func (param i32) (param i64) (param f32) (param f64) (result i64)
                (get_local 1)
              )
              (func (param i32) (param i64) (param f32) (param f64) (result f32)
                (get_local 2)
              )
              (func (param i32) (param i64) (param f32) (param f64) (result f64)
                (get_local 3)
              )
            )
        "#;

        let c = c as f32;
        let d = d as f64;

        let translated = translate_wat(&code);

        assert_eq!(translated.execute_func::<(i32, i64, f32, f64), i32>(0, (a, b, c, d)), Ok(a));
        assert_eq!(translated.execute_func::<(i32, i64, f32, f64), i64>(1, (a, b, c, d)), Ok(b));
        assert_eq!(translated.execute_func::<(i32, i64, f32, f64), f32>(2, (a, b, c, d)), Ok(c));
        assert_eq!(translated.execute_func::<(i32, i64, f32, f64), f64>(3, (a, b, c, d)), Ok(d));

        true
    }
}

macro_rules! test_select {
    ($name:ident, $ty:ident) => {
        mod $name {
            use super::{lazy_static, quickcheck, translate_wat, ExecutableModule};
            use std::sync::Once;

            lazy_static! {
                static ref AS_PARAMS: ExecutableModule = translate_wat(&format!(
                    "
                    (module
                        (func (param {ty}) (param {ty}) (param i32) (result {ty})
                            (select (get_local 0) (get_local 1) (get_local 2))
                        )
                    )",
                    ty = stringify!($ty)
                ));
            }

            quickcheck! {
                fn as_param(cond: bool, then: $ty, else_: $ty) -> bool {
                     let icond: i32 = if cond { 1 } else { 0 };
                     AS_PARAMS.execute_func::<($ty, $ty, i32), $ty>(0, (then, else_, icond)) ==
                        Ok(if cond { then } else { else_ })
                }

                fn lit(cond: bool, then: $ty, else_: $ty) -> bool {
                    let icond: i32 = if cond { 1 } else { 0 };
                                                    let translated = translate_wat(&format!("
                            (module (func (param {ty}) (param {ty}) (result {ty})
                                (select (get_local 0) (get_local 1) (i32.const {val}))))
                        ",
                        val = icond,
                        ty = stringify!($ty)
                    ));
                    static ONCE: Once = Once::new();
                    ONCE.call_once(|| translated.disassemble());

                    translated.execute_func::<($ty, $ty), $ty>(0, (then, else_)) ==
                        Ok(if cond { then } else { else_ })
                }
            }
        }
    };
}

test_select!(select32, i32);
test_select!(select64, i64);
