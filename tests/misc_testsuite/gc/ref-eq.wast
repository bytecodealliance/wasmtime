;;! gc = true

;; Test ref.eq for various GC reference types.

(module
  (type $s (struct))
  (type $t (struct (field i32)))
  (type $arr (array i8))

  (table 10 (ref null eq))

  (func (export "init")
    ;; slot 0: null (eq null)
    (table.set (i32.const 0) (ref.null eq))
    ;; slot 1: null i31ref
    (table.set (i32.const 1) (ref.null i31))
    ;; slot 2: i31(7)
    (table.set (i32.const 2) (ref.i31 (i32.const 7)))
    ;; slot 3: i31(7) again; equal value
    (table.set (i32.const 3) (ref.i31 (i32.const 7)))
    ;; slot 4: i31(8); different value
    (table.set (i32.const 4) (ref.i31 (i32.const 8)))
    ;; slot 5: struct $s instance A
    (table.set (i32.const 5) (struct.new_default $s))
    ;; slot 6: struct $s instance B (different object)
    (table.set (i32.const 6) (struct.new_default $s))
    ;; slot 7: array instance A
    (table.set (i32.const 7) (array.new_default $arr (i32.const 0)))
    ;; slot 8: array instance B
    (table.set (i32.const 8) (array.new_default $arr (i32.const 0)))
    ;; slot 9: copy of slot 5 (same identity)
    (table.set (i32.const 9) (table.get (i32.const 5)))
  )

  (func (export "eq") (param $i i32) (param $j i32) (result i32)
    (ref.eq (table.get (local.get $i)) (table.get (local.get $j)))
  )
)

(invoke "init")

;; null eq null -> true (both are null / none)
(assert_return (invoke "eq" (i32.const 0) (i32.const 0)) (i32.const 1))
;; null i31 eq null eq -> true
(assert_return (invoke "eq" (i32.const 0) (i32.const 1)) (i32.const 1))
(assert_return (invoke "eq" (i32.const 1) (i32.const 0)) (i32.const 1))
;; null vs non-null -> false
(assert_return (invoke "eq" (i32.const 0) (i32.const 2)) (i32.const 0))
(assert_return (invoke "eq" (i32.const 2) (i32.const 0)) (i32.const 0))
;; same i31 value -> true
(assert_return (invoke "eq" (i32.const 2) (i32.const 2)) (i32.const 1))
(assert_return (invoke "eq" (i32.const 2) (i32.const 3)) (i32.const 1))
;; different i31 values -> false
(assert_return (invoke "eq" (i32.const 2) (i32.const 4)) (i32.const 0))
;; same struct identity (via table.get copy) -> true
(assert_return (invoke "eq" (i32.const 5) (i32.const 9)) (i32.const 1))
;; different struct instances -> false
(assert_return (invoke "eq" (i32.const 5) (i32.const 6)) (i32.const 0))
;; struct vs array -> false
(assert_return (invoke "eq" (i32.const 5) (i32.const 7)) (i32.const 0))
;; different array instances -> false
(assert_return (invoke "eq" (i32.const 7) (i32.const 8)) (i32.const 0))
;; array identity -> true
(assert_return (invoke "eq" (i32.const 7) (i32.const 7)) (i32.const 1))
