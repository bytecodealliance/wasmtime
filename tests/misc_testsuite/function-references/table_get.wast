;;! gc = true

(module
  (type $res-i32 (func (result i32)))
  (table $t2 2 externref)
  (table $t3 3 funcref)
  (table $t4 (ref null $res-i32) (elem (ref.func $returns-five)))
  (elem (table $t3) (i32.const 1) func $returns-five)
  (func $returns-five (result i32) (i32.const 5))

  (func (export "init") (param $r externref)
    (table.set $t2 (i32.const 1) (local.get $r))
    (table.set $t3 (i32.const 2) (table.get $t3 (i32.const 1)))
  )

  (func (export "get-externref") (param $i i32) (result externref)
    (table.get $t2 (local.get $i))
  )
  (func $f3 (export "get-funcref") (param $i i32) (result funcref)
    (table.get $t3 (local.get $i))
  )
  (func $f4 (export "get-typed-func") (param $i i32) (result (ref $res-i32))
    (ref.as_non_null (table.get $t4 (local.get $i)))
  )

  (func (export "is_null-funcref") (param $i i32) (result i32)
    (ref.is_null (call $f3 (local.get $i)))
  )
  (func (export "get-typed-and-call") (param $i i32) (result i32) (call_ref $res-i32 (call $f4 (local.get $i))))
)

(invoke "init" (ref.extern 1))

(assert_return (invoke "get-externref" (i32.const 0)) (ref.null extern))
(assert_return (invoke "get-externref" (i32.const 1)) (ref.extern 1))

(assert_return (invoke "get-funcref" (i32.const 0)) (ref.null func))
(assert_return (invoke "is_null-funcref" (i32.const 1)) (i32.const 0))
(assert_return (invoke "is_null-funcref" (i32.const 2)) (i32.const 0))

(assert_return (invoke "get-typed-and-call" (i32.const 0)) (i32.const 5))

(assert_trap (invoke "get-externref" (i32.const 2)) "out of bounds table access")
(assert_trap (invoke "get-funcref" (i32.const 3)) "out of bounds table access")
(assert_trap (invoke "get-typed-func" (i32.const 2)) "out of bounds table access")
(assert_trap (invoke "get-externref" (i32.const -1)) "out of bounds table access")
(assert_trap (invoke "get-funcref" (i32.const -1)) "out of bounds table access")
(assert_trap (invoke "get-typed-func" (i32.const -1)) "out of bounds table access")


;; Type errors

(assert_invalid
  (module
    (table $t 10 externref)
    (func $type-index-empty-vs-i32 (result externref)
      (table.get $t)
    )
  )
  "type mismatch"
)
(assert_invalid
  (module
    (table $t 10 externref)
    (func $type-index-f32-vs-i32 (result externref)
      (table.get $t (f32.const 1))
    )
  )
  "type mismatch"
)

(assert_invalid
  (module
    (table $t 10 externref)
    (func $type-result-externref-vs-empty
      (table.get $t (i32.const 0))
    )
  )
  "type mismatch"
)
(assert_invalid
  (module
    (table $t 10 externref)
    (func $type-result-externref-vs-funcref (result funcref)
      (table.get $t (i32.const 1))
    )
  )
  "type mismatch"
)

(assert_invalid
  (module
    (table $t1 1 funcref)
    (table $t2 1 externref)
    (func $type-result-externref-vs-funcref-multi (result funcref)
      (table.get $t2 (i32.const 0))
    )
  )
  "type mismatch"
)
