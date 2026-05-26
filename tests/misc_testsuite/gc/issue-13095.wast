;;! gc = true
;;! bulk_memory = true

(module $A
  (type $super (sub (func (param i32) (result i32))))
  (type $sub (sub $super (func (param i32) (result i32))))
  (func (export "f") (type $sub) (param i32) (result i32) (local.get 0))
)
(register "A" $A)

(module $B
  (type $super (sub (func (param i32) (result i32))))
  (type $sub (sub $super (func (param i32) (result i32))))

  ;; Valid covariant import: $sub <: $super
  (import "A" "f" (func $f (type $super)))

  (table 10 funcref)
  (elem declare func $f)

  (func (export "test_super") (result i32)
    (table.set (i32.const 0) (ref.func $f))
    (i32.const 99) (i32.const 0)
    (call_indirect (type $super)))

  (func (export "test_sub") (result i32)
    (table.set (i32.const 0) (ref.func $f))
    (i32.const 99) (i32.const 0)
    (call_indirect (type $sub)))
)

(assert_return (invoke "test_super") (i32.const 99))
(assert_return (invoke "test_sub")   (i32.const 99))
