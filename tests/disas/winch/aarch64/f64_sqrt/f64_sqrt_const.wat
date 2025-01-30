;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result f64)
        (f64.const 1.32)
        (f64.sqrt)
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
;;       mov     x16, #0x851f
;;       movk    x16, #0x51eb, lsl #16
;;       movk    x16, #0x1eb8, lsl #32
;;       movk    x16, #0x3ff5, lsl #48
;;       fmov    d0, x16
;;       fsqrt   d0, d0
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
