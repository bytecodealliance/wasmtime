;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i32)
	(i32.const 7)
	(i32.const 5)
	(i32.rem_s)
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
;;       mov     x16, #5
;;       mov     w0, w16
;;       mov     x16, #7
;;       mov     w1, w16
;;       cbz     w0, #0x58
;;   34: sxtw    x0, w0
;;       sxtw    x1, w1
;;       sdiv    x16, x1, x0
;;       msub    x1, x0, x16, x1
;;       mov     w0, w1
;;       add     sp, sp, #0x10
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   58: .byte   0x1f, 0xc1, 0x00, 0x00
