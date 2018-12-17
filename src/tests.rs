use super::{translate, TranslatedModule};
use wabt;

fn translate_wat(wat: &str) -> TranslatedModule {
    let wasm = wabt::wat2wasm(wat).unwrap();
    let compiled = translate(&wasm).unwrap();
    compiled
}

/// Execute the first function in the module.
fn execute_wat(wat: &str, a: u32, b: u32) -> u32 {
    let translated = translate_wat(wat);
    unsafe { translated.execute_func(0, (a, b)) }
}

#[test]
fn empty() {
    let _ = translate_wat("(module (func))");
}

macro_rules! binop_test {
    ($op:ident, $func:expr) => {
        quickcheck! {
            fn $op(a: u32, b: u32) -> bool {
                static CODE: &str = concat!(
                    "(module (func (param i32) (param i32) (result i32) (i32.",
                    stringify!($op),
                    " (get_local 0) (get_local 1))))"
                );

                lazy_static! {
                    static ref TRANSLATED: TranslatedModule = translate_wat(CODE);
                }

                unsafe { TRANSLATED.execute_func::<(u32, u32), u32>(0, (a, b)) == $func(a, b) }
            }
        }
    };
}

binop_test!(add, u32::wrapping_add);
binop_test!(sub, u32::wrapping_sub);
binop_test!(and, std::ops::BitAnd::bitand);
binop_test!(or, std::ops::BitOr::bitor);
binop_test!(xor, std::ops::BitXor::bitxor);
binop_test!(mul, u32::wrapping_mul);

quickcheck! {
    fn relop_eq(a: u32, b: u32) -> bool{
        static CODE: &str = r#"
            (module
              (func (param i32) (param i32) (result i32) (i32.eq (get_local 0) (get_local 1)))
            )
        "#;

        lazy_static! {
            static ref TRANSLATED: TranslatedModule = translate_wat(CODE);
        }

        let out = unsafe { TRANSLATED.execute_func::<(u32, u32), u32>(0, (a, b)) };

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
            static ref TRANSLATED: TranslatedModule = translate_wat(CODE);
        }

        let out = unsafe { TRANSLATED.execute_func::<(u32, u32), u32>(0, (a, b)) };

        out == (if a == b { a } else { b })
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
            let out: u32 = unsafe { translated.execute_func(0, (5, 4, 3, 2, 1, 0)) };
            out
        },
        5
    );
}

#[test]
fn function_read_args_spill_to_stack() {
    let code = r#"
(module
  (func (param i32) (param i32) (param i32) (param i32)
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
            let out: u32 = unsafe {
                translated.execute_func(0, (7u32, 6u32, 5u32, 4u32, 3u32, 2u32, 1u32, 0u32))
            };
            out
        },
        7
    );
}

#[test]
fn function_write_args_spill_to_stack() {
    let code = r#"
(module
  (func (param i32) (param i32) (param i32) (param i32)
        (param i32) (param i32) (param i32) (param i32)
        (param i32) (param i32) (param i32) (param i32)
        (result i32)

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
        (param i32) (param i32) (param i32) (param i32)
        (param i32) (param i32) (param i32) (param i32)
        (param i32) (param i32) (param i32) (param i32)
        (result i32)

    (call $assert_zero
      (get_local 11)
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
            let out: u32 =
                unsafe { translated.execute_func(0, (11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0)) };
            out
        },
        11
    );
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
        unsafe {
            assert_eq!(translated.execute_func::<_, u32>(0, (x,)), fib(x));
        }
    }
}

#[bench]
fn bench_compile(b: &mut test::Bencher) {
    let wasm = wabt::wat2wasm(FIBONACCI).unwrap();

    b.iter(|| test::black_box(translate(&wasm).unwrap()));
}

#[bench]
fn bench_run(b: &mut test::Bencher) {
    let wasm = wabt::wat2wasm(FIBONACCI).unwrap();
    let module = translate(&wasm).unwrap();

    b.iter(|| unsafe { module.execute_func::<_, u32>(0, (20,)) });
}
