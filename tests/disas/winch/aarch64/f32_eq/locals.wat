;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i32)
        (local $foo f32)
        (local $bar f32)

        (f32.const 2)
        (local.set $foo)
        (f32.const 3)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        (f32.eq)
    )
)

;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       mov     x16, #0
;;       stur    x16, [x28]
;;       mov     x16, #0x40000000
;;       fmov    s0, w16
;;       stur    s0, [x28, #4]
;;       mov     x16, #0x40400000
;;       fmov    s0, w16
;;       stur    s0, [x28]
;;       ldur    s0, [x28]
;;       ldur    s1, [x28, #4]
;;       fcmp    s0, s1
;;       cset    x0, eq
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
