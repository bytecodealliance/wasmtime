;;! gc = true
;;! bulk_memory = true

;; All element segment expressions are evaluated before any active element
;; segment is written to a table, so a trapping expression must leave imported
;; tables untouched. See #15036.

(module $M
  (type $t (func (result i32)))
  (table (export "t") 2 funcref)
  (table (export "a") 2 anyref)
  (func (export "call") (param i32) (result i32)
    (call_indirect (type $t) (local.get 0)))
  (func (export "a-is-null") (param i32) (result i32)
    (ref.is_null (table.get 1 (local.get 0))))
  (func (export "get-a") (param i32) (result i32)
    (i31.get_s (ref.cast (ref i31) (table.get 1 (local.get 0))))))
(register "M" $M)

;; A trap in a later segment must not leave an earlier segment's writes behind.
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

;; A trap in a later element must not leave an earlier element of the same
;; segment behind.
(assert_trap
  (module
    (type $a (array (mut i64)))
    (import "M" "a" (table 2 anyref))
    (elem (table 0) (i32.const 0) anyref
      (ref.i31 (i32.const 5))
      (array.new_default $a (i32.const 0x7fffffff))))
  "allocation size too large")
(assert_return (invoke $M "a-is-null" (i32.const 0)) (i32.const 1))

;; A trap in a passive segment also comes before any active segment write.
(assert_trap
  (module
    (type $a (array (mut i64)))
    (import "M" "a" (table 2 anyref))
    (elem (table 0) (i32.const 0) anyref
      (ref.i31 (i32.const 5))
      (array.new_default $a (i32.const 1)))
    (elem anyref (array.new_default $a (i32.const 0x7fffffff))))
  "allocation size too large")
(assert_return (invoke $M "a-is-null" (i32.const 0)) (i32.const 1))

;; Out-of-bounds active segments still trap in order: earlier segments stay
;; written.
(assert_trap
  (module
    (type $a (array (mut i64)))
    (import "M" "a" (table 2 anyref))
    (elem (table 0) (i32.const 0) anyref (ref.i31 (i32.const 9)))
    (elem (table 0) (i32.const 1) anyref
      (array.new_default $a (i32.const 1))
      (array.new_default $a (i32.const 1))))
  "out of bounds table access")
(assert_return (invoke $M "get-a" (i32.const 0)) (i32.const 9))
(assert_return (invoke $M "a-is-null" (i32.const 1)) (i32.const 1))

;; When nothing traps, staged segments are written to the table as normal.
(module $N
  (type $a (array (mut i64)))
  (table $t 4 anyref)
  (elem (table $t) (i32.const 1) anyref
    (ref.i31 (i32.const 3))
    (array.new $a (i64.const 42) (i32.const 2))
    (ref.null any))
  (func (export "i31") (param i32) (result i32)
    (i31.get_s (ref.cast (ref i31) (table.get $t (local.get 0)))))
  (func (export "arr") (param i32) (result i64)
    (array.get $a (ref.cast (ref $a) (table.get $t (local.get 0))) (i32.const 1)))
  (func (export "is-null") (param i32) (result i32)
    (ref.is_null (table.get $t (local.get 0))))
  ;; Active segments are dropped after instantiation, so they behave like
  ;; empty segments for `table.init`.
  (func (export "init") (param i32)
    (table.init $t 0 (i32.const 0) (i32.const 0) (local.get 0))))
(assert_return (invoke $N "is-null" (i32.const 0)) (i32.const 1))
(assert_return (invoke $N "i31" (i32.const 1)) (i32.const 3))
(assert_return (invoke $N "arr" (i32.const 2)) (i64.const 42))
(assert_return (invoke $N "is-null" (i32.const 3)) (i32.const 1))
(assert_return (invoke $N "init" (i32.const 0)))
(assert_trap (invoke $N "init" (i32.const 1)) "out of bounds table access")
