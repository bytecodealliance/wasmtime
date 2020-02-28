(module $m
  (table (export "table") funcref (elem $zero $zero $zero $zero $zero $zero $zero $zero $zero $zero))

  (func $zero (result i32)
    (i32.const 0))

  (func (export "indirect-call") (param i32) (result i32)
    local.get 0
    call_indirect (result i32)))

(register "m" $m)

(assert_trap
  (module
    (table (import "m" "table") 10 funcref)

    (func $one (result i32)
      (i32.const 1))

    ;; An in-bounds segment that should get initialized in the table.
    (elem (i32.const 7) $one)

    ;; Part of this segment is out of bounds, so none of its elements should be
    ;; initialized into the table, and it should trap.
    (elem (i32.const 9) $one $one $one)
  )
  "out of bounds"
)

;; The first `$one` segment *was* initialized OK.
(assert_return (invoke "indirect-call" (i32.const 7)) (i32.const 1))

;; The second `$one` segment is partially out of bounds, and therefore none of
;; its elements were written into the table.
(assert_return (invoke "indirect-call" (i32.const 9)) (i32.const 0))
