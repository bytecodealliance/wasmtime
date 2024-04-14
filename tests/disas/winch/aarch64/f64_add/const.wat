;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result f64)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.add)
    )
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     sp, sp, #0x10
;;       mov     x28, sp
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       mov     x16, #0x999a
;;       movk    x16, #0x9999, lsl #16
;;       movk    x16, #0x9999, lsl #32
;;       movk    x16, #0x4001, lsl #48
;;       fmov    d0, x16
;;       mov     x16, #0x999a
;;       movk    x16, #0x9999, lsl #16
;;       movk    x16, #0x9999, lsl #32
;;       movk    x16, #0x3ff1, lsl #48
;;       fmov    d1, x16
;;       fadd    d1, d1, d0
;;       fmov    d0, d1
;;       add     sp, sp, #0x10
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
