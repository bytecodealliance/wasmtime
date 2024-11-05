;;! gc = true
;;! function_references = true

(module
  (table $t1 2 funcref)
  (elem (table $t1) (i32.const 0) func $nop)
  (func $nop)

  (func (export "t1") (param i32)
    local.get 0
    call_indirect $t1)
  (func (export "t1-wrong-type") (param i32)
    i32.const 0
    local.get 0
    call_indirect $t1 (param i32))

  (type $empty (func))
  (table $t2 2 (ref null $empty))
  (elem (table $t2) (i32.const 0) (ref null $empty) (ref.func $nop))

  (func (export "t2") (param i32)
    local.get 0
    call_indirect $t2)
  (func (export "t2-wrong-type") (param i32)
    i32.const 0
    local.get 0
    call_indirect $t2 (param i32))

  (table $t3 2 (ref $empty) (ref.func $nop))

  (func (export "t3") (param i32)
    local.get 0
    call_indirect $t3)
  (func (export "t3-wrong-type") (param i32)
    i32.const 0
    local.get 0
    call_indirect $t3 (param i32))
)

(assert_return (invoke "t1" (i32.const 0)))
(assert_trap (invoke "t1" (i32.const 1)) "uninitialized element")
(assert_trap (invoke "t1" (i32.const 2)) "out of bounds")
(assert_trap (invoke "t1-wrong-type" (i32.const 0)) "call type mismatch")
(assert_trap (invoke "t1-wrong-type" (i32.const 1)) "uninitialized element")
(assert_trap (invoke "t1-wrong-type" (i32.const 2)) "out of bounds")
(assert_return (invoke "t2" (i32.const 0)))
(assert_trap (invoke "t2" (i32.const 1)) "uninitialized element")
(assert_trap (invoke "t2" (i32.const 2)) "out of bounds")
(assert_trap (invoke "t2-wrong-type" (i32.const 0)) "call type mismatch")
(assert_trap (invoke "t2-wrong-type" (i32.const 1)) "uninitialized element")
(assert_trap (invoke "t2-wrong-type" (i32.const 2)) "out of bounds")
(assert_return (invoke "t3" (i32.const 0)))
(assert_return (invoke "t3" (i32.const 1)))
(assert_trap (invoke "t3" (i32.const 2)) "out of bounds")
(assert_trap (invoke "t3-wrong-type" (i32.const 0)) "call type mismatch")
(assert_trap (invoke "t3-wrong-type" (i32.const 1)) "call type mismatch")
(assert_trap (invoke "t3-wrong-type" (i32.const 2)) "out of bounds")
