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
        f64.mul
    )
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     sp, sp, #0x20
;;       mov     x28, sp
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
;;       stur    x0, [x28, #8]
;;       mov     x16, #0x999a
;;       movk    x16, #0x9999, lsl #16
;;       movk    x16, #0x9999, lsl #32
;;       movk    x16, #0x4001, lsl #48
;;       fmov    d0, x16
;;       stur    x0, [x28]
;;       ldur    x0, [x28]
;;       ldur    x1, [x28, #8]
;;       fmul    d1, d1, d0
;;       fmov    d0, d1
;;       add     sp, sp, #0x20
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
