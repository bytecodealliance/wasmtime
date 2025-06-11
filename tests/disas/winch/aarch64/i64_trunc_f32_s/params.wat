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
;;       b.lo    #0x8c
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x18
;;       mov     sp, x28
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       stur    s0, [x28, #4]
;;       ldur    s0, [x28, #4]
;;       sub     sp, x28, #8
;;       fcmp    s0, s0
;;       b.vs    #0x90
;;   54: ldr     s31, #0xa0
;;       fcmp    s0, s31
;;       b.le    #0x94
;;   60: ldr     s31, #0xa8
;;       fcmp    s0, s31
;;       b.ge    #0x98
;;   6c: fcvtzs  x0, s0
;;       mov     sp, x28
;;       add     x28, x28, #0x18
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   8c: .byte   0x1f, 0xc1, 0x00, 0x00
;;   90: .byte   0x1f, 0xc1, 0x00, 0x00
;;   94: .byte   0x1f, 0xc1, 0x00, 0x00
;;   98: .byte   0x1f, 0xc1, 0x00, 0x00
;;   9c: .byte   0x00, 0x00, 0x00, 0x00
;;   a0: .byte   0x01, 0x00, 0x00, 0xdf
;;   a4: .byte   0x00, 0x00, 0x00, 0x00
;;   a8: .byte   0x00, 0x00, 0x00, 0x5f
