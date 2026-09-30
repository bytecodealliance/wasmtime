;;! tail_call = true
;;! reference_types = true

(module
  (type $expected (func (result i32)))
  (type $wrong (func (result i64)))

  (func $good (type $expected) i32.const 7)
  (func $bad (type $wrong) i64.const 8)

  (table 3 funcref)
  (elem (i32.const 0) $good $bad)

  (func (export "run") (param i32) (result i32)
    local.get 0
    return_call_indirect (type $expected)))

(assert_return (invoke "run" (i32.const 0)) (i32.const 7))
(assert_trap (invoke "run" (i32.const 1)) "indirect call type mismatch")
(assert_trap (invoke "run" (i32.const 2)) "uninitialized element")
