;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i32)
        (local $foo f32)
        (local $bar f32)

        (f32.const -2)
        (local.set $foo)
        (f32.const -3)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        (f32.le)
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
;;       movk    x17, #0x18
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x80
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       mov     x16, #0
;;       stur    x16, [x28]
;;       fmov    s0, #-2.00000000
;;       stur    s0, [x28, #4]
;;       fmov    s0, #-3.00000000
;;       stur    s0, [x28]
;;       ldur    s0, [x28]
;;       ldur    s1, [x28, #4]
;;       fcmp    s1, s0
;;       cset    x0, ls
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   80: .byte   0x1f, 0xc1, 0x00, 0x00
