;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i32)
        (local f64)  

        (local.get 0)
        (i32.trunc_f64_s)
    )
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
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
;;       b.vs    #0x70
;;   34: mov     x16, #0x200000
;;       movk    x16, #0xc1e0, lsl #48
;;       fmov    d31, x16
;;       fcmp    d31, d0
;;       b.le    #0x74
;;   48: mov     x16, #0x41e0000000000000
;;       fmov    d31, x16
;;       fcmp    d31, d0
;;       b.ge    #0x78
;;   58: fcvtzs  w0, d0
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   70: .byte   0x1f, 0xc1, 0x00, 0x00
;;   74: .byte   0x1f, 0xc1, 0x00, 0x00
;;   78: .byte   0x1f, 0xc1, 0x00, 0x00
