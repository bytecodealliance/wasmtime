;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i32)
        (local f64)  

        (local.get 0)
        (i32.trunc_f64_u)
    )
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     sp, sp, #0x18
;;       mov     x28, sp
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       mov     x16, #0
;;       stur    x16, [x28]
;;       ldur    d1, [x28]
;;       fcmp    d1, d1
;;       b.vs    #0x64
;;   34: fmov    d0, #-1.00000000
;;       fcmp    d1, d0
;;       b.le    #0x68
;;   40: mov     x16, #0x41f0000000000000
;;       fmov    d0, x16
;;       fcmp    d1, d0
;;       b.ge    #0x6c
;;   50: fcvtzu  w0, d1
;;       add     sp, sp, #0x18
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   64: .byte   0x1f, 0xc1, 0x00, 0x00
;;   68: .byte   0x1f, 0xc1, 0x00, 0x00
;;   6c: .byte   0x1f, 0xc1, 0x00, 0x00
