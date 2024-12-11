;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i32)
	(i32.const -1)
	(i32.const -1)
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
;;       orr     x16, xzr, #0xffffffff
;;       mov     w0, w16
;;       orr     x16, xzr, #0xffffffff
;;       mov     w1, w16
;;       cbz     x0, #0x54
;;   34: mov     w0, w0
;;       mov     w1, w1
;;       udiv    x1, x1, x0
;;       mov     w0, w1
;;       add     sp, sp, #0x10
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   54: .byte   0x1f, 0xc1, 0x00, 0x00
