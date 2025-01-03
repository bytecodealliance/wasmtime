;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i64)
        (f64.const 1.0)
        (i64.trunc_f64_u)
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
;;       mov     x16, #0x3ff0000000000000
;;       fmov    d1, x16
;;       fcmp    d1, d1
;;       b.vs    #0x60
;;   30: fmov    d31, #-1.00000000
;;       fcmp    d31, d1
;;       b.le    #0x64
;;   3c: mov     x16, #0x43f0000000000000
;;       fmov    d31, x16
;;       fcmp    d31, d1
;;       b.ge    #0x68
;;   4c: fcvtzu  x0, d1
;;       add     sp, sp, #0x10
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   60: .byte   0x1f, 0xc1, 0x00, 0x00
;;   64: .byte   0x1f, 0xc1, 0x00, 0x00
;;   68: .byte   0x1f, 0xc1, 0x00, 0x00
