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
                        "
                        (module (func (param i32) (param i32) (result i32)
                            (i32.{op} (get_local 0) (get_local 1))))
                    ",
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
                        translate_wat(&format!("
                            (module (func (param i64) (result {retty})
                                (i64.{op} (get_local 0) (i64.const {right}))))
                        ", retty = RETTY, op = OP, right = b)).execute_func::<(i64,), $retty>(0, (a,)) == Ok($func(a, b) as $retty)
                    }
                }
            }
        };
    }

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
}

quickcheck! {
    fn relop_eq(a: u32, b: u32) -> bool {
        static CODE: &str = r#"
            (module
              (func (param i32) (param i32) (result i32) (i32.eq (get_local 0) (get_local 1)))
            )
        "#;

        lazy_static! {
            static ref TRANSLATED: ExecutableModule = translate_wat(CODE);
        }

        let out = TRANSLATED.execute_func::<(u32, u32), u32>(0, (a, b)).unwrap();

        (a == b) == (out == 1)
    }
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
            static ref TRANSLATED: ExecutableModule = translate_wat(CODE);
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

        assert_eq!(TRANSLATED.execute_func::<(i32,), i32>(0, (n,)), Ok(fac(n)));
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

#[test]
fn literals() {
    let code = r#"
(module
  (func (param i32) (param i32) (result i32)
    (i32.const 228)
  )
)
    "#;

    assert_eq!(execute_wat(code, 0, 0), 228);
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
fn fib() {
    fn fib(n: u32) -> u32 {
        let (mut a, mut b) = (1, 1);

        for _ in 0..n {
            let old_a = a;
            a = b;
            b += old_a;
        }

        a
    }

    let translated = translate_wat(FIBONACCI);
    translated.disassemble();

    for x in 0..30 {
        assert_eq!(
            translated.execute_func::<_, u32>(0, (x,)),
            Ok(fib(x)),
            "Failed for x={}",
            x
        );
    }
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

#[test]
fn call_indirect() {
    const CODE: &str = r#"
(module
  (type $over-i64 (func (param i64) (result i64)))

  (table anyfunc
    (elem
      $dispatch $fac $fib
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
            (i32.const 1)
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
            (i32.const 2)
          )
          (call_indirect (type $over-i64)
            (i64.sub (get_local 0) (i64.const 1))
            (i32.const 2)
          )
        )
      )
    )
  )
)"#;

    let wasm = wabt::wat2wasm(CODE).unwrap();
    let module = translate(&wasm).unwrap();

    module.disassemble();

    assert_eq!(module.execute_func::<(i32, i64), i64>(0, (1, 10)).unwrap(), 3628800);
    assert_eq!(module.execute_func::<(i32, i64), i64>(0, (2, 10)).unwrap(), 89);
}

#[bench]
fn bench_fibonacci_compile(b: &mut test::Bencher) {
    let wasm = wabt::wat2wasm(FIBONACCI).unwrap();

    b.iter(|| test::black_box(translate(&wasm).unwrap()));
}

#[bench]
fn bench_fibonacci_run(b: &mut test::Bencher) {
    let wasm = wabt::wat2wasm(FIBONACCI).unwrap();
    let module = translate(&wasm).unwrap();

    b.iter(|| module.execute_func::<_, u32>(0, (20,)));
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
