;;! target = "aarch64"
;;! test = "winch"

(module
    (func (result i64)
	(i64.const 0)
	(i64.const 0)
	(i64.rem_s)
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
;;       mov     x0, x16
;;       mov     x16, #0
;;       mov     x1, x16
;;       cbz     x0, #0x50
;;   34: sdiv    x16, x1, x0
;;       msub    x1, x0, x16, x1
;;       mov     x0, x1
;;       add     sp, sp, #0x10
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   50: .byte   0x1f, 0xc1, 0x00, 0x00
