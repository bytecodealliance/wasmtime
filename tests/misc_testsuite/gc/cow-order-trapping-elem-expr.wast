;;! gc = true

(module $M
  (type $t (func (result i32)))
  (table (export "t") 2 funcref)
  (func (export "call") (param i32) (result i32)
    (call_indirect (type $t) (local.get 0))))
(register "M" $M)

;; Instantiation fails because the `array.new_default` overflows, and this must
;; mean that the `data` segment is not applied.
(assert_trap
  (module
    (type $a (array (mut i64)))
    (import "M" "t" (table 2 funcref))
    (table $t2 1 anyref)
    (memory 1)
    (func $f (result i32) (i32.load8_u (i32.const 0)))
    (elem (table 0) (i32.const 0) func $f)
    (elem (table $t2) (i32.const 0) anyref (array.new_default $a (i32.const 0x7fffffff)))
    (data (i32.const 0) "\2a"))
  "allocation size too large")

(assert_return (invoke $M "call" (i32.const 0)) (i32.const 0))
