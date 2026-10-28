;; Issue 15036: Element segment expressions are evaluated before any table writes during instantiation.
(module $M
  (type $t (func (result i32)))
  (table (export "t") 2 funcref)
  (table (export "a") 2 anyref)
  (func (export "call") (param i32) (result i32)
    (call_indirect (type $t) (local.get 0)))
  (func (export "get-a") (param i32) (result i32)
    (i31.get_s (ref.cast (ref i31) (table.get 1 (local.get 0))))))
(register "M" $M)

(assert_trap
  (module
    (type $a (array (mut i64)))
    (import "M" "t" (table 2 funcref))
    (table $t2 1 anyref)
    (func $f (result i32) (i32.const 7))
    (elem (table 0) (i32.const 0) func $f)
    (elem (table $t2) (i32.const 0) anyref
      (array.new_default $a (i32.const 0x7fffffff))))
  "allocation size too large")
(assert_trap (invoke $M "call" (i32.const 0)) "uninitialized element")

(assert_trap
  (module
    (type $a (array (mut i64)))
    (import "M" "a" (table 2 anyref))
    (elem (table 0) (i32.const 0) anyref
      (ref.i31 (i32.const 5))
      (array.new_default $a (i32.const 0x7fffffff))))
  "allocation size too large")
(assert_trap (invoke $M "get-a" (i32.const 0)) "null reference")
