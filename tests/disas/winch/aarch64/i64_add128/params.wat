;;! target = "aarch64"
;;! test = "winch"
;;! flags = "-Wwide-arithmetic"

(module
  (func (param i64 i64 i64 i64) (result i64 i64)
    (local.get 0)
    (local.get 1)
    (local.get 2)
    (local.get 3)
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
;;       movk    x17, #0x40
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0xa8
;;   2c: mov     x9, x1
;;       sub     x28, x28, #0x38
;;       mov     sp, x28
;;       stur    x1, [x28, #0x30]
;;       stur    x2, [x28, #0x28]
;;       stur    x3, [x28, #0x20]
;;       stur    x4, [x28, #0x18]
;;       stur    x5, [x28, #0x10]
;;       stur    x6, [x28, #8]
;;       stur    x0, [x28]
;;       ldur    x0, [x28, #8]
;;       ldur    x1, [x28, #0x10]
;;       ldur    x2, [x28, #0x18]
;;       ldur    x3, [x28, #0x20]
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
;;       add     x28, x28, #0x38
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   a8: .byte   0x1f, 0xc1, 0x00, 0x00
