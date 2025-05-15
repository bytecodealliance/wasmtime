;;! target = "aarch64"
;;! test = "winch"

(module
    (func (param f32) (result i64)
        (local.get 0)
        (i64.trunc_f32_u)
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
;;       stur    s0, [x28, #4]
;;       ldur    s0, [x28, #4]
;;       fcmp    s0, s0
;;       b.vs    #0x8c
;;   50: fmov    s31, #-1.00000000
;;       fcmp    s31, s0
;;       b.le    #0x90
;;   5c: mov     x16, #0x5f800000
;;       fmov    s31, w16
;;       fcmp    s31, s0
;;       b.ge    #0x94
;;   6c: fcvtzu  x0, s0
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
