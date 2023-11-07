;; Test `local.set` operator

(module
  ;; Typing

  (func (export "type-local-i32") (local i32) (local.set 0 (i32.const 0)))
  (func (export "type-local-i64") (local i64) (local.set 0 (i64.const 0)))
  (func (export "type-local-f32") (local f32) (local.set 0 (f32.const 0)))
  (func (export "type-local-f64") (local f64) (local.set 0 (f64.const 0)))

  (func (export "type-param-i32") (param i32) (local.set 0 (i32.const 10)))
  (func (export "type-param-i64") (param i64) (local.set 0 (i64.const 11)))
  (func (export "type-param-f32") (param f32) (local.set 0 (f32.const 11.1)))
  (func (export "type-param-f64") (param f64) (local.set 0 (f64.const 12.2)))

  (func (export "type-mixed") (param i64 f32 f64 i32 i32) (local f32 i64 i64 f64)
    (local.set 0 (i64.const 0))
    (local.set 1 (f32.const 0))
    (local.set 2 (f64.const 0))
    (local.set 3 (i32.const 0))
    (local.set 4 (i32.const 0))
    (local.set 5 (f32.const 0))
    (local.set 6 (i64.const 0))
    (local.set 7 (i64.const 0))
    (local.set 8 (f64.const 0))
  )

  ;; As parameter of control constructs and instructions

  (func (export "as-block-value") (param i32)
    (block (local.set 0 (i32.const 1)))
  )
  (func (export "as-loop-value") (param i32)
    (loop (local.set 0 (i32.const 3)))
  )

  (func (export "as-br-value") (param i32)
    (block (br 0 (local.set 0 (i32.const 9))))
  )
  (func (export "as-br_if-value") (param i32)
    (block
      (br_if 0 (local.set 0 (i32.const 8)) (i32.const 1))
    )
  )
  (func (export "as-br_if-value-cond") (param i32)
    (block
      (br_if 0 (i32.const 6) (local.set 0 (i32.const 9)))
    )
  )
  (func (export "as-br_table-value") (param i32)
    (block
      (br_table 0 (local.set 0 (i32.const 10)) (i32.const 1))
    )
  )

  (func (export "as-return-value") (param i32)
    (return (local.set 0 (i32.const 7)))
  )

  (func (export "as-if-then") (param i32)
    (if (local.get 0) (then (local.set 0 (i32.const 3))))
  )
  (func (export "as-if-else") (param i32)
    (if (local.get 0) (then) (else (local.set 0 (i32.const 1))))
  )
)

(assert_return (invoke "type-local-i32"))
(assert_return (invoke "type-local-i64"))
(assert_return (invoke "type-local-f32"))
(assert_return (invoke "type-local-f64"))

(assert_return (invoke "type-param-i32" (i32.const 2)))
(assert_return (invoke "type-param-i64" (i64.const 3)))
(assert_return (invoke "type-param-f32" (f32.const 4.4)))
(assert_return (invoke "type-param-f64" (f64.const 5.5)))

(assert_return (invoke "as-block-value" (i32.const 0)))
(assert_return (invoke "as-loop-value" (i32.const 0)))

(assert_return (invoke "as-br-value" (i32.const 0)))
(assert_return (invoke "as-br_if-value" (i32.const 0)))
(assert_return (invoke "as-br_if-value-cond" (i32.const 0)))
(assert_return (invoke "as-br_table-value" (i32.const 0)))

(assert_return (invoke "as-return-value" (i32.const 0)))

(assert_return (invoke "as-if-then" (i32.const 1)))
(assert_return (invoke "as-if-else" (i32.const 0)))

(assert_return
  (invoke "type-mixed"
    (i64.const 1) (f32.const 2.2) (f64.const 3.3) (i32.const 4) (i32.const 5)
  )
)


