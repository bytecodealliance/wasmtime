;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i32)
	(i32.const 0)
	(i32.const 0)
	(i32.div_u)
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
;;       mov     x16, #0
;;       mov     w0, w16
;;       mov     x16, #0
;;       mov     w1, w16
;;       cbz     w0, #0x4c
;;   34: udiv    w1, w1, w0
;;       mov     w0, w1
;;       add     sp, sp, #0x10
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   4c: .byte   0x1f, 0xc1, 0x00, 0x00
