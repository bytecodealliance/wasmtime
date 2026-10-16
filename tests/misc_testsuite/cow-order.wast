(module $M
  (type $t (func (result i32)))
  (table (export "t") 2 funcref)
  (func (export "call") (param i32) (result i32)
    (call_indirect (type $t) (local.get 0))))
(register "M" $M)

(assert_trap
  (module
    (import "M" "t" (table 2 funcref))
    (memory 1)
    (func $f (result i32) (i32.load8_u (i32.const 0)))
    (elem (table 0) (i32.const 0) func $f)
    (elem (table 0) (i32.const 2) func $f)
    (data (i32.const 0) "\2a"))
  "out of bounds table access")

(assert_return (invoke $M "call" (i32.const 0)) (i32.const 0))
