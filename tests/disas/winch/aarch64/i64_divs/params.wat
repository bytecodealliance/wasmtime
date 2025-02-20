;;! target = "aarch64"
;;! test = "winch"

(module
    (func (param i64) (param i64) (result i64)
	(local.get 0)
	(local.get 1)
	(i64.div_s)
    )
)

;; wasm[0]::function[0]:
;;       stp     x29, x30, [sp, #-0x10]!
;;       mov     x29, sp
;;       mov     x28, sp
;;       mov     x9, x0
;;       sub     x28, x28, #0x20
;;       mov     sp, x28
;;       stur    x0, [x28, #0x18]
;;       stur    x1, [x28, #0x10]
;;       stur    x2, [x28, #8]
;;       stur    x3, [x28]
;;       ldur    x0, [x28]
;;       ldur    x1, [x28, #8]
;;       cbz     x0, #0x5c
;;   34: cmn     x0, #1
;;       ccmp    x1, #1, #0, eq
;;       b.vs    #0x60
;;   40: sdiv    x1, x1, x0
;;       mov     x0, x1
;;       add     x28, x28, #0x20
;;       mov     sp, x28
;;       mov     sp, x28
;;       ldp     x29, x30, [sp], #0x10
;;       ret
;;   5c: .byte   0x1f, 0xc1, 0x00, 0x00
;;   60: .byte   0x1f, 0xc1, 0x00, 0x00
