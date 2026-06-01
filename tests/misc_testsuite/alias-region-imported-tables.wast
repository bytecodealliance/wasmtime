;;! reference_types = true
;;! bulk_memory = true
;; Test that two imports of the same table correctly alias.
;; The alias region system should ensure that table.set through one import
;; is visible through call_indirect on the other import.

(module $M
  (type $ft (func (result i32)))
  (func $f99 (type $ft) (i32.const 99))
  (func $f42 (type $ft) (i32.const 42))
  (table (export "a") (export "b") 10 funcref)
  (elem declare func $f99 $f42)
  (func (export "get_f99") (result funcref) (ref.func $f99))
  (func (export "get_f42") (result funcref) (ref.func $f42))
)
(register "M")

(module
  (type $ft (func (result i32)))
  (import "M" "a" (table $a 10 funcref))
  (import "M" "b" (table $b 10 funcref))
  (import "M" "get_f99" (func $get_f99 (result funcref)))
  (import "M" "get_f42" (func $get_f42 (result funcref)))
  (func (export "test") (result i32)
    ;; Set element 0 of table $a
    (table.set $a (i32.const 0) (call $get_f99))
    ;; Call through table $b at index 0 -- should see what we set via $a
    (call_indirect $b (type $ft) (i32.const 0))
  )
)
(assert_return (invoke "test") (i32.const 99))
