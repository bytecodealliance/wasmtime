;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i64)
        (local f64)  

        (local.get 0)
        (i64.trunc_f64_s)
    )
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       mov     x16, #0
;;       stur    x16, [x28]
;;       ldur    d0, [x28]
;;       fcmp    d0, d0
;;       b.vs    #0x74
;;   38: mov     x16, #-0x3c20000000000000
;;       fmov    d31, x16
;;       fcmp    d31, d0
;;       b.le    #0x78
;;   48: mov     x16, #0x43e0000000000000
;;       fmov    d31, x16
;;       fcmp    d31, d0
;;       b.ge    #0x7c
;;   58: fcvtzs  x0, d0
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   74: .byte   0x1f, 0xc1, 0x00, 0x00
;;   78: .byte   0x1f, 0xc1, 0x00, 0x00
;;   7c: .byte   0x1f, 0xc1, 0x00, 0x00
