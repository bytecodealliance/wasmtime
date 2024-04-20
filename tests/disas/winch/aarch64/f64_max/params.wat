;;! target = "aarch64"
;;! test = "winch"

(module
    (func (param f64) (param f64) (result f64)
        (local.get 0)
        (local.get 1)
        (f64.max)
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
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       ldur    x0, [x28]
;;       ldur    x1, [x28, #8]
;;       fmax    d1, d1, d0
;;       fmov    d0, d1
;;       add     sp, sp, #0x20
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
