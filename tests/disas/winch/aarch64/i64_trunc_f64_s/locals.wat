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
;;       ldur    x16, [x0, #8]
;;       ldur    x16, [x16, #0x10]
;;       mov     x17, #0
;;       movk    x17, #0x18
;;       add     x16, x16, x17
;;       cmp     sp, x16
<<<<<<< HEAD
;;       b.lo    #0x94
||||||| parent of 549c6a5f0 (Update disassembly tests)
;;       b.lo    #0x90
=======
;;       b.lo    #0x88
>>>>>>> 549c6a5f0 (Update disassembly tests)
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       mov     x16, #0
;;       stur    x16, [x28]
;;       ldur    d0, [x28]
;;       fcmp    d0, d0
<<<<<<< HEAD
;;       b.vs    #0x98
;;   54: mov     x16, #1
;;       movk    x16, #0xc3e0, lsl #48
;;       fmov    d31, x16
;;       fcmp    d0, d31
;;       b.le    #0x9c
;;   68: mov     x16, #0x43e0000000000000
;;       fmov    d31, x16
;;       fcmp    d0, d31
;;       b.ge    #0xa0
;;   78: fcvtzs  x0, d0
||||||| parent of 549c6a5f0 (Update disassembly tests)
;;       b.vs    #0x94
;;   54: mov     x16, #-0x3c20000000000000
;;       fmov    d31, x16
;;       fcmp    d31, d0
;;       b.le    #0x98
;;   64: mov     x16, #0x43e0000000000000
;;       fmov    d31, x16
;;       fcmp    d31, d0
;;       b.ge    #0x9c
;;   74: fcvtzs  x0, d0
=======
;;       b.vs    #0x8c
;;   54: ldr     d31, #0x98
;;       fcmp    d31, d0
;;       b.le    #0x90
;;   60: ldr     d31, #0xa0
;;       fcmp    d31, d0
;;       b.ge    #0x94
;;   6c: fcvtzs  x0, d0
>>>>>>> 549c6a5f0 (Update disassembly tests)
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
<<<<<<< HEAD
||||||| parent of 549c6a5f0 (Update disassembly tests)
;;   90: .byte   0x1f, 0xc1, 0x00, 0x00
=======
;;   88: .byte   0x1f, 0xc1, 0x00, 0x00
;;   8c: .byte   0x1f, 0xc1, 0x00, 0x00
;;   90: .byte   0x1f, 0xc1, 0x00, 0x00
>>>>>>> 549c6a5f0 (Update disassembly tests)
;;   94: .byte   0x1f, 0xc1, 0x00, 0x00
<<<<<<< HEAD
;;   98: .byte   0x1f, 0xc1, 0x00, 0x00
;;   9c: .byte   0x1f, 0xc1, 0x00, 0x00
;;   a0: .byte   0x1f, 0xc1, 0x00, 0x00
||||||| parent of 549c6a5f0 (Update disassembly tests)
;;   98: .byte   0x1f, 0xc1, 0x00, 0x00
;;   9c: .byte   0x1f, 0xc1, 0x00, 0x00
=======
;;   98: .byte   0x00, 0x00, 0x00, 0x00
;;   9c: .byte   0x00, 0x00, 0xe0, 0xc3
;;   a0: .byte   0x00, 0x00, 0x00, 0x00
;;   a4: .byte   0x00, 0x00, 0xe0, 0x43
>>>>>>> 549c6a5f0 (Update disassembly tests)
