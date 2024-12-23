;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i32)
        (f32.const 1.0)
        (i32.trunc_f32_u)
    )
)
;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     sp, sp, #0x10
;;       mov     x28, sp
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       mov     x16, #0x3f800000
;;       fmov    s1, w16
;;       fcmp    s1, s1
;;       b.vs    #0x60
;;   30: fmov    s0, #-1.00000000
;;       fcmp    s1, s0
;;       b.le    #0x64
;;   3c: mov     x16, #0x4f800000
;;       fmov    s0, w16
;;       fcmp    s1, s0
;;       b.ge    #0x68
;;   4c: fcvtzu  w0, s1
;;       add     sp, sp, #0x10
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   60: .byte   0x1f, 0xc1, 0x00, 0x00
;;   64: .byte   0x1f, 0xc1, 0x00, 0x00
;;   68: .byte   0x1f, 0xc1, 0x00, 0x00
