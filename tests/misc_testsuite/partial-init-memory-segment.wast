(module $m
  (memory (export "mem") 1)

  (func (export "load") (param i32) (result i32)
    local.get 0
    i32.load8_u))

(register "m" $m)

(assert_trap
  (module
    (memory (import "m" "mem") 1)

    ;; This is in bounds, and should get written to the memory.
    (data (i32.const 0) "abc")

    ;; Partially out of bounds. None of these bytes should get written, and
    ;; instantiation should trap.
    (data (i32.const 65530) "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz")
  )
  "out of bounds"
)

;; The first data segment got written.
(assert_return (invoke $m "load" (i32.const 0)) (i32.const 97))
(assert_return (invoke $m "load" (i32.const 1)) (i32.const 98))
(assert_return (invoke $m "load" (i32.const 2)) (i32.const 99))

;; The second did not get partially written.
(assert_return (invoke $m "load" (i32.const 65530)) (i32.const 0))
(assert_return (invoke $m "load" (i32.const 65531)) (i32.const 0))
(assert_return (invoke $m "load" (i32.const 65532)) (i32.const 0))
(assert_return (invoke $m "load" (i32.const 65533)) (i32.const 0))
(assert_return (invoke $m "load" (i32.const 65534)) (i32.const 0))
(assert_return (invoke $m "load" (i32.const 65535)) (i32.const 0))
