;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i32)
        (local $foo f64)
        (local $bar f64)

        (f64.const 2)
        (local.set $foo)
        (f64.const 3)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        (f64.eq)
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
;;       b.lo    #0x84
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x20
;;       mov     sp, x28
;;       stur    x0, [x28, #0x18]
;;       stur    x1, [x28, #0x10]
;;       mov     x16, #0
;;       stur    x16, [x28, #8]
;;       stur    x16, [x28]
;;       fmov    d0, #2.00000000
;;       stur    d0, [x28, #8]
;;       fmov    d0, #3.00000000
;;       stur    d0, [x28]
;;       ldur    d0, [x28]
;;       ldur    d1, [x28, #8]
;;       fcmp    d1, d0
;;       cset    x0, eq
;;       add     x28, x28, #0x20
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   84: .byte   0x1f, 0xc1, 0x00, 0x00
