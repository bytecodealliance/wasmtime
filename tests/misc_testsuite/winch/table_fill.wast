;;! reference_types = true

(module
  (type $t0 (func))
  (func $f1 (type $t0))
  (func $f2 (type $t0))
  (func $f3 (type $t0))

  ;; Define two tables of funcref
  (table $t1 3 funcref)
  (table $t2 10 funcref)

  ;; Initialize table $t1 with functions $f1, $f2, $f3
  (elem (i32.const 0) $f1 $f2 $f3)

  ;; Function to fill table $t1 using a function reference from table $t2
  (func (export "fill") (param $i i32) (param $r i32) (param $n i32)
    (local $ref funcref)
    (local.set $ref (table.get $t1 (local.get $r)))
    (table.fill $t2 (local.get $i) (local.get $ref) (local.get $n))
  )

  (func (export "get") (param $i i32) (result funcref)
    (table.get $t2 (local.get $i))
  )
)

(assert_return (invoke "get" (i32.const 1)) (ref.null func))
(assert_return (invoke "get" (i32.const 2)) (ref.null func))
(assert_return (invoke "get" (i32.const 3)) (ref.null func))
(assert_return (invoke "get" (i32.const 4)) (ref.null func))
(assert_return (invoke "get" (i32.const 5)) (ref.null func))

(assert_return (invoke "fill" (i32.const 2) (i32.const 0) (i32.const 3)))
(assert_return (invoke "get" (i32.const 1)) (ref.null func))
(assert_return (invoke "get" (i32.const 2)) (ref.func 0))
(assert_return (invoke "get" (i32.const 3)) (ref.func 0))
(assert_return (invoke "get" (i32.const 4)) (ref.func 0))
(assert_return (invoke "get" (i32.const 5)) (ref.null func))

(assert_return (invoke "fill" (i32.const 4) (i32.const 1) (i32.const 2)))
(assert_return (invoke "get" (i32.const 3)) (ref.func 0))
(assert_return (invoke "get" (i32.const 4)) (ref.func 1))
(assert_return (invoke "get" (i32.const 5)) (ref.func 1))
(assert_return (invoke "get" (i32.const 6)) (ref.null func))

(assert_return (invoke "fill" (i32.const 4) (i32.const 2) (i32.const 0)))
(assert_return (invoke "get" (i32.const 3)) (ref.func 0))
(assert_return (invoke "get" (i32.const 4)) (ref.func 1))
(assert_return (invoke "get" (i32.const 5)) (ref.func 1))

(assert_return (invoke "fill" (i32.const 8) (i32.const 0) (i32.const 2)))
(assert_return (invoke "get" (i32.const 7)) (ref.null func))
(assert_return (invoke "get" (i32.const 8)) (ref.func 0))
(assert_return (invoke "get" (i32.const 9)) (ref.func 0))

(assert_return (invoke "fill" (i32.const 9) (i32.const 2) (i32.const 1)))
(assert_return (invoke "get" (i32.const 8)) (ref.func 0))
(assert_return (invoke "get" (i32.const 9)) (ref.func 2))

(assert_return (invoke "fill" (i32.const 10) (i32.const 1) (i32.const 0)))
(assert_return (invoke "get" (i32.const 9)) (ref.func 2))

(assert_trap
  (invoke "fill" (i32.const 8) (i32.const 0) (i32.const 3))
  "out of bounds table access"
)
