;;! target = "aarch64"
;;! test = "winch"
;;! flags = "-Wwide-arithmetic"

(module
  (func (result i64 i64)
    (i64.const 10)
    (i64.const 20)
    (i64.const 30)
    (i64.const 40)
    (i64.add128)
  )
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x1, #8]
;;       ldur    x16, [x16, #0x18]
;;       mov     x17, #0
;;       movk    x17, #0x20
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x98
;;   2c: mov     x9, x1
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x1, [x28, #0x10]
;;       stur    x2, [x28, #8]
;;       stur    x0, [x28]
;;       mov     x0, #0x28
;;       mov     x1, #0x1e
;;       mov     x2, #0x14
;;       mov     x3, #0xa
;;       adds    x3, x3, x1, uxtx
;;       adc     x2, x2, x0
;;       mov     x0, x2
;;       sub     x28, x28, #8
;;       mov     sp, x28
;;       stur    x3, [x28]
;;       ldur    x1, [x28, #8]
;;       ldur    x16, [x28]
;;       add     x28, x28, #8
;;       mov     sp, x28
;;       stur    x16, [x1]
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   98: .byte   0x1f, 0xc1, 0x00, 0x00
