;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i32)
        (local f32)  

        (local.get 0)
        (i32.trunc_f32_u)
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
;;       b.lo    #0x88
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       mov     x16, #0
;;       stur    x16, [x28]
;;       ldur    s0, [x28, #4]
;;       fcmp    s0, s0
;;       b.vs    #0x8c
;;   54: fmov    s31, #-1.00000000
<<<<<<< HEAD
;;       fcmp    s0, s31
;;       b.le    #0x94
;;   60: mov     x16, #0x4f800000
;;       fmov    s31, w16
;;       fcmp    s0, s31
;;       b.ge    #0x98
;;   70: fcvtzu  w0, s0
||||||| parent of 549c6a5f0 (Update disassembly tests)
;;       fcmp    s31, s0
;;       b.le    #0x94
;;   60: mov     x16, #0x4f800000
;;       fmov    s31, w16
;;       fcmp    s31, s0
;;       b.ge    #0x98
;;   70: fcvtzu  w0, s0
=======
;;       fcmp    s31, s0
;;       b.le    #0x90
;;   60: ldr     s31, #0x98
;;       fcmp    s31, s0
;;       b.ge    #0x94
;;   6c: fcvtzu  w0, s0
>>>>>>> 549c6a5f0 (Update disassembly tests)
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   88: .byte   0x1f, 0xc1, 0x00, 0x00
;;   8c: .byte   0x1f, 0xc1, 0x00, 0x00
;;   90: .byte   0x1f, 0xc1, 0x00, 0x00
;;   94: .byte   0x1f, 0xc1, 0x00, 0x00
;;   98: .byte   0x00, 0x00, 0x80, 0x4f
