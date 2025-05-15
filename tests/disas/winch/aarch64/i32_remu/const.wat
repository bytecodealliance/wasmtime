;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i32)
	(i32.const 7)
	(i32.const 5)
	(i32.rem_u)
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
;;       b.lo    #0x78
;;   2c: mov     x9, x0
;;       sub     x28, x28, #0x10
;;       mov     sp, x28
;;       stur    x0, [x28, #8]
;;       stur    x1, [x28]
;;       mov     x16, #5
;;       mov     w0, w16
;;       mov     x16, #7
;;       mov     w1, w16
;;       cbz     w0, #0x7c
;;   54: udiv    w16, w1, w0
;;       msub    w1, w0, w16, w1
;;       mov     w0, w1
;;       add     x28, x28, #0x10
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldr     x28, [sp], #0x10
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   78: .byte   0x1f, 0xc1, 0x00, 0x00
;;   7c: .byte   0x1f, 0xc1, 0x00, 0x00
