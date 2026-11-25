;;! reference_types = true
;;! bulk_memory = true

;; Functions referenced several times from global initializers and element
;; segments can be called through each of these references.

(module
  (type $t (func (result i32)))
  (func $f (type $t) (i32.const 1))
  (func $g (type $t) (i32.const 2))

  (global $a funcref (ref.func $f))
  (global $b funcref (ref.func $f))
  (global $c funcref (ref.func $g))

  (table $tab 6 funcref)
  (elem (table $tab) (i32.const 0) func $f $g $f)
  (elem $passive funcref (ref.func $g) (ref.func $f))

  (func $call-at-5 (param funcref) (result i32)
    (table.set $tab (i32.const 5) (local.get 0))
    (call_indirect $tab (type $t) (i32.const 5)))
  (func (export "call-a") (result i32) (call $call-at-5 (global.get $a)))
  (func (export "call-b") (result i32) (call $call-at-5 (global.get $b)))
  (func (export "call-c") (result i32) (call $call-at-5 (global.get $c)))

  (func (export "call-table") (param i32) (result i32)
    (call_indirect $tab (type $t) (local.get 0)))
  (func (export "init-from-passive")
    (table.init $tab $passive (i32.const 3) (i32.const 0) (i32.const 2)))
)

(assert_return (invoke "call-a") (i32.const 1))
(assert_return (invoke "call-b") (i32.const 1))
(assert_return (invoke "call-c") (i32.const 2))
(assert_return (invoke "call-table" (i32.const 0)) (i32.const 1))
(assert_return (invoke "call-table" (i32.const 1)) (i32.const 2))
(assert_return (invoke "call-table" (i32.const 2)) (i32.const 1))
(invoke "init-from-passive")
(assert_return (invoke "call-table" (i32.const 3)) (i32.const 2))
(assert_return (invoke "call-table" (i32.const 4)) (i32.const 1))
