;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i64)
        (f32.const 1.0)
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
;;       movk    x17, #0x10
;;       add     x16, x16, x17
;;       cmp     sp, x16
<<<<<<< HEAD
;;       b.lo    #0x90
||||||| parent of 549c6a5f0 (Update disassembly tests)
;;       b.lo    #0x8c
=======
;;       b.lo    #0x80
>>>>>>> 549c6a5f0 (Update disassembly tests)
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       fmov    s0, #1.00000000
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
;;       b.vs    #0x84
;;   4c: ldr     s31, #0x90
;;       fcmp    s31, s0
;;       b.le    #0x88
;;   58: ldr     s31, #0x98
;;       fcmp    s31, s0
;;       b.ge    #0x8c
;;   64: fcvtzs  x0, s0
>>>>>>> 549c6a5f0 (Update disassembly tests)
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
<<<<<<< HEAD
;;   90: .byte   0x1f, 0xc1, 0x00, 0x00
;;   94: .byte   0x1f, 0xc1, 0x00, 0x00
;;   98: .byte   0x1f, 0xc1, 0x00, 0x00
;;   9c: .byte   0x1f, 0xc1, 0x00, 0x00
||||||| parent of 549c6a5f0 (Update disassembly tests)
;;   8c: .byte   0x1f, 0xc1, 0x00, 0x00
;;   90: .byte   0x1f, 0xc1, 0x00, 0x00
;;   94: .byte   0x1f, 0xc1, 0x00, 0x00
;;   98: .byte   0x1f, 0xc1, 0x00, 0x00
=======
;;   80: .byte   0x1f, 0xc1, 0x00, 0x00
;;   84: .byte   0x1f, 0xc1, 0x00, 0x00
;;   88: .byte   0x1f, 0xc1, 0x00, 0x00
;;   8c: .byte   0x1f, 0xc1, 0x00, 0x00
;;   90: .byte   0x00, 0x00, 0x00, 0xdf
;;   94: .byte   0x00, 0x00, 0x00, 0x00
;;   98: .byte   0x00, 0x00, 0x00, 0x5f
>>>>>>> 549c6a5f0 (Update disassembly tests)
