;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i32)
        (f64.const -1)
        (f64.const -2)
        (f64.lt)
    )
)

;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       mov     x16, #-0x4000000000000000
;;       fmov    d0, x16
;;       mov     x16, #-0x4010000000000000
;;       fmov    d1, x16
;;       fcmp    d0, d1
;;       cset    x0, mi
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
