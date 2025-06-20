;;! target = "aarch64"
;;! test = "winch"

(module
  (func $fibonacci8 (param $n i32) (result i32)
    (if (result i32) (i32.le_s (local.get $n) (i32.const 1))
      (then
        ;; If n <= 1, return n (base case)
        (local.get $n)
      )
      (else
        ;; Else, return fibonacci(n - 1) + fibonacci(n - 2)
        (i32.add
          (call $fibonacci8
            (i32.sub (local.get $n) (i32.const 1)) ;; Calculate n - 1
          )
          (call $fibonacci8
            (i32.sub (local.get $n) (i32.const 2)) ;; Calculate n - 2
          )
        )
      )
    )
  )
  (export "fib" (func $fibonacci8))
)

;; wasm[0]::function[0]::fibonacci8:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x10]
;;       mov     x17, #0
;;       movk    x17, #0x20
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x10c
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    w2, [x28, #4]
;;       ldur    w0, [x28, #4]
;;       cmp     w0, #1
;;       cset    x0, le
;;       tst     w0, w0
;;       b.eq    #0x64
;;       b       #0x5c
;;   5c: ldur    w0, [x28, #4]
;;       b       #0xf4
;;   64: ldur    w0, [x28, #4]
;;       sub     w0, w0, #1
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       mov     x0, x9
;;       mov     x1, x9
;;       ldur    w2, [x28, #4]
;;       bl      #0
;;   90: add     x28, x28, #4
;;       mov     sp, x28
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x10]
;;       ldur    w1, [x28, #4]
;;       sub     w1, w1, #2
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w0, [x28]
;;       sub     x28, x28, #4
;;       mov     sp, x28
;;       stur    w1, [x28]
;;       mov     x0, x9
;;       mov     x1, x9
;;       ldur    w2, [x28]
;;       bl      #0
;;   d4: add     x28, x28, #4
;;       mov     sp, x28
;;       ldur    x9, [x28, #0x14]
;;       ldur    w1, [x28]
;;       add     x28, x28, #4
;;       mov     sp, x28
;;       add     w1, w1, w0, uxtx
;;       mov     w0, w1
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;  10c: .byte   0x1f, 0xc1, 0x00, 0x00
