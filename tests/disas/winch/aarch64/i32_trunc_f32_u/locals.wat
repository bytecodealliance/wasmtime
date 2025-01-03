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
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     sp, sp, #0x18
;;       mov     x28, sp
;;       stur    x0, [x28, #0x10]
;;       stur    x1, [x28, #8]
;;       mov     x16, #0
;;       stur    x16, [x28]
;;       ldur    s1, [x28, #4]
;;       fcmp    s1, s1
;;       b.vs    #0x64
;;   34: fmov    s31, #-1.00000000
;;       fcmp    s31, s1
;;       b.le    #0x68
;;   40: mov     x16, #0x4f800000
;;       fmov    s31, w16
;;       fcmp    s31, s1
;;       b.ge    #0x6c
;;   50: fcvtzu  w0, s1
;;       add     sp, sp, #0x18
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   64: .byte   0x1f, 0xc1, 0x00, 0x00
;;   68: .byte   0x1f, 0xc1, 0x00, 0x00
;;   6c: .byte   0x1f, 0xc1, 0x00, 0x00
