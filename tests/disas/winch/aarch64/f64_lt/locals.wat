;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i32)
        (local $foo f64)
        (local $bar f64)

        (f64.const -2)
        (local.set $foo)
        (f64.const -3)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        (f64.lt)
    )
)

;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     x28, x28, #0x20
;;       mov     sp, x28
;;       stur    x0, [x28, #0x18]
;;       stur    x1, [x28, #0x10]
;;       mov     x16, #0
;;       stur    x16, [x28, #8]
;;       stur    x16, [x28]
;;       mov     x16, #-0x4000000000000000
;;       fmov    d0, x16
;;       stur    d0, [x28, #8]
;;       mov     x16, #-0x3ff8000000000000
;;       fmov    d0, x16
;;       stur    d0, [x28]
;;       ldur    d0, [x28]
;;       ldur    d1, [x28, #8]
;;       fcmp    d0, d1
;;       cset    x0, mi
;;       add     x28, x28, #0x20
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
