;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result f64)
        (f64.const -1.1)
        (f64.const 2.2)
        (f64.copysign)
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
;;       mov     x16, #0x999a
;;       movk    x16, #0x9999, lsl #16
;;       movk    x16, #0x9999, lsl #32
;;       movk    x16, #0x4001, lsl #48
;;       fmov    d0, x16
;;       mov     x16, #0x999a
;;       movk    x16, #0x9999, lsl #16
;;       movk    x16, #0x9999, lsl #32
;;       movk    x16, #0xbff1, lsl #48
;;       fmov    d1, x16
;;       ushr    d0, d0, #0x3f
;;       sli     d1, d0, #0x3f
;;       fmov    d0, d1
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
