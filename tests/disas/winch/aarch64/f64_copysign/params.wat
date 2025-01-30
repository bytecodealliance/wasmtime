;;! target = "aarch64"
;;! test = "winch"

(module
    (func (param f64) (param f64) (result f64)
        (local.get 0)
        (local.get 1)
        (f64.copysign)
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
;;       stur    d0, [x28, #8]
;;       stur    d1, [x28]
;;       ldur    d0, [x28]
;;       ldur    d1, [x28, #8]
;;       ushr    d0, d0, #0x3f
;;       sli     d1, d0, #0x3f
;;       fmov    d0, d1
;;       add     x28, x28, #0x20
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
