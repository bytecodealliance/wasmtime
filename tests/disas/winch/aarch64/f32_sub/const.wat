;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result f32)
        (f32.const 1.1)
        (f32.const 2.2)
        (f32.sub)
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
;;       mov     w16, #0xcccd
;;       movk    w16, #0x400c, lsl #16
;;       fmov    s0, w16
;;       mov     w16, #0xcccd
;;       movk    w16, #0x3f8c, lsl #16
;;       fmov    s1, w16
;;       fsub    s1, s1, s0
;;       fmov    d0, d1
;;       add     sp, sp, #0x10
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
