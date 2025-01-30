;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i32)
        (f64.const 1.0)
        (i32.trunc_f64_u)
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
;;       mov     x16, #0x3ff0000000000000
;;       fmov    d0, x16
;;       fcmp    d0, d0
;;       b.vs    #0x60
;;   30: fmov    d31, #-1.00000000
;;       fcmp    d31, d0
;;       b.le    #0x64
;;   3c: mov     x16, #0x41f0000000000000
;;       fmov    d31, x16
;;       fcmp    d31, d0
;;       b.ge    #0x68
;;   4c: fcvtzu  w0, d0
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   60: .byte   0x1f, 0xc1, 0x00, 0x00
;;   64: .byte   0x1f, 0xc1, 0x00, 0x00
;;   68: .byte   0x1f, 0xc1, 0x00, 0x00
