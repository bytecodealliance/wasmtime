;;! target = "aarch64"
;;! test = "winch"

(module
    (func (param i64) (param i64) (result i64)
	(local.get 0)
	(local.get 1)
	(i64.rem_s)
    )
)

;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     sp, sp, #0x20
;;       mov     x28, sp
;;       stur    x0, [x28, #0x18]
;;       stur    x1, [x28, #0x10]
;;       stur    x2, [x28, #8]
;;       stur    x3, [x28]
;;       ldur    x0, [x28]
;;       ldur    x1, [x28, #8]
;;       cbz     x0, #0x50
;;   34: sdiv    x16, x1, x0
;;       msub    x1, x0, x16, x1
;;       mov     x0, x1
;;       add     sp, sp, #0x20
;;       mov     x28, sp
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   50: .byte   0x1f, 0xc1, 0x00, 0x00
