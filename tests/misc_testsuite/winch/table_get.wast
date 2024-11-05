;;! reference_types = true

(module
  (table $t3 3 funcref)
  (elem (table $t3) (i32.const 1) func $dummy)
  (func $dummy)
  (func $f3 (export "get-funcref") (param $i i32) (result funcref)
    (table.get $t3 (local.get $i))
  )
)

(assert_return (invoke "get-funcref" (i32.const 0)) (ref.null func))
(assert_trap (invoke "get-funcref" (i32.const 3)) "out of bounds table access")
(assert_trap (invoke "get-funcref" (i32.const -1)) "out of bounds table access")

