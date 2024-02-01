(module $m
  (func $f (param i32 i32 i32 i32 i32 i32) (result i32) (local.get 0))
  (func $g (param i32 i32 i32 i32 i32 i32) (result i32) (local.get 1))
  (func $h (param i32 i32 i32 i32 i32 i32) (result i32) (local.get 2))

  (table $t (export "t") funcref (elem $f $g $h $f $g $h)))

(register "m" $m)

(module $n
  (table $t (import "m" "t") 6 funcref)

  (func $i (param i32 i32 i32 i32 i32 i32) (result i32) (local.get 3))
  (func $j (param i32 i32 i32 i32 i32 i32) (result i32) (local.get 4))
  (func $k (param i32 i32 i32 i32 i32 i32) (result i32) (local.get 5))

  (table $u (export "u") funcref (elem $i $j $k $i $j $k))

  (func (export "copy_into_t_from_u") (param i32 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    table.copy $t $u)

  (func (export "copy_into_u_from_t") (param i32 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    table.copy $u $t)

  (func (export "call_t") (param i32 i32 i32 i32 i32 i32 i32) (result i32)
    local.get 0
    local.get 1
    local.get 2
    local.get 3
    local.get 4
    local.get 5
    local.get 6
    call_indirect $t (param i32 i32 i32 i32 i32 i32) (result i32))

  (func (export "call_u") (param i32 i32 i32 i32 i32 i32 i32) (result i32)
    local.get 0
    local.get 1
    local.get 2
    local.get 3
    local.get 4
    local.get 5
    local.get 6
    call_indirect $u (param i32 i32 i32 i32 i32 i32) (result i32)))

;; Everything has what we initially expect.
(assert_return
  (invoke "call_t" (i32.const 1) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
    (i32.const 0))
  (i32.const 1))
(assert_return
  (invoke "call_t" (i32.const 0) (i32.const 1) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
    (i32.const 1))
  (i32.const 1))
(assert_return
  (invoke "call_t" (i32.const 0) (i32.const 0) (i32.const 1) (i32.const 0) (i32.const 0) (i32.const 0)
    (i32.const 2))
  (i32.const 1))
(assert_return
  (invoke "call_u" (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 1) (i32.const 0) (i32.const 0)
    (i32.const 0))
  (i32.const 1))
(assert_return
  (invoke "call_u" (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 1) (i32.const 0)
    (i32.const 1))
  (i32.const 1))
(assert_return
  (invoke "call_u" (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 1)
    (i32.const 2))
  (i32.const 1))

;; Now test copying between a local and an imported table.

;; Copy $i $j $k into $t at 3..6 from $u at 0..3.
(invoke "copy_into_t_from_u" (i32.const 3) (i32.const 0) (i32.const 3))

(assert_return
  (invoke "call_t" (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 1) (i32.const 0) (i32.const 0)
    (i32.const 3))
  (i32.const 1))
(assert_return
  (invoke "call_t" (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 1) (i32.const 0)
    (i32.const 4))
  (i32.const 1))
(assert_return
  (invoke "call_t" (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 1)
    (i32.const 5))
  (i32.const 1))

;; Copy $f $g $h into $u at 0..3 from $t at 0..3.
(invoke "copy_into_u_from_t" (i32.const 0) (i32.const 0) (i32.const 3))

(assert_return
  (invoke "call_u" (i32.const 1) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
    (i32.const 0))
  (i32.const 1))
(assert_return
  (invoke "call_u" (i32.const 0) (i32.const 1) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
    (i32.const 1))
  (i32.const 1))
(assert_return
  (invoke "call_u" (i32.const 0) (i32.const 0) (i32.const 1) (i32.const 0) (i32.const 0) (i32.const 0)
    (i32.const 2))
  (i32.const 1))

(register "n" $n)

(module $o
  (table $t (import "m" "t") 6 funcref)
  (table $u (import "n" "u") 6 funcref)

  (func (export "copy_into_t_from_u_2") (param i32 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    table.copy $t $u)

  (func (export "copy_into_u_from_t_2") (param i32 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    table.copy $u $t)

  (func (export "call_t_2") (param i32 i32 i32 i32 i32 i32 i32) (result i32)
    local.get 0
    local.get 1
    local.get 2
    local.get 3
    local.get 4
    local.get 5
    local.get 6
    call_indirect $t (param i32 i32 i32 i32 i32 i32) (result i32))

  (func (export "call_u_2") (param i32 i32 i32 i32 i32 i32 i32) (result i32)
    local.get 0
    local.get 1
    local.get 2
    local.get 3
    local.get 4
    local.get 5
    local.get 6
    call_indirect $u (param i32 i32 i32 i32 i32 i32) (result i32)))

;; Now test copying between two imported tables.

;; Copy $i into $t at 0 from $u at 3.
(invoke "copy_into_t_from_u_2" (i32.const 0) (i32.const 3) (i32.const 1))

(assert_return
  (invoke "call_t_2" (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 1) (i32.const 0) (i32.const 0)
    (i32.const 0))
  (i32.const 1))

;; Copy $g into $u at 4 from $t at 1.
(invoke "copy_into_u_from_t_2" (i32.const 4) (i32.const 1) (i32.const 1))

(assert_return
  (invoke "call_u_2" (i32.const 0) (i32.const 1) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0)
    (i32.const 4))
  (i32.const 1))
