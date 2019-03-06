use super::{module::ExecutionError, translate, ExecutableModule};
use wabt;

fn translate_wat(wat: &str) -> ExecutableModule {
    let wasm = wabt::wat2wasm(wat).unwrap();
    let compiled = translate(&wasm).unwrap();
    compiled
}

/// Execute the first function in the module.
fn execute_wat(wat: &str, a: u32, b: u32) -> u32 {
    let translated = translate_wat(wat);
    translated.disassemble();
    translated.execute_func(0, (a, b)).unwrap()
}

#[test]
fn empty() {
    let _ = translate_wat("(module (func))");
}

mod op32 {
    use super::{translate_wat, ExecutableModule};

    macro_rules! binop_test {
        ($op:ident, $func:expr) => {
            mod $op {
                use super::{translate_wat, ExecutableModule};
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
                use super::{translate_wat, ExecutableModule};
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
    use super::{translate_wat, ExecutableModule};

    macro_rules! binop_test {
        ($op:ident, $func:expr) => {
            binop_test!($op, $func, i64);
        };
        ($op:ident, $func:expr, $retty:ident) => {
            mod $op {
                use super::{translate_wat, ExecutableModule};

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
                use super::{translate_wat, ExecutableModule};
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
    use super::{translate_wat, ExecutableModule};

    macro_rules! binop_test {
        ($op:ident, $func:expr) => {
            binop_test!($op, $func, f32);
        };
        ($op:ident, $func:expr, $retty:ident) => {
            mod $op {
                use super::{translate_wat, ExecutableModule};

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
                use super::{translate_wat, ExecutableModule};
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
}

mod opf64 {
    use super::{translate_wat, ExecutableModule};

    macro_rules! binop_test {
        ($op:ident, $func:expr) => {
            binop_test!($op, $func, f64);
        };
        ($op:ident, $func:expr, $retty:ident) => {
            mod $op {
                use super::{translate_wat, ExecutableModule};

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
                use super::{translate_wat, ExecutableModule};
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
#[test]
fn if_without_result() {
    let code = r#"
(module
  (func (param i32) (param i32) (result i32)
    (if
      (i32.eq
        (get_local 0)
        (get_local 1)
      )
      (then (unreachable))
    )

    (get_local 0)
  )
)
    "#;

    assert_eq!(execute_wat(code, 2, 3), 2);
}

#[test]
fn function_call() {
    let code = r#"
(module
  (func (param i32) (param i32) (result i32)
    (call $assert_zero
      (get_local 1)
    )
    (get_local 0)
  )

  (func $assert_zero (param $v i32)
    (local i32)
    (if (get_local $v)
      (unreachable)
    )
  )
)
    "#;

    assert_eq!(execute_wat(code, 2, 0), 2);
}

#[test]
fn large_function() {
    let code = r#"
(module
  (func (param i32) (param i32) (param i32) (param i32)
        (param i32) (param i32)
        (result i32)

    (call $assert_zero
      (get_local 5)
    )
    (get_local 0)
  )

  (func $assert_zero (param $v i32)
    (local i32)
    (if (get_local $v)
      (unreachable)
    )
  )
)
    "#;

    assert_eq!(
        {
            let translated = translate_wat(code);
            translated.disassemble();
            let out: Result<u32, _> = translated.execute_func(0, (5, 4, 3, 2, 1, 0));
            out
        },
        Ok(5)
    );
}

#[test]
fn function_read_args_spill_to_stack() {
    let code = r#"
(module
  (func (param i32) (param i32) (param i32) (param i32)
        (param i32) (param i32) (param i32) (param i32)
        (param i32) (param i32) (param i32) (param i32)
        (result i32)

    (call $assert_zero
      (get_local 7)
    )
    (get_local 0)
  )

  (func $assert_zero (param $v i32)
    (local i32)
    (if (get_local $v)
      (unreachable)
    )
  )
)
    "#;

    assert_eq!(
        {
            let translated = translate_wat(code);
            translated.disassemble();
            translated.execute_func(
                0,
                (
                    7u32, 6u32, 5u32, 4u32, 3u32, 2u32, 1u32, 0u32, 1u32, 2u32, 3u32, 4u32,
                ),
            )
        },
        Ok(7u32)
    );
}

macro_rules! mk_function_write_args_spill_to_stack {
    ($name:ident, $typ:ty) => {
        #[test]
        fn $name() {
            let code = format!(
                "
(module
  (func (param {typ}) (param {typ}) (param {typ}) (param {typ})
        (param {typ}) (param {typ}) (param {typ}) (param {typ})
        (param {typ}) (param {typ}) (param {typ}) (param {typ})
        (result {typ})

    (call $called
      (get_local 0)
      (get_local 1)
      (get_local 2)
      (get_local 3)
      (get_local 4)
      (get_local 5)
      (get_local 6)
      (get_local 7)
      (get_local 8)
      (get_local 9)
      (get_local 10)
      (get_local 11)
    )
  )

  (func $called
        (param {typ}) (param {typ}) (param {typ}) (param {typ})
        (param {typ}) (param {typ}) (param {typ}) (param {typ})
        (param {typ}) (param {typ}) (param {typ}) (param {typ})
        (result {typ})

    (call $assert_zero
      (get_local 11)
    )
    (get_local 0)
  )

  (func $assert_zero (param $v {typ})
    (local {typ})
    (if ({typ}.ne (get_local $v) ({typ}.const 0))
      (unreachable)
    )
  )
)
    ",
                typ = stringify!($typ)
            );

            assert_eq!(
                {
                    let translated = translate_wat(&code);
                    translated.disassemble();
                    let out: Result<$typ, _> = translated.execute_func(
                        0,
                        (
                            11 as $typ, 10 as $typ, 9 as $typ, 8 as $typ, 7 as $typ, 6 as $typ,
                            5 as $typ, 4 as $typ, 3 as $typ, 2 as $typ, 1 as $typ, 0 as $typ,
                        ),
                    );
                    out
                },
                Ok(11)
            );
        }
    };
}

mk_function_write_args_spill_to_stack!(function_write_args_spill_to_stack_i32, i32);
mk_function_write_args_spill_to_stack!(function_write_args_spill_to_stack_i64, i64);

#[test]
fn block() {
    let code = r#"
(module
  (func (param i32) (param i32) (result i32)
    (block (result i32)
        get_local 0
    )
  )
)
    "#;
    assert_eq!(execute_wat(code, 10, 20), 10);
}

#[test]
fn br_block() {
    let code = r#"
(module
  (func (param i32) (param i32) (result i32)
    get_local 1
    (block (result i32)
        get_local 0
        get_local 0
        br 0
        unreachable
    )
    i32.add
  )
)
    "#;

    let translated = translate_wat(code);
    translated.disassemble();

    assert_eq!(
        translated.execute_func::<(i32, i32), i32>(0, (5, 7)),
        Ok(12)
    );
}

// Tests discarding values on the value stack, while
// carrying over the result using a conditional branch.
#[test]
fn brif_block() {
    let code = r#"
(module
  (func (param i32) (param i32) (result i32)
    get_local 1
    (block (result i32)
        get_local 0
        get_local 0
        br_if 0
        unreachable
    )
    i32.add
  )
)
    "#;
    assert_eq!(execute_wat(code, 5, 7), 12);
}

#[test]
fn spec_loop() {
    let code = r#"
(module
  (func
    (call $assert-eq (call $as-binary-operand) (i32.const 12))
    (call $assert-eq (call $break-bare) (i32.const 19))
    (call $assert-eq (call $break-value) (i32.const 18))
    (call $assert-eq (call $break-repeated) (i32.const 18))
    (call $assert-eq (call $break-inner) (i32.const 0x7))
  )
  (func $dummy)
  (func $as-binary-operand (result i32)
    (i32.mul
      (loop (result i32) (call $dummy) (i32.const 3))
      (loop (result i32) (call $dummy) (i32.const 4))
    )
  )
  (func $break-bare (result i32)
    (block (loop (br 1) (br 0) (unreachable)))
    (block (loop (br_if 1 (i32.const 1)) (unreachable)))
    (i32.const 19)
  )
  (func $break-value (result i32)
    (block (result i32)
      (loop (result i32) (br 1 (i32.const 18)) (br 0) (i32.const 19))
    )
  )
  (func $break-repeated (result i32)
    (block (result i32)
      (loop (result i32)
        (br 1 (i32.const 18))
        (br 1 (i32.const 19))
        (drop (br_if 1 (i32.const 20) (i32.const 0)))
        (drop (br_if 1 (i32.const 20) (i32.const 1)))
        (br 1 (i32.const 21))
        (i32.const 21)
      )
    )
  )
  (func $break-inner (result i32)
    (local i32)
    (set_local 0 (i32.const 0))
    (set_local 0 (i32.add (get_local 0) (block (result i32) (loop (result i32) (block (result i32) (br 2 (i32.const 0x1)))))))
    (set_local 0 (i32.add (get_local 0) (block (result i32) (loop (result i32) (loop (result i32) (br 2 (i32.const 0x2)))))))
    (set_local 0 (i32.add (get_local 0) (block (result i32) (loop (result i32) (block (result i32) (loop (result i32) (br 1 (i32.const 0x4))))))))
    (get_local 0)
  )
  (func $assert-eq (param i32) (param i32)
     (if (i32.ne (get_local 0) (get_local 1))
        (unreachable)
     )
  )
)
"#;

    let translated = translate_wat(code);
    translated.disassemble();
    translated.execute_func::<(), ()>(0, ()).unwrap();
}

quickcheck! {
    fn spec_fac(n: i8) -> bool {
        const CODE: &str = r#"
            (module
              (func (param i32) (result i32)
                (local i32)
                (set_local 1 (call $fac-iter (get_local 0)))
                (call $assert-eq (get_local 1) (call $fac-opt (get_local 0)))
                (get_local 1)
              )

              (func $assert-eq (param i32) (param i32)
                 (if (i32.ne (get_local 0) (get_local 1))
                    (unreachable)
                 )
              )

              ;; Iterative factorial
              (func $fac-iter (param i32) (result i32)
                (local i32 i32)
                (set_local 1 (get_local 0))
                (set_local 2 (i32.const 1))
                (block
                  (loop
                    (if
                      (i32.lt_s (get_local 1) (i32.const 2))
                      (then (br 2))
                      (else
                        (set_local 2 (i32.mul (get_local 1) (get_local 2)))
                        (set_local 1 (i32.sub (get_local 1) (i32.const 1)))
                      )
                    )
                    (br 0)
                  )
                )
                (get_local 2)
              )
            
              ;; Optimized factorial.
              (func $fac-opt (param i32) (result i32)
                (local i32)
                (set_local 1 (i32.const 1))
                (block
                  (br_if 0 (i32.lt_s (get_local 0) (i32.const 2)))
                  (loop
                    (set_local 1 (i32.mul (get_local 1) (get_local 0)))
                    (set_local 0 (i32.add (get_local 0) (i32.const -1)))
                    (br_if 0 (i32.gt_s (get_local 0) (i32.const 1)))
                  )
                )
                (get_local 1)
              )
            )"#;

        fn fac(mut n: i32) -> i32 {
            let mut a = 1i32;

            while n > 1 {
                a = a.wrapping_mul(n);
                n -= 1;
            }

            a
        }

        lazy_static! {
            static ref TRANSLATED: ExecutableModule = {
                let out = translate_wat(CODE);
                out.disassemble();
                out
            };
        }

        let n = n as i32;

        assert_eq!(TRANSLATED.execute_func::<(i32,), i32>(2, (n,)), Ok(fac(n)));
        assert_eq!(TRANSLATED.execute_func::<(i32,), i32>(3, (n,)), Ok(fac(n)));
        true
    }
}

// Tests that br_if keeps values in the case if the branch
// hasn't been taken.
#[test]
fn brif_block_passthru() {
    let code = r#"
(module
  (func (param i32) (param i32) (result i32)
    (block (result i32)
        get_local 1
        get_local 0
        br_if 0
        get_local 1
        i32.add
    )
  )
)
    "#;
    assert_eq!(execute_wat(code, 0, 3), 6);
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
#[test]
fn wrong_type() {
    let code = r#"
(module
  (func (param i32) (param i64) (result i32)
    (i32.const 228)
  )
)
    "#;

    let translated = translate_wat(code);
    assert_eq!(
        translated
            .execute_func::<_, ()>(0, (0u32, 0u32))
            .unwrap_err(),
        ExecutionError::TypeMismatch
    );
}

#[test]
fn wrong_index() {
    let code = r#"
(module
  (func (param i32) (param i64) (result i32)
    (i32.const 228)
  )
)
    "#;

    let translated = translate_wat(code);
    assert_eq!(
        translated
            .execute_func::<_, ()>(10, (0u32, 0u32))
            .unwrap_err(),
        ExecutionError::FuncIndexOutOfBounds
    );
}

fn iterative_fib_baseline(n: u32) -> u32 {
    let (mut a, mut b) = (1, 1);

    for _ in 0..n {
        let old_a = a;
        a = b;
        b += old_a;
    }

    a
}

const FIBONACCI: &str = r#"
(module
  (func $fib (param $n i32) (result i32)
    (if (result i32)
      (i32.eq
        (i32.const 0)
        (get_local $n)
      )
      (then
        (i32.const 1)
      )
      (else
        (if (result i32)
          (i32.eq
            (i32.const 1)
            (get_local $n)
          )
          (then
            (i32.const 1)
          )
          (else
            (i32.add
              ;; fib(n - 1)
              (call $fib
                (i32.add
                  (get_local $n)
                  (i32.const -1)
                )
              )
              ;; fib(n - 2)
              (call $fib
                (i32.add
                  (get_local $n)
                  (i32.const -2)
                )
              )
            )
          )
        )
      )
    )
  )
)
    "#;

#[test]
fn fib_unopt() {
    let translated = translate_wat(FIBONACCI);
    translated.disassemble();

    for x in 0..30 {
        assert_eq!(
            translated.execute_func::<_, u32>(0, (x,)),
            Ok(iterative_fib_baseline(x)),
            "Failed for x={}",
            x
        );
    }
}

// Generated by Rust for the `fib` function in `bench_fibonacci_baseline`
const FIBONACCI_OPT: &str = r"
(module
  (func $fib (param $p0 i32) (result i32)
    (local $l1 i32)
    (set_local $l1
      (i32.const 1))
    (block $B0
      (br_if $B0
        (i32.lt_u
          (get_local $p0)
          (i32.const 2)))
      (set_local $l1
        (i32.const 1))
      (loop $L1
        (set_local $l1
          (i32.add
            (call $fib
              (i32.add
                (get_local $p0)
                (i32.const -1)))
            (get_local $l1)))
        (br_if $L1
          (i32.gt_u
            (tee_local $p0
              (i32.add
                (get_local $p0)
                (i32.const -2)))
            (i32.const 1)))))
    (get_local $l1)))";

#[test]
fn fib_opt() {
    let translated = translate_wat(FIBONACCI_OPT);
    translated.disassemble();

    for x in 0..30 {
        assert_eq!(
            translated.execute_func::<_, u32>(0, (x,)),
            Ok(iterative_fib_baseline(x)),
            "Failed for x={}",
            x
        );
    }
}

#[test]
fn br_table() {
    const CODE: &str = r"
(module
  (func
    (block (br_table 0 0 0 (i32.const 0)) (call $dummy))
  )
  (func
    (block (call $dummy) (br_table 0 0 0 (i32.const 0)) (call $dummy))
  )
  (func
    (block (nop) (call $dummy) (br_table 0 0 0 (i32.const 0)))
  )
  (func $dummy)
)
";

    let translated = translate_wat(CODE);
    translated.disassemble();

    println!("as-block-first");
    assert_eq!(translated.execute_func::<_, ()>(0, ()), Ok(()),);
    println!("as-block-mid");
    assert_eq!(translated.execute_func::<_, ()>(1, ()), Ok(()),);
    println!("as-block-last");
    assert_eq!(translated.execute_func::<_, ()>(2, ()), Ok(()),);
}

#[test]
fn f32_storage() {
    const CODE: &str = r#"
(module
  (memory (data "\00\00\a0\7f"))

  (func (result f32)
    (f32.load (i32.const 0))
  )
  (func (result i32)
    (i32.load (i32.const 0))
  )
  (func
    (f32.store (i32.const 0) (f32.const nan:0x200000))
  )
  (func
    (i32.store (i32.const 0) (i32.const 0x7fa00000))
  )
  (func
    (i32.store (i32.const 0) (i32.const 0))
  )
)
  "#;
    const EXPECTED: u32 = 0x7fa00000;

    let translated = translate_wat(CODE);
    translated.disassemble();

    // TODO: We don't support the data section with Lightbeam's test runtime
    assert!(translated.execute_func::<(), ()>(2, ()).is_ok());

    assert_eq!(translated.execute_func::<(), u32>(1, ()), Ok(EXPECTED));
    assert_eq!(
        translated
            .execute_func::<(), f32>(0, ())
            .map(|f| f.to_bits()),
        Ok(EXPECTED)
    );
    assert!(translated.execute_func::<(), ()>(4, ()).is_ok());
    assert_eq!(translated.execute_func::<(), u32>(1, ()), Ok(0));
    assert_eq!(
        translated
            .execute_func::<(), f32>(0, ())
            .map(|f| f.to_bits()),
        Ok(0)
    );
    assert!(translated.execute_func::<(), ()>(2, ()).is_ok());
    assert_eq!(translated.execute_func::<(), u32>(1, ()), Ok(EXPECTED));
    assert_eq!(
        translated
            .execute_func::<(), f32>(0, ())
            .map(|f| f.to_bits()),
        Ok(EXPECTED)
    );
    assert!(translated.execute_func::<(), ()>(4, ()).is_ok());
    assert_eq!(translated.execute_func::<(), u32>(1, ()), Ok(0));
    assert_eq!(
        translated
            .execute_func::<(), f32>(0, ())
            .map(|f| f.to_bits()),
        Ok(0)
    );
    assert!(translated.execute_func::<(), ()>(3, ()).is_ok());
    assert_eq!(translated.execute_func::<(), u32>(1, ()), Ok(EXPECTED));
    assert_eq!(
        translated
            .execute_func::<(), f32>(0, ())
            .map(|f| f.to_bits()),
        Ok(EXPECTED)
    );
}

#[test]
fn f64_storage() {
    const CODE: &str = r#"
(module
  (memory (data "\00\00\00\00\00\00\f4\7f"))

  (func (export "f64.load") (result f64) (f64.load (i32.const 0)))
  (func (export "i64.load") (result i64) (i64.load (i32.const 0)))
  (func (export "f64.store") (f64.store (i32.const 0) (f64.const nan:0x4000000000000)))
  (func (export "i64.store") (i64.store (i32.const 0) (i64.const 0x7ff4000000000000)))
  (func (export "reset") (i64.store (i32.const 0) (i64.const 0)))
)
  "#;
    const EXPECTED: u64 = 0x7ff4000000000000;

    let translated = translate_wat(CODE);
    translated.disassemble();

    // TODO: We don't support the data section with Lightbeam's test runtime
    assert!(translated.execute_func::<(), ()>(2, ()).is_ok());

    assert_eq!(translated.execute_func::<(), u64>(1, ()), Ok(EXPECTED));
    assert_eq!(
        translated
            .execute_func::<(), f64>(0, ())
            .map(|f| f.to_bits()),
        Ok(EXPECTED)
    );
    assert!(translated.execute_func::<(), ()>(4, ()).is_ok());
    assert_eq!(translated.execute_func::<(), u64>(1, ()), Ok(0));
    assert_eq!(
        translated
            .execute_func::<(), f64>(0, ())
            .map(|f| f.to_bits()),
        Ok(0)
    );
    assert!(translated.execute_func::<(), ()>(2, ()).is_ok());
    assert_eq!(translated.execute_func::<(), u64>(1, ()), Ok(EXPECTED));
    assert_eq!(
        translated
            .execute_func::<(), f64>(0, ())
            .map(|f| f.to_bits()),
        Ok(EXPECTED)
    );
    assert!(translated.execute_func::<(), ()>(4, ()).is_ok());
    assert_eq!(translated.execute_func::<(), u64>(1, ()), Ok(0));
    assert_eq!(
        translated
            .execute_func::<(), f64>(0, ())
            .map(|f| f.to_bits()),
        Ok(0)
    );
    assert!(translated.execute_func::<(), ()>(3, ()).is_ok());
    assert_eq!(translated.execute_func::<(), u64>(1, ()), Ok(EXPECTED));
    assert_eq!(
        translated
            .execute_func::<(), f64>(0, ())
            .map(|f| f.to_bits()),
        Ok(EXPECTED)
    );
}

#[test]
fn storage() {
    const CODE: &str = r#"
(module
  (memory 1 1)

  (func (result i32)
    (local i32 i32 i32)
    (set_local 0 (i32.const 10))
    (block
      (loop
        (if
          (i32.eq (get_local 0) (i32.const 0))
          (then (br 2))
        )
        (set_local 2 (i32.mul (get_local 0) (i32.const 4)))
        (i32.store (get_local 2) (get_local 0))
        (set_local 1 (i32.load (get_local 2)))
        (if
          (i32.ne (get_local 0) (get_local 1))
          (then (return (i32.const 0)))
        )
        (set_local 0 (i32.sub (get_local 0) (i32.const 1)))
        (br 0)
      )
    )
    (i32.const 1)
  )
)"#;

    let translated = translate_wat(CODE);
    translated.disassemble();

    assert_eq!(translated.execute_func::<(), i32>(0, ()), Ok(1));
}

#[test]
fn nested_storage_calls() {
    const CODE: &str = r#"
(module
  (memory 1 1)

  (func (result i32)
    (local i32 i32 i32)
    (set_local 0 (i32.const 10))
    (block
      (loop
        (if
          (i32.eq (get_local 0) (i32.const 0))
          (then (br 2))
        )
        (set_local 2 (i32.mul (get_local 0) (i32.const 4)))
        (call $assert_eq (call $inner) (i32.const 1))
        (i32.store (get_local 2) (get_local 0))
        (set_local 1 (i32.load (get_local 2)))
        (if
          (i32.ne (get_local 0) (get_local 1))
          (then (return (i32.const 0)))
        )
        (set_local 0 (i32.sub (get_local 0) (i32.const 1)))
        (br 0)
      )
    )
    (i32.const 1)
  )

  (func $assert_eq (param $a i32) (param $b i32)
    (if (i32.ne (get_local $a) (get_local $b))
      (unreachable)
    )
  )

  (func $inner (result i32)
    (local i32 i32 i32)
    (set_local 0 (i32.const 10))
    (block
      (loop
        (if
          (i32.eq (get_local 0) (i32.const 0))
          (then (br 2))
        )
        (set_local 2 (i32.mul (get_local 0) (i32.const 4)))
        (i32.store (get_local 2) (get_local 0))
        (set_local 1 (i32.load (get_local 2)))
        (if
          (i32.ne (get_local 0) (get_local 1))
          (then (return (i32.const 0)))
        )
        (set_local 0 (i32.sub (get_local 0) (i32.const 1)))
        (br 0)
      )
    )
    (i32.const 1)
  )
)"#;

    let translated = translate_wat(CODE);
    translated.disassemble();

    assert_eq!(translated.execute_func::<(), i32>(0, ()), Ok(1));
}

// TODO: Signature mismatches correctly fail at time of writing this comment,
//       but we can't add a test for that until we implement traps properly.
#[test]
fn call_indirect() {
    const CODE: &str = r#"
(module
  (type $over-i64 (func (param i64) (result i64)))

  (table anyfunc
    (elem
      $fac $fib
    )
  )

  (func $dispatch (param i32 i64) (result i64)
    (call_indirect (type $over-i64) (get_local 1) (get_local 0))
  )

  (func $fac (type $over-i64)
    (if (result i64) (i64.eqz (get_local 0))
      (then (i64.const 1))
      (else
        (i64.mul
          (get_local 0)
          (call_indirect (type $over-i64)
            (i64.sub (get_local 0) (i64.const 1))
            (i32.const 0)
          )
        )
      )
    )
  )

  (func $fib (type $over-i64)
    (if (result i64) (i64.le_u (get_local 0) (i64.const 1))
      (then (i64.const 1))
      (else
        (i64.add
          (call_indirect (type $over-i64)
            (i64.sub (get_local 0) (i64.const 2))
            (i32.const 1)
          )
          (call_indirect (type $over-i64)
            (i64.sub (get_local 0) (i64.const 1))
            (i32.const 1)
          )
        )
      )
    )
  )
)"#;

    let wasm = wabt::wat2wasm(CODE).unwrap();
    let module = translate(&wasm).unwrap();

    module.disassemble();

    assert_eq!(
        module.execute_func::<(i32, i64), i64>(0, (0, 10)).unwrap(),
        3628800
    );
    assert_eq!(
        module.execute_func::<(i32, i64), i64>(0, (1, 10)).unwrap(),
        89
    );
}

macro_rules! test_select {
    ($name:ident, $ty:ident) => {
        mod $name {
            use super::{translate_wat, ExecutableModule};
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

#[bench]
fn bench_fibonacci_compile(b: &mut test::Bencher) {
    let wasm = wabt::wat2wasm(FIBONACCI).unwrap();

    b.iter(|| test::black_box(translate(&wasm).unwrap()));
}

#[bench]
fn bench_fibonacci_run(b: &mut test::Bencher) {
    let wasm = wabt::wat2wasm(FIBONACCI_OPT).unwrap();
    let module = translate(&wasm).unwrap();

    b.iter(|| module.execute_func::<_, u32>(0, (20,)));
}

#[bench]
fn bench_fibonacci_compile_run(b: &mut test::Bencher) {
    let wasm = wabt::wat2wasm(FIBONACCI).unwrap();

    b.iter(|| translate(&wasm).unwrap().execute_func::<_, u32>(0, (20,)));
}

#[bench]
fn bench_fibonacci_baseline(b: &mut test::Bencher) {
    fn fib(n: i32) -> i32 {
        if n == 0 || n == 1 {
            1
        } else {
            fib(n - 1) + fib(n - 2)
        }
    }

    b.iter(|| test::black_box(fib(test::black_box(20))));
}

// #[test]
#[allow(dead_code)]
fn sieve() {
    const CODE: &str = r#"
(module
  (type $t0 (func (param i32 i32)))
  (type $t1 (func (param i32 i32 i32) (result i32)))
  (type $t2 (func (param i32 i32 i32)))
  (type $t3 (func (param i32)))
  (type $t4 (func (param i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)))
  (type $t5 (func (param i32 i32) (result i32)))
  (func $optimized_sieve (export "optimized_sieve") (type $t0) (param $p0 i32) (param $p1 i32)
    (local $l0 i32) (local $l1 i32) (local $l2 i32) (local $l3 i32) (local $l4 i32) (local $l5 i32) (local $l6 i32) (local $l7 i32) (local $l8 i32) (local $l9 i32) (local $l10 i32) (local $l11 i32) (local $l12 i32) (local $l13 i32) (local $l14 i32) (local $l15 i32) (local $l16 i32) (local $l17 i32) (local $l18 f64)
    get_global $g0
    i32.const 64
    i32.sub
    tee_local $l0
    set_global $g0
    i32.const 0
    set_local $l1
    get_local $l0
    i32.const 0
    i32.const 64
    call $memset
    set_local $l0
    block $B0
      block $B1
        block $B2
          block $B3
            get_local $p1
            i32.const 2
            i32.gt_u
            br_if $B3
            i32.const -1
            i32.const 0
            get_local $p1
            i32.const 2
            i32.eq
            select
            set_local $p1
            i32.const 0
            set_local $l2
            i32.const 0
            set_local $l3
            i32.const 0
            set_local $l4
            i32.const 0
            set_local $l5
            i32.const 0
            set_local $l6
            i32.const 0
            set_local $l7
            i32.const 0
            set_local $l8
            i32.const 0
            set_local $l9
            i32.const 0
            set_local $l10
            i32.const 0
            set_local $l11
            i32.const 0
            set_local $l12
            i32.const 0
            set_local $l13
            i32.const 0
            set_local $l14
            i32.const 0
            set_local $l15
            i32.const 0
            set_local $l16
            i32.const 0
            set_local $l17
            br $B2
          end
          get_local $p1
          i32.const -3
          i32.add
          i32.const 1
          i32.shr_u
          set_local $l3
          block $B4
            block $B5
              get_local $p1
              f64.convert_u/i32
              f64.sqrt
              tee_local $l18
              f64.const 0x1p+32 (;=4.29497e+09;)
              f64.lt
              get_local $l18
              f64.const 0x0p+0 (;=0;)
              f64.ge
              i32.and
              br_if $B5
              i32.const 0
              set_local $p1
              br $B4
            end
            get_local $l18
            i32.trunc_u/f64
            set_local $p1
          end
          get_local $l3
          i32.const 1
          i32.add
          set_local $l17
          get_local $p1
          i32.const -3
          i32.add
          i32.const 1
          i32.shr_u
          set_local $l5
          i32.const 0
          set_local $p1
          loop $L6
            get_local $p1
            tee_local $l4
            i32.const 5
            i32.shr_u
            set_local $p1
            get_local $l4
            i32.const 511
            i32.gt_u
            br_if $B0
            block $B7
              get_local $l0
              get_local $p1
              i32.const 2
              i32.shl
              i32.add
              i32.load
              i32.const 1
              get_local $l4
              i32.const 31
              i32.and
              i32.shl
              i32.and
              br_if $B7
              get_local $l4
              i32.const 1
              i32.shl
              i32.const 3
              i32.add
              tee_local $l2
              get_local $l2
              i32.mul
              i32.const -3
              i32.add
              i32.const 1
              i32.shr_u
              tee_local $p1
              get_local $l3
              i32.gt_u
              br_if $B7
              loop $L8
                get_local $p1
                i32.const 5
                i32.shr_u
                set_local $l1
                get_local $p1
                i32.const 511
                i32.gt_u
                br_if $B1
                get_local $l0
                get_local $l1
                i32.const 2
                i32.shl
                i32.add
                tee_local $l1
                get_local $l1
                i32.load
                i32.const 1
                get_local $p1
                i32.const 31
                i32.and
                i32.shl
                i32.or
                i32.store
                get_local $p1
                get_local $l2
                i32.add
                tee_local $p1
                get_local $l3
                i32.le_u
                br_if $L8
              end
            end
            get_local $l4
            i32.const 1
            i32.add
            set_local $p1
            get_local $l4
            get_local $l5
            i32.lt_u
            br_if $L6
          end
          i32.const -1
          set_local $p1
          get_local $l0
          i32.load offset=60
          set_local $l1
          get_local $l0
          i32.load offset=56
          set_local $l2
          get_local $l0
          i32.load offset=52
          set_local $l3
          get_local $l0
          i32.load offset=48
          set_local $l4
          get_local $l0
          i32.load offset=44
          set_local $l5
          get_local $l0
          i32.load offset=40
          set_local $l6
          get_local $l0
          i32.load offset=36
          set_local $l7
          get_local $l0
          i32.load offset=32
          set_local $l8
          get_local $l0
          i32.load offset=28
          set_local $l9
          get_local $l0
          i32.load offset=24
          set_local $l10
          get_local $l0
          i32.load offset=20
          set_local $l11
          get_local $l0
          i32.load offset=16
          set_local $l12
          get_local $l0
          i32.load offset=12
          set_local $l13
          get_local $l0
          i32.load offset=8
          set_local $l14
          get_local $l0
          i32.load offset=4
          set_local $l15
          get_local $l0
          i32.load
          set_local $l16
        end
        get_local $p0
        get_local $l1
        i32.store offset=68
        get_local $p0
        get_local $l2
        i32.store offset=64
        get_local $p0
        get_local $l3
        i32.store offset=60
        get_local $p0
        get_local $l4
        i32.store offset=56
        get_local $p0
        get_local $l5
        i32.store offset=52
        get_local $p0
        get_local $l6
        i32.store offset=48
        get_local $p0
        get_local $l7
        i32.store offset=44
        get_local $p0
        get_local $l8
        i32.store offset=40
        get_local $p0
        get_local $l9
        i32.store offset=36
        get_local $p0
        get_local $l10
        i32.store offset=32
        get_local $p0
        get_local $l11
        i32.store offset=28
        get_local $p0
        get_local $l12
        i32.store offset=24
        get_local $p0
        get_local $l13
        i32.store offset=20
        get_local $p0
        get_local $l14
        i32.store offset=16
        get_local $p0
        get_local $l15
        i32.store offset=12
        get_local $p0
        get_local $l16
        i32.store offset=8
        get_local $p0
        get_local $l17
        i32.store offset=4
        get_local $p0
        get_local $p1
        i32.store
        get_local $l0
        i32.const 64
        i32.add
        set_global $g0
        return
      end
      i32.const 1396
      get_local $l1
      i32.const 16
      unreachable
    end
    i32.const 1380
    get_local $p1
    i32.const 16
    unreachable)
  (func $memset (type $t1) (param $p0 i32) (param $p1 i32) (param $p2 i32) (result i32)
    (local $l0 i32)
    block $B0
      get_local $p2
      i32.eqz
      br_if $B0
      get_local $p0
      set_local $l0
      loop $L1
        get_local $l0
        get_local $p1
        i32.store8
        get_local $l0
        i32.const 1
        i32.add
        set_local $l0
        get_local $p2
        i32.const -1
        i32.add
        tee_local $p2
        br_if $L1
      end
    end
    get_local $p0)
  (memory $memory (export "memory") 17 17)
  (global $g0 (mut i32) (i32.const 1050032)))
"#;

    translate(&wabt::wat2wasm(CODE).unwrap()).unwrap();
}

