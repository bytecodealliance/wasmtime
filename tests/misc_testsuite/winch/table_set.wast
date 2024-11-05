;;! reference_types = true

(module
  (table $t3 2 funcref)
  (elem (table $t3) (i32.const 1) func $dummy)
  (func $dummy)

  (func $f3 (export "get-funcref") (param $i i32) (result funcref)
    (table.get $t3 (local.get $i))
  )

  (func (export "set-funcref") (param $i i32) (param $r funcref)
    (table.set $t3 (local.get $i) (local.get $r))
  )
  (func (export "set-funcref-from") (param $i i32) (param $j i32)
    (table.set $t3 (local.get $i) (table.get $t3 (local.get $j)))
  )
)

(assert_return (invoke "get-funcref" (i32.const 0)) (ref.null func))
(assert_return (invoke "set-funcref-from" (i32.const 0) (i32.const 1)))
(assert_return (invoke "set-funcref" (i32.const 0) (ref.null func)))
(assert_return (invoke "get-funcref" (i32.const 0)) (ref.null func))

(assert_trap (invoke "set-funcref" (i32.const 3) (ref.null func)) "out of bounds table access")
(assert_trap (invoke "set-funcref" (i32.const -1) (ref.null func)) "out of bounds table access")

(assert_trap (invoke "set-funcref-from" (i32.const 3) (i32.const 1)) "out of bounds table access")
(assert_trap (invoke "set-funcref-from" (i32.const -1) (i32.const 1)) "out of bounds table access")
