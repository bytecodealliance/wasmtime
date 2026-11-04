(module
  (func (export "i32.div_s") (param i32) (param i32) (result i32)
    (i32.div_s (local.get 0) (local.get 1))
  )
)

(assert_return (invoke "i32.div_s" (i32.const -1) (i32.const -1)) (i32.const 1))

(module
  (func (export "i32.rem_s") (param i32) (param i32) (result i32)
    (i32.rem_s (local.get 0) (local.get 1))
  )
)

(assert_return (invoke "i32.rem_s" (i32.const 123121) (i32.const -1)) (i32.const 0))

(module
  (func (export "i64.div_s") (param i64) (param i64) (result i64)
    (i64.div_s (local.get 0) (local.get 1))
  )
)

(assert_return (invoke "i64.div_s" (i64.const -1) (i64.const -1)) (i64.const 1))

(module
  (func (export "i64.rem_s") (param i64) (param i64) (result i64)
    (i64.rem_s (local.get 0) (local.get 1))
  )
)

(assert_return (invoke "i64.rem_s" (i64.const 123121) (i64.const -1)) (i64.const 0))

;; The divisor is loaded from memory, so a trap may come from the load or from
;; the division, and each must be reported correctly.
(module
  (memory 1)
  (data (i32.const 0) "\01\00\00\00")
  (data (i32.const 4) "\00\00\00\00")
  (data (i32.const 8) "\ff\ff\ff\ff")
  (func (export "i32.div_s") (param i32 i32) (result i32)
    (i32.div_s (local.get 0) (i32.load (local.get 1))))
  (func (export "i32.div_u") (param i32 i32) (result i32)
    (i32.div_u (local.get 0) (i32.load (local.get 1))))
  (func (export "i32.rem_s") (param i32 i32) (result i32)
    (i32.rem_s (local.get 0) (i32.load (local.get 1))))
  (func (export "i32.rem_u") (param i32 i32) (result i32)
    (i32.rem_u (local.get 0) (i32.load (local.get 1))))
)

;; Divide by 1.
(assert_return (invoke "i32.div_s" (i32.const 100) (i32.const 0))
  (i32.const 100))
(assert_return (invoke "i32.div_u" (i32.const 101) (i32.const 0))
  (i32.const 101))
(assert_return (invoke "i32.rem_s" (i32.const 102) (i32.const 0))
  (i32.const 0))
(assert_return (invoke "i32.rem_u" (i32.const 103) (i32.const 0))
  (i32.const 0))

;; Divide by 0.
(assert_trap (invoke "i32.div_s" (i32.const 100) (i32.const 4))
  "integer divide by zero")
(assert_trap (invoke "i32.div_u" (i32.const 100) (i32.const 4))
  "integer divide by zero")
(assert_trap (invoke "i32.rem_s" (i32.const 100) (i32.const 4))
  "integer divide by zero")
(assert_trap (invoke "i32.rem_u" (i32.const 100) (i32.const 4))
  "integer divide by zero")

;; Divide by -1.
(assert_trap (invoke "i32.div_s" (i32.const 0x80000000) (i32.const 8))
  "integer overflow")
(assert_return (invoke "i32.rem_s" (i32.const 0x80000000) (i32.const 8))
  (i32.const 0))

;; The load is out of bounds.
(assert_trap (invoke "i32.div_s" (i32.const 100) (i32.const 100000))
  "out of bounds memory access")
(assert_trap (invoke "i32.div_u" (i32.const 100) (i32.const 100000))
  "out of bounds memory access")
(assert_trap (invoke "i32.rem_s" (i32.const 100) (i32.const 100000))
  "out of bounds memory access")
(assert_trap (invoke "i32.rem_u" (i32.const 100) (i32.const 100000))
  "out of bounds memory access")
