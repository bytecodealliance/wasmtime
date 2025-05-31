;;! target = "aarch64"
;;! test = "winch"

(module
    (func (param f32) (result i64)
        (local.get 0)
        (i64.trunc_f32_s)
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
;;       b.lo    #0x90
||||||| parent of 549c6a5f0 (Update disassembly tests)
;;       b.lo    #0x8c
=======
;;       b.lo    #0x84
>>>>>>> 549c6a5f0 (Update disassembly tests)
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    s0, [x28, #4]
;;       ldur    s0, [x28, #4]
;;       fcmp    s0, s0
<<<<<<< HEAD
;;       b.vs    #0x94
;;   50: mov     w16, #1
;;       movk    w16, #0xdf00, lsl #16
;;       fmov    s31, w16
;;       fcmp    s0, s31
;;       b.le    #0x98
;;   64: mov     x16, #0x5f000000
;;       fmov    s31, w16
;;       fcmp    s0, s31
;;       b.ge    #0x9c
;;   74: fcvtzs  x0, s0
||||||| parent of 549c6a5f0 (Update disassembly tests)
;;       b.vs    #0x90
;;   50: mov     x16, #0xdf000000
;;       fmov    s31, w16
;;       fcmp    s31, s0
;;       b.le    #0x94
;;   60: mov     x16, #0x5f000000
;;       fmov    s31, w16
;;       fcmp    s31, s0
;;       b.ge    #0x98
;;   70: fcvtzs  x0, s0
=======
;;       b.vs    #0x88
;;   50: ldr     s31, #0x98
;;       fcmp    s31, s0
;;       b.le    #0x8c
;;   5c: ldr     s31, #0xa0
;;       fcmp    s31, s0
;;       b.ge    #0x90
;;   68: fcvtzs  x0, s0
>>>>>>> 549c6a5f0 (Update disassembly tests)
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
<<<<<<< HEAD
||||||| parent of 549c6a5f0 (Update disassembly tests)
;;   8c: .byte   0x1f, 0xc1, 0x00, 0x00
=======
;;   84: .byte   0x1f, 0xc1, 0x00, 0x00
;;   88: .byte   0x1f, 0xc1, 0x00, 0x00
;;   8c: .byte   0x1f, 0xc1, 0x00, 0x00
>>>>>>> 549c6a5f0 (Update disassembly tests)
;;   90: .byte   0x1f, 0xc1, 0x00, 0x00
<<<<<<< HEAD
;;   94: .byte   0x1f, 0xc1, 0x00, 0x00
;;   98: .byte   0x1f, 0xc1, 0x00, 0x00
;;   9c: .byte   0x1f, 0xc1, 0x00, 0x00
||||||| parent of 549c6a5f0 (Update disassembly tests)
;;   94: .byte   0x1f, 0xc1, 0x00, 0x00
;;   98: .byte   0x1f, 0xc1, 0x00, 0x00
=======
;;   94: .byte   0x00, 0x00, 0x00, 0x00
;;   98: .byte   0x00, 0x00, 0x00, 0xdf
;;   9c: .byte   0x00, 0x00, 0x00, 0x00
;;   a0: .byte   0x00, 0x00, 0x00, 0x5f
>>>>>>> 549c6a5f0 (Update disassembly tests)
