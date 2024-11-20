;;! simd = true

;; aligned and out of bounds
(module
  (func
    i32.const 0
    v128.load32_splat
    v128.any_true
    if
    end
  )
  (memory 0 6)
  (export "x" (func 0))
)
(assert_trap (invoke "x") "out of bounds memory access")

;; unaligned an in bounds
(module
  (func
    i32.const 1
    v128.load32_splat
    v128.any_true
    if
    end
  )
  (memory 1 6)
  (export "x" (func 0))
)
(assert_return (invoke "x"))
