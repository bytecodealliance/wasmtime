;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result f64)
        (local $foo f64)  
        (local $bar f64)

        (f64.const 1.1)
        (local.set $foo)

        (f64.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f64.div
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
;;       movk    x17, #0x20
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0xa4
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x20
;;       mov     sp, x28
;;       stur    x0, [x28, #0x18]
;;       stur    x1, [x28, #0x10]
;;       mov     x16, #0
;;       stur    x16, [x28, #8]
;;       stur    x16, [x28]
;;       mov     x16, #0x999a
;;       movk    x16, #0x9999, lsl #16
;;       movk    x16, #0x9999, lsl #32
;;       movk    x16, #0x3ff1, lsl #48
;;       fmov    d0, x16
;;       stur    d0, [x28, #8]
;;       mov     x16, #0x999a
;;       movk    x16, #0x9999, lsl #16
;;       movk    x16, #0x9999, lsl #32
;;       movk    x16, #0x4001, lsl #48
;;       fmov    d0, x16
;;       stur    d0, [x28]
;;       ldur    d0, [x28]
;;       ldur    d1, [x28, #8]
;;       fdiv    d1, d1, d0
;;       fmov    d0, d1
;;       add     x28, x28, #0x20
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   a4: .byte   0x1f, 0xc1, 0x00, 0x00
