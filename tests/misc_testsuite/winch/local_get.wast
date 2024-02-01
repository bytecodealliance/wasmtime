;; Test `local.get` operator

(module
  ;; Typing

  (func (export "type-local-i32") (result i32) (local i32) (local.get 0))
  (func (export "type-local-i64") (result i64) (local i64) (local.get 0))
  (func (export "type-local-f32") (result f32) (local f32) (local.get 0))
  (func (export "type-local-f64") (result f64) (local f64) (local.get 0))

  (func (export "type-param-i32") (param i32) (result i32) (local.get 0))
  (func (export "type-param-i64") (param i64) (result i64) (local.get 0))
  (func (export "type-param-f32") (param f32) (result f32) (local.get 0))
  (func (export "type-param-f64") (param f64) (result f64) (local.get 0))

  (func (export "type-mixed") (param i64 f32 f64 i32 i32)
    (local f32 i64 i64 f64)
    (drop (i64.eqz (local.get 0)))
    (drop (f32.neg (local.get 1)))
    (drop (f64.neg (local.get 2)))
    (drop (i32.eqz (local.get 3)))
    (drop (i32.eqz (local.get 4)))
    (drop (f32.neg (local.get 5)))
    (drop (i64.eqz (local.get 6)))
    (drop (i64.eqz (local.get 7)))
    (drop (f64.neg (local.get 8)))
  )


  ;; As parameter of control constructs and instructions

  (func (export "as-block-value") (param i32) (result i32)
    (block (result i32) (local.get 0))
  )
  (func (export "as-loop-value") (param i32) (result i32)
    (loop (result i32) (local.get 0))
  )
  (func (export "as-br-value") (param i32) (result i32)
    (block (result i32) (br 0 (local.get 0)))
  )
  (func (export "as-br_if-value") (param i32) (result i32)
    (block $l0 (result i32) (br_if $l0 (local.get 0) (i32.const 1)))
  )

  (func (export "as-br_if-value-cond") (param i32) (result i32)
    (block (result i32)
      (br_if 0 (local.get 0) (local.get 0))
    )
  )
  (func (export "as-br_table-value") (param i32) (result i32)
    (block
      (block
        (block
          (br_table 0 1 2 (local.get 0))
          (return (i32.const 0))
        )
        (return (i32.const 1))
      )
      (return (i32.const 2))
    )
    (i32.const 3)
  )

  (func (export "as-return-value") (param i32) (result i32)
    (return (local.get 0))
  )

  (func (export "as-if-then") (param i32) (result i32)
    (if (result i32) (local.get 0) (then (local.get 0)) (else (i32.const 0)))
  )
  (func (export "as-if-else") (param i32) (result i32)
    (if (result i32) (local.get 0) (then (i32.const 1)) (else (local.get 0)))
  )
)

(assert_return (invoke "type-local-i32") (i32.const 0))
(assert_return (invoke "type-local-i64") (i64.const 0))
(assert_return (invoke "type-local-f32") (f32.const 0))
(assert_return (invoke "type-local-f64") (f64.const 0))

(assert_return (invoke "type-param-i32" (i32.const 2)) (i32.const 2))
(assert_return (invoke "type-param-i64" (i64.const 3)) (i64.const 3))
(assert_return (invoke "type-param-f32" (f32.const 4.4)) (f32.const 4.4))
(assert_return (invoke "type-param-f64" (f64.const 5.5)) (f64.const 5.5))

(assert_return (invoke "as-block-value" (i32.const 6)) (i32.const 6))
(assert_return (invoke "as-loop-value" (i32.const 7)) (i32.const 7))

(assert_return (invoke "as-br-value" (i32.const 8)) (i32.const 8))
(assert_return (invoke "as-br_if-value" (i32.const 9)) (i32.const 9))
(assert_return (invoke "as-br_if-value-cond" (i32.const 10)) (i32.const 10))
(assert_return (invoke "as-br_table-value" (i32.const 1)) (i32.const 2))

(assert_return (invoke "as-return-value" (i32.const 0)) (i32.const 0))

(assert_return (invoke "as-if-then" (i32.const 1)) (i32.const 1))
(assert_return (invoke "as-if-else" (i32.const 0)) (i32.const 0))

(assert_return
  (invoke "type-mixed"
    (i64.const 1) (f32.const 2.2) (f64.const 3.3) (i32.const 4) (i32.const 5)
  )
)
