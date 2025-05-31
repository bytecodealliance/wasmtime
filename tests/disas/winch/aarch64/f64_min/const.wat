;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result f64)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.min)
    )
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x10]
;;       mov     x17, #0
;;       movk    x17, #0x10
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x68
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       ldr     d0, #0x70
;;       ldr     d1, #0x78
;;       fmin    d1, d1, d0
;;       fmov    d0, d1
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   68: .byte   0x1f, 0xc1, 0x00, 0x00
;;   6c: .byte   0x00, 0x00, 0x00, 0x00
;;   70: .byte   0x9a, 0x99, 0x99, 0x99
;;   74: .byte   0x99, 0x99, 0x01, 0x40
;;   78: .byte   0x9a, 0x99, 0x99, 0x99
;;   7c: .byte   0x99, 0x99, 0xf1, 0x3f
