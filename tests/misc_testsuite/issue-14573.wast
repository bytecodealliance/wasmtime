;; A trapping store must still trap when a loop that might not terminate lies
;; between it and a store that overwrites it.

(module
  (memory 1 1)
  (func (export "f") (param $addr i32) (param $spin i32)
    (i32.store (local.get $addr) (i32.const 1))
    (if (local.get $spin) (then (loop (br 0))))
    (i32.store (local.get $addr) (i32.const 2))
  )
)

(assert_trap (invoke "f" (i32.const 65536) (i32.const 1)) "out of bounds memory access")

(module
  (memory 1 1)
  (func (export "f") (param $addr i32) (param $spin i32)
    (i32.store (local.get $addr) (i32.const 1))
    (loop (br_if 0 (local.get $spin)))
    (i32.store (local.get $addr) (i32.const 2))
  )
)

(assert_trap (invoke "f" (i32.const 65536) (i32.const 1)) "out of bounds memory access")
