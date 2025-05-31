;;! target = "aarch64"
;;! test = "winch"

(module
    (func (param f64) (result i64)
        (local.get 0)
        (i64.trunc_f64_u)
    )
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       str     x28, [sp, #-0x10]!
;;       mov     x28, sp
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x10]
;;       mov     x17, #0
;;       movk    x17, #0x18
;;       add     x16, x16, x17
;;       cmp     sp, x16
;;       b.lo    #0x84
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    d0, [x28]
;;       ldur    d0, [x28]
;;       fcmp    d0, d0
;;       b.vs    #0x88
;;   50: fmov    d31, #-1.00000000
<<<<<<< HEAD
;;       fcmp    d0, d31
;;       b.le    #0x90
;;   5c: mov     x16, #0x43f0000000000000
;;       fmov    d31, x16
;;       fcmp    d0, d31
;;       b.ge    #0x94
;;   6c: fcvtzu  x0, d0
||||||| parent of 549c6a5f0 (Update disassembly tests)
;;       fcmp    d31, d0
;;       b.le    #0x90
;;   5c: mov     x16, #0x43f0000000000000
;;       fmov    d31, x16
;;       fcmp    d31, d0
;;       b.ge    #0x94
;;   6c: fcvtzu  x0, d0
=======
;;       fcmp    d31, d0
;;       b.le    #0x8c
;;   5c: ldr     d31, #0x98
;;       fcmp    d31, d0
;;       b.ge    #0x90
;;   68: fcvtzu  x0, d0
>>>>>>> 549c6a5f0 (Update disassembly tests)
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   84: .byte   0x1f, 0xc1, 0x00, 0x00
;;   88: .byte   0x1f, 0xc1, 0x00, 0x00
;;   8c: .byte   0x1f, 0xc1, 0x00, 0x00
;;   90: .byte   0x1f, 0xc1, 0x00, 0x00
;;   94: .byte   0x00, 0x00, 0x00, 0x00
;;   98: .byte   0x00, 0x00, 0x00, 0x00
;;   9c: .byte   0x00, 0x00, 0xf0, 0x43
