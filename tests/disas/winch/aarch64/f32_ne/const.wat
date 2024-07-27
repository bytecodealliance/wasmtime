;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i32)
        (f32.const 1)
        (f32.const 2)
        (f32.ne)
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
;;       mov     x16, #0x40000000
;;       fmov    s0, w16
;;       mov     x16, #0x3f800000
;;       fmov    s1, w16
;;       fcmp    s0, s1
;;       cset    x0, ne
;;       add     sp, sp, #0x10
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
